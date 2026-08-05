# 普通任务到点提醒 — 执行计划

## 前置

- 工作区存在 08-02（widget 移除）与 08-03（AI 同步）的 dirty 改动。**本任务基于当前工作区继续，commit 时按任务分组。**
- 当前 migrations 最高 v16，本任务使用 v17。

## 执行顺序（按依赖关系）

### 第 1 层：数据库

- [ ] **1.1** `db/migrations.rs` — 追加 migration v17 `task_remind_at` + `apply_task_remind_at_migration`（幂等加列 `remind_at TEXT`）
- [ ] **1.2** `db/models.rs` — `Task` / `CreateTaskRequest` / `UpdateTaskRequest` 新增 `remind_at: Option<String>`
- [ ] **1.3** `db/tasks.rs` — INSERT / SELECT / UPDATE 增加 `remind_at` 列（列序 14）

### 第 2 层：提醒调度

- [ ] **2.1** `notifications/daily_reminder.rs` — 扩展为两类提醒：
  - `ReminderKind`（Daily / OneShot）
  - `load_reminders` 同时查每日任务（reminder_time）与普通任务（remind_at）
  - `plan_next` 统一按下次触发时刻排序；一次性任务用立即时刻
  - 到点：每日任务发通知；一次性任务发通知 + 清空 `remind_at`
- [ ] **2.2** 新增函数 `clear_expired_remind_at(conn)` — 清理已过期的 remind_at（启动时调用）
- [ ] **2.3** `lib.rs` — 启动时 `ensure_scheduled` 前调用 `clear_expired_remind_at`

### 第 3 层：命令层

- [ ] **3.1** `commands/tasks.rs` — `CreateTaskCmd` / `UpdateTaskCmd` 增加 `remind_at` 字段透传
- [ ] **3.2** 普通任务完成（status → done）自动清空 `remind_at`（在 `update_task` 命令中判断）

### 第 4 层：同步

- [ ] **4.1** `sync/exporter.rs` — tasks SELECT / INSERT / UPDATE 增加 `remind_at` 列（普通字段 LWW 覆盖，不进 `merge_daily_fields`）

### 第 5 层：前端

- [ ] **5.1** `types/task.ts` — `Task` / `CreateTaskRequest` / `UpdateTaskRequest` 新增 `remind_at`
- [ ] **5.2** `TaskForm.tsx` — 非每日任务显示 `<input type="datetime-local">`；提交时 `T` → 空格；每日任务分支不变

### 第 6 层：测试

- [ ] **6.1** `daily_reminder.rs` 单测：一次性到点触发、过期清空、每日回归
- [ ] **6.2** `db/tasks.rs` tests：remind_at 增删改查
- [ ] **6.3** exporter tests：remind_at 同步（含平局整体覆盖）

### 最终验证

- [ ] **7.1** `cargo test` 全部通过
- [ ] **7.2** `npm run build` 通过
- [ ] **7.3** `rg -i remind_at` 全仓库无残留引用问题

## 提交分组（Phase 3.4）

1. `feat(todo): 普通任务支持到时提醒（remind_at）` — 后端 + 前端 + 迁移 + 测试
2. `chore(trellis): 归档 08-03-todo-reminder-time` — 任务文件（若自动提交未生效）

## 回滚

- 删除 v17 迁移定义 + 还原代码；`remind_at` 列无业务数据价值。
- 提醒调度逻辑失败不影响主流程（仅发通知，失败记 warn）。