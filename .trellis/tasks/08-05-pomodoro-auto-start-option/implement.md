# 番茄钟自动开始下一阶段开关 — 执行计划

## 前置

- 任务基于当前 `main`（HEAD `e222632`）继续，工作区干净。
- 当前 migrations 最高 v17，本任务使用 v18。

## 执行顺序（按依赖关系）

### 第 1 层：数据库

- [ ] **1.1** `db/migrations.rs` — 追加 migration v18 `auto_start_next_phase` + `apply_auto_start_next_phase_migration`（幂等加列 `auto_start_next_phase INTEGER NOT NULL DEFAULT 0`）
- [ ] **1.2** `db/models.rs` — `PomodoroConfig` / `UpdatePomodoroConfigRequest` 新增 `auto_start_next_phase: bool`
- [ ] **1.3** `db/pomodoro.rs` — `get_config` SELECT 增加列 + 默认行 INSERT 带列；`update_config` UPDATE 增加列（i64 ↔ bool）

### 第 2 层：引擎

- [ ] **2.1** `timer.rs` — `tick()` 归零分支改为 `advance_phase(self.config.auto_start_next_phase)`
- [ ] **2.2** `timer.rs` 测试 — 补结构体字面量字段 + 新增开关行为单测（false 暂停 / true 自动运行 / break→work 遵循开关）

### 第 3 层：命令层

- [ ] **3.1** `commands/pomodoro.rs` — `PomodoroConfigData` 加 `auto_start_next_phase`；`get_pomodoro_config` / `update_pomodoro_config` 透传（含引擎 `PomodoroConfig` 字面量）

### 第 4 层：前端

- [ ] **4.1** `types/pomodoro.ts` — `PomodoroConfig` 加 `auto_start_next_phase: boolean`
- [ ] **4.2** `PomodoroTimer.tsx` — 设置弹窗加"自动开始下一阶段"开关行（参考 NotificationSettings 的 role="switch" 模式）；加载/保存透传

### 第 5 层：验证

- [ ] **5.1** `cargo test` 全部通过（新增引擎单测覆盖）
- [ ] **5.2** `npm run build` 通过
- [ ] **5.3** 桌面端手动抽查：关闭开关 → 专注结束进入短休暂停；开启 → 自动开始；重启持久化

### 第 6 层：收尾

- [ ] **6.1** 按需更新 `.trellis/spec/`（若发现新约定）
- [ ] **6.2** 提交：`feat(pomodoro): 阶段切换默认需用户确认，新增自动开始下一阶段开关`
- [ ] **6.3** `task.py archive` 归档

## 提交分组

1. `feat(pomodoro): 阶段切换默认需用户确认，新增自动开始下一阶段开关` — 迁移 + 引擎 + 命令 + 前端

## 回滚

- 删除 v18 迁移定义 + 还原代码；`auto_start_next_phase` 列无业务数据价值。
- 引擎切换暂停失败不影响主流程（最坏回到自动开始旧行为）。
