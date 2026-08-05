# 普通任务到点提醒 — 技术设计

## 背景与目标

普通任务当前无法设定提醒。目标：为普通（非每日）任务增加一次性提醒时间戳 `remind_at`，到点系统通知一次，完成后自动清空。每日任务提醒（`reminder_time` HH:MM 每天重复）保持不变。

## 数据模型

### 新列

`tasks.remind_at`（TEXT，`YYYY-MM-DD HH:MM`，+08:00 中国时区）：

- 仅普通任务使用；每日任务恒为 NULL。
- 与 `due_date` 独立（`due_date` 是截止日期，`remind_at` 是提醒时刻，可不同）。

### migrations v17

```rust
(
    17,
    "task_remind_at",
    "", // 由 apply_task_remind_at_migration 处理（add_column_if_missing 幂等加列）
),
```

```rust
fn apply_task_remind_at_migration(conn: &Connection) -> Result<()> {
    add_column_if_missing(conn, "tasks", "remind_at", "remind_at TEXT")?;
    Ok(())
}
```

`current_version` 自动取 MAX，老库执行 v17，新库从头执行到 v17，安全。

## 后端设计

### 模型（db/models.rs）

```rust
pub struct Task {
    // ...现有字段
    /// 普通任务一次性提醒时间（YYYY-MM-DD HH:MM，+08:00）；null = 不提醒。每日任务恒为 null。
    #[serde(default)]
    pub remind_at: Option<String>,
}

pub struct CreateTaskRequest {
    // ...
    #[serde(default)]
    pub remind_at: Option<String>,
}

pub struct UpdateTaskRequest {
    // ...
    #[serde(default)]
    pub remind_at: Option<String>,
}
```

### CRUD（db/tasks.rs）

- `create_task`：INSERT 增加 `remind_at`。
- `get_task` / `get_all_tasks`：SELECT 增加 `remind_at`（列序 14）。
- `update_task`：UPDATE 增加 `remind_at`。

### 提醒调度（notifications）

**方案：保留 `daily_reminder.rs` 文件名，内部扩展为同时处理两类提醒。**（重命名为 `task_reminder.rs` 会波及 lib.rs 引用，收益低；保持文件、扩展语义。）

现有结构 `DailyReminder { id, title, reminder_time }` 扩展为带 `kind`：

```rust
enum ReminderKind {
    /// 每日任务：reminder_time（HH:MM）每天重复。
    Daily,
    /// 普通任务：remind_at（YYYY-MM-DD HH:MM）一次性。
    OneShot,
}

struct Reminder {
    id: i64,
    title: String,
    kind: ReminderKind,
    remind_at: NaiveDateTime, // 计算后的下一次触发时刻
}
```

`load_reminders` 改为同时查两类：

```sql
-- 每日任务（reminder_time 非空、is_daily=1）
SELECT id, title, reminder_time FROM tasks
 WHERE is_daily = 1 AND reminder_time IS NOT NULL AND reminder_time != '' AND deleted_at IS NULL
-- 普通任务（remind_at 非空、is_daily=0）
SELECT id, title, remind_at FROM tasks
 WHERE is_daily = 0 AND remind_at IS NOT NULL AND remind_at != '' AND deleted_at IS NULL
```

`plan_next` 统一按 `next_occurrence_dt` 排序取最小等待。每日任务用现有 `next_occurrence_dt(now, NaiveTime)`；一次性任务直接用 `remind_at`（若已过则视为过期——调度线程启动时若发现已过应清空而不是安排到过去）。

**到点行为：**
- 每日任务：发通知（现有逻辑）。
- 一次性任务：发通知后**清空 `remind_at`**（写库），防止下次轮询重复触发。

**过期清理（启动时）：**
- `lib.rs` 启动调用 `ensure_scheduled` 前，先执行一次"清理过期 remind_at"（`UPDATE tasks SET remind_at = NULL, updated_at = ... WHERE remind_at IS NOT NULL AND remind_at < 当前时刻`）。提醒错过不补发。

### 命令层（commands/tasks.rs）

- `create_task` / `update_task`：传递 `remind_at`。
- 普通任务完成（status → done）：`remind_at` 自动清空。在 `update_task` 命令中：若 `existing.status != "done"` 且新 status == "done"，清空 `remind_at`。
- 每次任务变更后现有 `reschedule` 调用保持（重建调度线程读取新配置）。

### WebDAV 同步（sync/exporter.rs）

`remind_at` 加入 tasks 的 SELECT / INSERT / UPDATE 列。合并语义：作为普通字段参与 LWW 整体覆盖（不做 `merge_daily_fields` 那样的非空优先，因为一次性提醒是"完成即清空"语义，非空优先会复活已清空的提醒）。若远程在平局时带 remind_at，本地未清空，允许整体覆盖。

## 前端设计

### types/task.ts

```ts
/** 普通任务一次性提醒时间（YYYY-MM-DD HH:MM，本地时间）；null 表示不提醒。每日任务恒为 null。 */
remind_at: string | null
```

`CreateTaskRequest` / `UpdateTaskRequest` 同步新增 `remind_at?: string | null`。

### TaskForm.tsx

- 非每日任务（`!isDaily`）渲染 `<input type="datetime-local">`，值映射 `remind_at`。
- HTML `datetime-local` 值格式为 `YYYY-MM-DDTHH:MM`，后端存 `YYYY-MM-DD HH:MM`，前端提交前替换 `T` → 空格。
- 每日任务分支保持现有 `time` 输入（reminder_time）不变。
- 提示文案："到点系统通知提醒你（可选，完成后自动清除）"。

## 验证策略

1. `cargo test`：调度器单测（一次性到点、过期清空、每日回归）、db::tasks tests、exporter tests。
2. `npm run build`。
3. 手动：创建普通任务设提醒 → 观察通知 → 查库确认 remind_at 清空。

## 风险与回滚

- 低风险：新增列 + 独立调度分支，不影响每日任务与同步主流程。
- 回滚：删除 v17 迁移 + 恢复代码；remind_at 列无业务数据价值。
- 提醒依赖应用进程存活（Tauri 通知插件 + 线程调度），进程退出后不触发——已在 PRD Out of Scope 注明。

## 兼容性

- 桌面版升级：v17 加列，老任务无 remind_at，行为不变。
- Android 版升级：同数据库迁移，通知复用现有插件能力。
- 跨设备：remind_at 随 WebDAV 同步，但"已提醒"状态是本地的（提醒清空会同步为删除提醒）。