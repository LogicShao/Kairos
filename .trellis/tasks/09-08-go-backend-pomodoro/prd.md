# 番茄钟引擎与调度

## Goal

实现 `internal/domain/pomodoro`：阶段引擎（work/short_break/long_break、sessions_before_long_break、暂停/重置/中断）、配置单例 + 配置档、会话记录 + 按日完成轮数、运行态持久化恢复、`POST /api/pomodoro/finish-phase` 落库端点。

**架构差异（app→web）**：UI 时钟由**前端本地 setInterval 驱动**（非 Rust 后台 tick 线程）；阶段结束由前端调 `finish-phase` 后端落库 + 校验 + 触发邮件钩子（钩子 W9 实现，先留接口）。页面挂载拉取 `GET /api/pomodoro/state` 恢复运行态。

## Requirements

- G1 `internal/domain/pomodoro`：阶段机（对齐 Rust `timer.rs`：work/short_break/long_break、sessions_before_long_break、暂停/重置/中断、时长校验、自动开始下一阶段 auto_start_next_phase）
- G2 store：config 单例、runtime_state（date_key/active_session）、sessions（开始/结束/按日统计）、profiles（内置 default/intense/relaxed 保护）
- G3 handler：design §5 pomodoro 端点 + `POST /api/pomodoro/finish-phase`（校验时长 → 落库 → 邮件钩子接口）
- G4 DTO 对齐 `src/types/pomodoro.ts`

## Acceptance Criteria

- [ ] `go test ./internal/domain/pomodoro/...` 绿（引擎状态机全场景，对齐 Rust timer.rs 测试）
- [ ] `go test ./internal/...` 全绿
- [ ] curl 冒烟：开始→结束落库、完成轮数统计、运行态恢复、配置档 CRUD（内置保护）
- [ ] dev PG 容器仅验证期间临时启动，验证后已关闭

## Notes

- 移植锚点：Rust `src-tauri/src/timer.rs`（673 行）+ `db/pomodoro.rs` + `db/pomodoro_profiles.rs` + 命令层语义
- **不移植** Rust 的常驻 tick 线程/每秒事件；前端本地时钟 + 拉取校准
- finish-phase 校验阶段时长（防伪造），邮件钩子留接口（W9）
