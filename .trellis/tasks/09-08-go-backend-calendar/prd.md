# 日历/周视图聚合

## Goal

实现 `internal/domain/calendar`：周课表（课程+考试，按学期/周索引聚合）、日历周（课程+考试+有截止待办）、今日概览 `GET /api/briefing/today`。深度移植 Rust `schedule.rs` 聚合语义。

## Requirements

- G1 `internal/domain/calendar`：周课表聚合（课程周规则匹配 `matches_week_pattern`、单双周、假期跳过）、日历日聚合（+有截止待办）、学期周索引
- G2 handler：`GET /api/calendar/week?semester=&week_index=&semester_start_date=`、`GET /api/calendar/day?date=`、`GET /api/briefing/today`
- G3 复用 W4 的 termphase（阶段状态决定课程可见性/考试提醒门控）
- G4 DTO 对齐 `src/types/*`（schedule.ts/briefing.ts）

## Acceptance Criteria

- [ ] `go test ./internal/domain/calendar/...` 绿（对齐 Rust schedule.rs 测试场景：周模式匹配/单双周/假期）
- [ ] `go test ./internal/...` 全绿
- [ ] curl 冒烟：周/日/今日概览，认证通过
- [ ] dev PG 容器仅验证期间临时启动，验证后已关闭

## Notes

- 移植锚点：Rust `src-tauri/src/schedule.rs`（1007 行）及其 `#[cfg(test)]` 场景
- 周规则：`matches_week_pattern` 语义（"1-16"、"2-18双"、"单周"/"双周" 等）
- 聚合单次/并行查询；阶段状态复用 `domain/termphase`
