# 核心业务 CRUD（任务/课程/考试/学期阶段）

## Goal

按 parent design §5（tasks/courses/exams/semester REST 契约）实现 Go 后端四个核心域：domain 纯逻辑（importer 文本导入、termphase 学期阶段判定）+ store 补齐 + HTTP handler/DTO。**深度移植 Rust 行为**，非逐行翻译。

## Requirements

- G1 `internal/domain/importer`：课程/考试文本导入解析 + 去重（对齐 Rust `importers.rs` 测试）
- G2 `internal/domain/termphase`：学期阶段判定（对齐 Rust `term_phase.rs`，含 `has_courses` 降级、当前周推导、默认阶段）
- G3 `internal/store` 补齐：tasks（筛选/排序白名单/每日任务/remind_at 归一化）、courses（周规则字段/批量重置学期日期/导入去重）、exams、term_phases（默认阶段种子/当前状态）
- G4 `internal/httpapi` handler + DTO：对齐 design §5 契约与 `src/types/*`（snake_case）
- G5 软删除、sync_id 回填、`'' vs NULL` 语义一致

## Acceptance Criteria

- [ ] `go test ./internal/...` 绿（importer/termphase 单测 + store + httpapi，对齐 Rust 测试场景）
- [ ] curl 冒烟：认证 → tasks/courses/exams/term-phases CRUD、筛选、软删、导入去重
- [ ] 前端 `src/types/*` 字段名与 DTO 一致（snake_case）
- [ ] dev PG 容器仅验证期间临时启动，验证后已关闭

## Notes

- 移植锚点：Rust `src-tauri/src/{importers.rs, term_phase.rs, db/tasks.rs, db/courses.rs, db/exams.rs, db/term_phases.rs, db/semester.rs}` + 其 `#[cfg(test)]` 场景作为行为规格
- 时间归一化：TIMESTAMPTZ/DATE/TIME；remind_at 归一化 UTC
- 前端契约：`src/types/*.ts`
