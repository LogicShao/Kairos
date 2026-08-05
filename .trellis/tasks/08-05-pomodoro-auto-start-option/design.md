# 番茄钟自动开始下一阶段开关 — 技术设计

## 1. 现状机制

- 阶段推进在 `src-tauri/src/timer.rs`：`tick()` 倒计时归零 → `advance_phase(true)` → 切换阶段且 `is_running = true`（自动开始）。
- `advance_phase(run_next_phase: bool)` 已支持暂停切换（`false`）；`complete_phase_paused()` 即其公开封装（中断"补记完成"在用）。
- 配置：`pomodoro_config` 单例表（id=1），不参与 WebDAV 同步（exporter 仅同步 sessions）。
- 迁移最高 v17（本任务用 **v18**）。

## 2. 方案

在 `pomodoro_config` 新增布尔列 `auto_start_next_phase`（默认 0 = 关闭），引擎阶段归零时据此决定 `advance_phase(true/false)`。前端设置弹窗加开关透传。

### 数据流

```
PomodoroTimer.tsx 开关（默认 false）
  → update_pomodoro_config(config 含 auto_start_next_phase)
  → db update_config 写 pomodoro_config.auto_start_next_phase
  → engine.update_config(PomodoroConfig{auto_start_next_phase})
  → tick() 归零 → advance_phase(self.config.auto_start_next_phase)
```

## 3. 改动点

| 层 | 文件 | 改动 |
|----|------|------|
| DB 迁移 | `db/migrations.rs` | v18 `auto_start_next_phase` + `apply_auto_start_next_phase_migration`（add_column_if_missing 幂等） |
| 模型 | `db/models.rs` | `PomodoroConfig` / `UpdatePomodoroConfigRequest` 加 `auto_start_next_phase: bool` |
| DB 层 | `db/pomodoro.rs` | `get_config` SELECT 读列 + 默认行 INSERT 带列；`update_config` UPDATE 写列（i64 ↔ bool） |
| 引擎 | `timer.rs` | `tick()` 归零分支：`advance_phase(self.config.auto_start_next_phase)`；`update_config` 不变（重置到 work 暂停） |
| 命令层 | `commands/pomodoro.rs` | `PomodoroConfigData` 加字段；`get/update` 透传（含引擎 `PomodoroConfig` 字面量） |
| 前端类型 | `types/pomodoro.ts` | `PomodoroConfig` 加 `auto_start_next_phase: boolean` |
| 前端 UI | `PomodoroTimer.tsx` | 设置弹窗加开关行（参考 NotificationSettings 手写 `role="switch"` 模式），加载/保存透传 |

## 4. 关键细节

- **默认值语义**：`add_column_if_missing` 用 `DEFAULT 0`；`get_config` 默认行 INSERT 也带 `0` → 存量用户升级后默认关闭（符合需求"默认不自动进入"）。
- **bool ↔ i64**：rusqlite 无原生 bool 列，沿用现有 `is_daily` 等列的 `as i64` / `!= 0` 模式。
- **引擎字面量**：`PomodoroEngine::new/restore/update_config` 的 `PomodoroConfig` 结构体字面量（timer.rs 测试、commands/pomodoro.rs、db/pomodoro.rs、schedule.rs、briefing.rs、pomodoro.rs 测试构造处）全部补字段，编译期强制。
- **阶段结束通知**：`pomodoro_scheduler` 只调度"当前阶段结束"提醒，不感知下一阶段是否自动开始，无需改动。
- **work session 创建**：开关关闭时 work 归零 → 切短休暂停；短休归零 → 切 work 暂停（`is_running=false`），`lib.rs` 中 `next_phase==Work && state.is_running` 分支不触发（不创建 session）→ 用户按"开始"时 `start_pomodoro` 创建 session，正确。

## 5. 测试

- `timer.rs`：新增单测——`auto_start_next_phase=false` 时归零切换后 `is_running=false`；`true` 时保持 `is_running=true`（回归现有行为）；break→work 同样遵循。
- 现有测试结构体字面量补字段。
- `cargo test` 全绿 + `npm run build`。

## 6. 回滚

- 删除 v18 迁移 + 还原代码；列无业务数据价值。引擎行为开关关闭时回退默认（用户升级后新默认即为暂停切换，属需求内变更，无数据风险）。
