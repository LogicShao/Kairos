# 番茄钟重启后恢复本日状态

## Goal

让 Kairos 的番茄钟在应用重启后恢复“本日可用状态”，使用户不会因为关闭应用、崩溃或系统重启而丢失当天已完成的专注统计，并能明确处理上次未结束的番茄钟。

## What I Already Know

- 当前 `PomodoroEngine` 完全以内存为主，应用启动时总是通过 `PomodoroEngine::new(config)` 从工作阶段、未运行、0 次完成开始。
- `pomodoro_sessions` 表已存在，字段包含 `started_at`、`ended_at`、`session_type`、`task_id`、`sync_id`、`deleted_at`，可以作为今日统计的事实来源。
- `timer::PomodoroState` 当前只返回 `phase`、`remaining_seconds`、`total_seconds`、`is_running`、`completed_sessions`。
- `Today Briefing` 的番茄钟数据目前直接读取内存引擎，因此重启后也会丢失今日完成次数。
- 当前命令层 `start_pomodoro`、`pause_pomodoro`、`reset_pomodoro` 没有真正创建或结束 `pomodoro_sessions` 记录。
- 当前通知调度是进程内 `thread::sleep` + 取消 token；应用退出后通知和计时不会继续，这是本任务不解决的既有限制。

## Requirements

- 重启应用后，番茄钟页面和 Today Briefing 必须恢复当天已完成的专注轮数。
- 今日完成次数必须基于数据库中本地今日范围内已正常结束的 `work` sessions 计算，而不是依赖内存计数。
- 开始一个 `work` 阶段时必须创建或复用一条活跃 `pomodoro_sessions` 记录。
- `work` 阶段正常结束时必须补全该 session 的 `ended_at`，并计入今日完成次数。
- 暂停后重启应用时，应恢复当前阶段、剩余时间、今日完成次数和暂停状态。
- 如果应用退出前计时器处于运行中，重启后不得静默把离线时间全部计入专注；必须暴露“上次运行中断待处理”的状态给前端。
- 前端必须展示中断待处理状态，并提供明确操作：
  - 继续：从上次保存的剩余时间继续，状态变为暂停或运行由实现决定，但必须不自动补记离线时间。
  - 丢弃：关闭活跃 session 或标记为中断，不计入完成次数。
  - 补记完成：仅当用户确认时，将该 work session 结束并计入今日完成。
- 休息阶段可以恢复剩余时间和阶段，但休息阶段不计入今日完成次数。
- 修改番茄钟配置时，应保持现有行为：重置到工作阶段并暂停；如有活跃 session，必须先安全关闭或中断，避免遗留脏数据。
- 现有通知行为不得回退：开始、暂停、重置、阶段切换仍应正确重排或取消进程内通知。

## Acceptance Criteria

- [ ] 关闭并重启应用后，`get_pomodoro_state` 返回的 `completed_sessions` 等于当天已结束 `work` sessions 数。
- [ ] 完成一个 work 阶段后，`pomodoro_sessions` 新增或更新一条 `session_type = 'work'` 且 `ended_at IS NOT NULL` 的记录。
- [ ] 暂停后重启，番茄钟页面显示上次阶段和剩余时间，`is_running = false`，今日完成次数不丢失。
- [ ] 运行中关闭后重启，番茄钟页面不自动增加完成次数，并显示中断处理入口。
- [ ] 用户选择“丢弃”后，中断状态消失，今日完成次数不增加。
- [ ] 用户选择“补记完成”后，中断状态消失，该 work session 计入今日完成次数。
- [ ] Today Briefing 的番茄钟完成次数与番茄钟页面一致。
- [ ] 已有 `cargo test timer` 和 `cargo test db::pomodoro::tests` 通过，并新增覆盖恢复逻辑的 Rust 单元测试。
- [ ] `npm run lint` 和 `npx tsc --noEmit` 通过，或记录既有失败项。

## Definition of Done

- Rust 后端状态恢复逻辑有单元测试覆盖。
- 数据库查询使用参数化 SQL，不拼接用户输入。
- 命令层不新增 `unwrap()` / `expect()`。
- 前端类型与 Tauri IPC 返回结构保持同步。
- Today Briefing 和 Pomodoro 页面使用同一后端恢复结果。
- 不实现应用退出后的后台精准计时，也不承诺退出后发送阶段结束通知。

## Technical Approach

采用“数据库 session 为事实来源 + 内存引擎为运行态缓存”的方案：

- `pomodoro_sessions` 保存完成与活跃 session。
- 新增轻量运行态持久化表或配置记录，保存当前 phase、remaining_seconds、is_running、last_seen_at、active_session_id、date_key。
- 应用启动时先加载配置，再从持久化运行态和今日 session 统计构造 `PomodoroEngine`。
- 如果上次 `is_running = true`，启动时将恢复为“中断待处理”而不是继续扣减离线时间。
- 前端通过扩展后的 `PomodoroState` 渲染中断提示和处理按钮。

## Decision (ADR-lite)

**Context**: 用户希望重启后恢复本日状态，但番茄钟涉及“应用关闭期间是否继续计时”的产品语义。静默补记离线时间会污染专注数据。

**Decision**: MVP 只恢复本日统计和可处理的中断状态，不做后台精准计时。运行中退出后，重启时要求用户显式选择继续、丢弃或补记完成。

**Consequences**:

- 数据准确性优先于自动化。
- 实现不依赖 OS 后台任务，跨桌面和移动端更一致。
- 后续可扩展为“可信离线补计”策略，但本任务先不引入。

## Out of Scope

- 应用退出后继续后台倒计时。
- 应用退出后仍自动发送番茄钟结束通知。
- 跨天自动补完多个阶段。
- 历史统计页面、周/月报表、图表。
- 将番茄钟 session 关联到具体任务的 UI。
- WebDAV 同步冲突策略重构。

## Open Questions

- “继续”按钮是否直接恢复为运行中，还是恢复为暂停状态等待用户手动开始。推荐默认：恢复为暂停状态，避免用户刚打开应用时计时器立即运行。

## Technical Notes

- 主要后端文件：
  - `src-tauri/src/timer.rs`
  - `src-tauri/src/commands/pomodoro.rs`
  - `src-tauri/src/db/pomodoro.rs`
  - `src-tauri/src/db/models.rs`
  - `src-tauri/src/db/migrations.rs`
  - `src-tauri/src/lib.rs`
  - `src-tauri/src/commands/briefing.rs`
- 主要前端文件：
  - `src/types/pomodoro.ts`
  - `src/components/pomodoro/PomodoroTimer.tsx`
  - `src/types/briefing.ts`
  - `src/pages/today/TodayPage.tsx`
- 相关规范：
  - `.trellis/spec/backend/quality-guidelines.md`
  - `.trellis/spec/backend/comment-guidelines.md`
  - `.trellis/spec/backend/directory-structure.md`
  - `.trellis/spec/frontend/type-safety.md`
  - `.trellis/spec/guides/cross-layer-thinking-guide.md`
