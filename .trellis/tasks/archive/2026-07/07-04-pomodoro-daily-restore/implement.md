# 番茄钟重启后恢复本日状态实施计划

## Scope Guard

只实现恢复本日状态和中断处理。不实现后台计时、不实现退出后通知、不实现统计报表。

## Phase A：数据库能力

1. 在 `src-tauri/src/db/migrations.rs` 增加新 migration：
   - 创建 `pomodoro_runtime_state`
   - 插入默认 id=1 记录（可在读取时懒创建）
   - 增加 migration 幂等测试
2. 在 `src-tauri/src/db/models.rs` 增加：
   - `PomodoroRuntimeState`
   - `UpdatePomodoroRuntimeStateRequest` 或等价内部结构
   - 字段注释按 comment guideline 补齐
3. 在 `src-tauri/src/db/pomodoro.rs` 增加函数：
   - `get_or_create_runtime_state`
   - `update_runtime_state`
   - `clear_runtime_interruption`
   - `count_completed_work_sessions_for_date`
   - `find_latest_open_work_session`
   - `soft_delete_session` 或 `discard_session`
4. 补 DB 单元测试：
   - 默认 runtime state 创建
   - 今日完成数统计只统计 `work + ended_at + not deleted`
   - 跨天 session 不计入今天
   - 未结束 session 不计入完成
   - discard 后不计入完成

## Phase B：状态机恢复

1. 在 `src-tauri/src/timer.rs` 扩展 `PomodoroState`：
   - `interrupted`
   - `interrupted_session_id`
   - `last_seen_at`
2. 为 `PomodoroEngine` 增加恢复构造函数：
   - 从 phase 字符串安全解析
   - clamp remaining/total 到 `u32`
   - 恢复 completed_sessions 和 active_session_id
3. 增加 engine 方法：
   - `set_interrupted_metadata`
   - 或将 interrupted 信息作为 engine 字段纳入 `get_state`
4. 补 timer 单元测试：
   - 从持久化状态恢复 paused work
   - 从 running 状态恢复为 interrupted + not running
   - 今日 completed_sessions 不被 update_config 意外保留（配置更新仍归零或按需求重算）

## Phase C：命令层与启动流程

1. 修改 `src-tauri/src/lib.rs` 启动流程：
   - DB 打开后计算今日 date_key
   - 加载 runtime state
   - 处理运行中断恢复
   - 用恢复状态构造 `PomodoroEngine`
2. 修改 `start_pomodoro`：
   - work 阶段无 active session 时创建 session
   - 保存 runtime state 为 running
   - 保持通知调度
3. 修改 `pause_pomodoro`：
   - 保存 runtime state 为 paused
   - 取消通知
4. 修改 `reset_pomodoro`：
   - discard 未完成 active session
   - 保存 reset 后 runtime state
   - 取消通知
5. 在 tick loop phase change 时：
   - work 结束时更新 session `ended_at`
   - 更新今日 completed count
   - 保存下一阶段 runtime state
6. 新增 command：
   - `resolve_pomodoro_interruption`
   - 注册到 `invoke_handler!`
7. 命令层错误全部转 `String`，不新增 `unwrap()`。

## Phase D：Today Briefing

1. 修改 `get_today_briefing`：
   - 使用 `db::pomodoro::count_completed_work_sessions_for_date` 或恢复后的 engine state。
   - 推荐直接用 DB 函数覆盖 completed count，保证刷新一致。
2. 补/改 `commands::briefing` 测试：
   - 重启恢复后的 completed_sessions 来自 DB。

## Phase E：前端

1. 更新 `src/types/pomodoro.ts`：
   - 增加 interrupted 字段
   - 增加 resolve action 类型
2. 更新 `PomodoroTimer.tsx`：
   - 渲染中断提示
   - 三个按钮调用 `resolve_pomodoro_interruption`
   - 操作后刷新 state
   - loading/error 状态保持现有风格
3. 更新 `src/types/briefing.ts` 如后端响应字段变化。
4. Today 页面可只展示现有摘要，不新增处理入口。

## Phase F：验证

必须运行：

```bash
cd src-tauri
cargo test db::pomodoro::tests
cargo test timer
cargo test commands::briefing::tests
cargo check
```

前端验证：

```bash
npm run lint
npx tsc --noEmit
```

去重检查（项目规则，失败时区分新增/既有）：

```bash
npx jscpd . --threshold 10 --reporters console --format rust
```

手工验证：

- 启动番茄钟，等待一个 work 完成，重启应用，完成次数仍存在。
- 启动番茄钟后暂停，重启应用，剩余时间和暂停状态恢复。
- 启动番茄钟运行中直接关闭，重启后出现中断提示，完成次数不自动增加。
- 选择丢弃，中断提示消失，完成次数不增加。
- 再次制造中断，选择补记完成，完成次数增加。
- Today 页面显示的番茄完成次数与番茄钟页一致。

## Rollback Points

- 如果 runtime state migration 出问题，只回滚 migration 和 `db::pomodoro` 新函数，不触碰现有 `pomodoro_sessions` 数据。
- 如果前端中断 UI 复杂度过高，先保留后端字段和 command，前端只显示一条提示 + “继续/丢弃”两个按钮；“补记完成”可延后，但 PRD 中应明确调整。

## Handoff Notes for DeepSeek

- 开始实现前先读取 `implement.jsonl` 中列出的规范和源码。
- 不要 `task.py start`，除非当前会话负责人明确要求进入实现阶段。
- 不要修改通知模块最近的 Windows sender 修复，除非编译必需。
- 当前工作区有其他未提交改动，避免格式化或重写无关文件。
