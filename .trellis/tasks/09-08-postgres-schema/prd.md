# PostgreSQL Schema（Go 后端数据层）

## Goal

建立 `server/internal/store/`：自研 embed 迁移器 + PostgreSQL 迁移 SQL（13 张表，design §4.2 目标态）+ sqlc 生成的 store 层。本地用 dev PG 容器（`kairos-pg`）临时验证，调试后关闭。

## Requirements

- G1 `server/internal/store/migrations/`：版本化 SQL（`//go:embed`），迁移表 `schema_migrations(version, name, applied_at)`，幂等 + 顺序应用
- G2 按 design §4.2 建 13 张表（剔除 lzu_session）+ 索引/CHECK/部分唯一索引 + 种子（pomodoro_profiles 内置 default/intense/relaxed；pomodoro_config/sync_config/notification_config/ai_config 单例行）
- G3 时间归一化：TIMESTAMPTZ（业务时刻）/DATE（纯日期）/TIME（HH:mm）；空值统一 NULL；`remind_at` 归一化 UTC
- G4 `server/db/queries/` sqlc 查询（tasks/courses/exams/term_phases/pomodoro/semester/sync/ai/notify），`sqlc.yaml` + `sqlc generate` 生成 store
- G5 store 单测：对齐 Rust `db/*.rs` 测试行为（tasks 筛选/软删、sync_id 回填、幂等）
- G6 dev PG 容器仅调试时临时启动，用后关闭（`docker stop kairos-pg`）

## Acceptance Criteria

- [ ] 迁移器在干净库跑通且幂等（二次执行无副作用）
- [ ] `psql` 检查表结构符合 §4.2 映射（13 表 + 索引 + 种子）
- [ ] `sqlc generate` 成功生成 `internal/store`（编译通过）
- [ ] `go test ./internal/store/...` 绿（含 Rust 对齐行为测试）
- [ ] 测试/调试结束后 dev PG 容器已关闭

## Notes

- schema 是 design §4.2 目标态（非 Rust SQLite 演进史逐条翻译——那是 18 个 add_column/pragma 补丁，已折叠为最终表定义）
- Rust `#[cfg(test)]` 的 db 层测试是行为规格来源
- 工具：`go run github.com/sqlc-dev/sqlc/cmd/sqlc@latest`；迁移器自研 embed（不装二进制）
