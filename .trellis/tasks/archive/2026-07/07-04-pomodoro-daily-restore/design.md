# 番茄钟重启后恢复本日状态设计

## Boundary

本任务只处理“应用进程重启后恢复本日状态”。运行时每秒 tick、通知调度、阶段切换仍由现有 `PomodoroEngine` 和 `lib.rs` 后台线程负责。

不把应用关闭期间的时间默认算入番茄钟。重启时发现上次处于运行中时，进入“中断待处理”状态，由用户显式选择。

## Data Model

### Existing Table: `pomodoro_sessions`

继续作为完成统计的事实来源：

- `session_type = 'work'`
- `ended_at IS NOT NULL`
- `deleted_at IS NULL`
- `started_at` 或 `ended_at` 落在本地今天范围内

建议使用本地日期边界（当前项目 Today Briefing 使用 UTC+8）计算今日窗口：

- start: `YYYY-MM-DDT00:00:00+08:00` 对应 UTC
- end: 次日 00:00:00

### New Table: `pomodoro_runtime_state`

建议新增 migration，而不是塞入 config 表，避免配置与运行态职责混杂。

字段建议：

- `id INTEGER PRIMARY KEY DEFAULT 1`
- `phase TEXT NOT NULL CHECK(phase IN ('work', 'short_break', 'long_break'))`
- `remaining_seconds INTEGER NOT NULL`
- `total_seconds INTEGER NOT NULL`
- `is_running INTEGER NOT NULL DEFAULT 0`
- `active_session_id INTEGER`
- `date_key TEXT NOT NULL`
- `last_seen_at TEXT NOT NULL`
- `interrupted INTEGER NOT NULL DEFAULT 0`
- `created_at TEXT NOT NULL`
- `updated_at TEXT NOT NULL`

说明：

- `date_key` 用本地日期 `YYYY-MM-DD`，用于跨天清理和防止昨日运行态污染今天。
- `last_seen_at` 每次状态保存时更新，供 UI 展示“上次运行于...”或后续策略使用。
- `interrupted = 1` 表示上次关闭时处于运行中，重启后需要用户处理。
- `active_session_id` 指向当前 work session。休息阶段可以为空。

## Backend Contracts

### `PomodoroState`

扩展现有结构：

- `phase: String`
- `remaining_seconds: u32`
- `total_seconds: u32`
- `is_running: bool`
- `completed_sessions: u32`
- `interrupted: bool`
- `interrupted_session_id: Option<i64>`
- `last_seen_at: Option<String>`

前端已有 `PomodoroState` 类型，必须同步更新。

### New Command: `resolve_pomodoro_interruption`

建议入参：

```rust
pub struct ResolvePomodoroInterruptionRequest {
    pub action: String, // "continue" | "discard" | "complete"
}
```

行为：

- `continue`: 清除 interrupted，恢复同一 phase 和 remaining_seconds，保持暂停状态。
- `discard`: 若 active session 存在且未 ended，更新 `ended_at = now` 并可额外标记为 interrupted。由于现有表无 status 字段，MVP 可将其软删除或结束但不计入完成；推荐软删除，避免未完成 work 被统计误用。
- `complete`: 若 active work session 存在，补 `ended_at = now`，清除 interrupted，切换到下一阶段且暂停。

如果不想软删除 discard 的 session，可新增 `status` 字段。但这会牵涉同步 exporter 和历史数据兼容，MVP 推荐不新增。

## Engine Changes

`PomodoroEngine` 需要新增一个从持久化状态构造的入口，避免启动时只能 `new(config)`：

- `PomodoroEngine::from_state(config, restored_state, completed_sessions)`
- 或 `PomodoroEngine::restore(config, phase, remaining, total, is_running, completed, active_session_id)`

启动流程：

1. 打开 DB，运行 migration。
2. 加载 `pomodoro_config`。
3. 计算本地今日完成的 work sessions 数。
4. 加载 `pomodoro_runtime_state`。
5. 如果 state 缺失或 `date_key != today`，创建新的暂停 work 状态，completed_sessions 来自今日统计。
6. 如果 state 存在且 `is_running = true`，恢复为 `is_running = false`、`interrupted = true`。
7. 构造 `PomodoroEngine` 并 `manage`。

运行时保存：

- `start_pomodoro`: work 阶段如果没有 active session，创建 session；保存 runtime state 为 running。
- `pause_pomodoro`: 保存 runtime state 为 paused。
- `reset_pomodoro`: 如果存在未结束 active session，丢弃或软删除；保存当前 phase 的 full duration paused。
- tick loop 每秒或每 N 秒保存 state。推荐每秒保存，KISS，SQLite 本地写入量可接受；如担心写入，可在 remaining_seconds 变化时每 5 秒保存一次，但要确保退出损失不超过 5 秒。
- phase change: 若 ended phase 是 work，更新 active session `ended_at`，重新计算 completed_sessions 或递增并保存；进入下一阶段 paused/running 行为保持现状（当前代码自动 running）。

## Today Briefing

`get_today_briefing` 应继续读取 engine state，但 engine 启动时必须已恢复今日完成数。若希望 Briefing 与 DB 更强一致，也可在 command 中直接调用 DB 今日统计函数覆盖 `completed_sessions`。

推荐：新增 `db::pomodoro::count_completed_work_sessions_for_date(conn, date)`，启动恢复和 Today Briefing 都使用该函数，避免两套定义漂移。

## Frontend UX

`PomodoroTimer.tsx` 启动后通过 `get_pomodoro_state` 获取 `interrupted` 字段。

当 `interrupted = true` 时：

- 在计时圆环下方显示一条紧凑提示，例如“上次专注被中断”。
- 提供三个按钮：继续、丢弃、补记完成。
- 点击后调用 `resolve_pomodoro_interruption`，再刷新 `get_pomodoro_state`。

默认状态：

- 不弹模态阻断用户；使用页面内提示即可。
- Today 页面只展示状态摘要，不提供中断处理按钮；点击进入番茄钟页处理。

## Compatibility

- 旧数据库无 `pomodoro_runtime_state`：migration 创建表，启动时插入默认状态。
- 旧 `pomodoro_sessions` 无 `sync_id` 的兼容由既有 migration v4 处理。
- 不修改 `pomodoro_sessions` 字段，避免触碰 sync exporter。

## Risks

- 每秒保存 runtime state 可能增加写入量，但本地 SQLite 负载很低，优先选择简单可靠。
- discard 用软删除会让同步层传播删除；这是合理的，因为未完成 session 不应作为历史记录同步。
- 如果应用崩溃发生在 session 创建后、runtime state 保存前，可能出现未结束孤儿 session。启动恢复应查询最近未结束 work session 并作为中断候选兜底。
