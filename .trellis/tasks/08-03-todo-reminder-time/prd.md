# 普通任务到点提醒（一次性任务提醒时间）

## Goal

让 Kairos 的普通（非每日）任务可以设定一个独立提醒时间戳，到点通过系统通知提醒用户完成。提醒只触发一次，任务完成后自动清理提醒。

## Background

- 现有 `reminder_time`（HH:MM）仅用于每日任务（`is_daily=1`），每日任务每天到点提醒。普通任务无法设定提醒。
- 用户期望：普通任务（如"交报告"）能设定 `2026-08-05 14:00` 这样的时间，到点通知一次；任务完成后提醒自动消失。
- Android 与桌面端通知系统已就绪（`notifications::system::show_system_notification`），可复用。

## Requirements

### 数据库

- 新增 `tasks.remind_at` 列（TEXT，格式 `YYYY-MM-DD HH:MM`，中国时区 +08:00），普通任务的一次性提醒时间。
- 每日任务不设 `remind_at`（仍用 `reminder_time`，每天重复）。
- migrations v17：幂等加列（`add_column_if_missing`）。

### 后端

- `Task` / `CreateTaskRequest` / `UpdateTaskRequest` 模型新增 `remind_at: Option<String>` 字段。
- `db::tasks` CRUD 支持 `remind_at`（INSERT、SELECT、UPDATE）。
- 扩展 `notifications::daily_reminder`（或改为 `task_reminder.rs`）：
  - 每日任务：保持现有 `reminder_time` 逻辑不变。
  - 普通任务：`remind_at` 到点发一次通知，发完后清空 `remind_at`（避免重复）。
- 启动时：清理已过期的 `remind_at`（提醒时刻错过不补发）。
- `complete_daily_task` / `uncomplete_daily_task` 命令不受影响。
- 普通任务完成（status → done）：自动清空 `remind_at`。
- WebDAV 同步：`remind_at` 加入 exporter（作为普通字段参与 LWW 整体覆盖，不进 `merge_daily_fields`）。

### 前端

- `TaskForm.tsx`：对非每日任务显示「提醒时间」输入框（`datetime-local`），值存入 `remind_at`。
- `types/task.ts`：`Task` / `CreateTaskRequest` / `UpdateTaskRequest` 新增 `remind_at?: string | null`。
- 每日任务仍只显示「每日提醒时间」（`time` 输入），不变。

## Acceptance Criteria

- [ ] `cargo test` 全部通过（含调度器新单元测试、db::tasks tests、exporter tests）
- [ ] `npm run build` 通过
- [ ] 创建普通任务时设定提醒时间 → 到点收到系统通知
- [ ] 通知发完后 `remind_at` 自动清空（数据库确认），不重复提醒
- [ ] 提醒时间已过期（应用未运行）→ 启动时自动清空，不补发
- [ ] 完成普通任务（status → done）→ `remind_at` 自动清空
- [ ] 每日任务提醒完全不受影响（回归确认）
- [ ] `rg -i remind_at src/ src-tauri/src/` 无残留引用问题类型错误

## Constraints

- 不改变每日任务的提醒语义（`reminder_time` HH:MM 每天重复）。
- 不与现有迁移冲突（当前最高 v16，使用 v17）。
- 工作区存在 08-02 / 08-03 任务的 dirty 改动，实现时需先处理提交顺序。

## Out of Scope

- 暂不支持提醒后未完成的情况下自动重复提醒（后续可扩）。
- 暂不支持 Android 原生 AlarmManager 级别的后台提醒（应用不在前台时仍工作，当前依赖 Tauri 通知插件 + 线程调度，应用进程存活时正常）。
- 不涉及 AI 晨报、番茄钟、考试提醒。