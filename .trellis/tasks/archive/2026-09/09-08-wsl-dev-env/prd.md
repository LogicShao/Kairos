# WSL 开发环境与 Go 后端骨架（本地轻量验证）

## Goal

在 WSL（Ubuntu）建立 Go 后端开发骨架：确认工具链（Go/Docker/Node 已就绪），创建 `server/` Go 项目骨架，
根 `Makefile` 增加 `go-*` 目标，使 `make go-dev` 可并行启动 Go 占位服务（healthz）与 Vite dev server。

本地**仅承载轻量验证**（`go test` / tsc / eslint / smoke / lsp）；不安装系统级 PostgreSQL，不承载部署
（全栈部署目标为远端服务器，见 parent 决策 10）。所有环境配置变更（装软件、起容器、写 .env）须经用户确认。

## Requirements

- G1 `server/` Go 骨架：module + `cmd/api/main.go` 占位 + `GET /healthz` + `GET /healthz/live`
- G2 根 `Makefile` 增加 `go-build`/`go-test`/`go-lint`/`go-dev` 目标；保留前端目标；`verify` 阶段先不并入 go（后端未就绪）
- G3 `make go-dev` 同时启动 Go 占位（:8080）与 Vite dev（:5173，dev 代理 `/api`→Go）
- G4 dev 数据库：按需 `docker run postgres:16` 容器（本地不装系统 PG）；启动动作延至 W2 集成测试前，经用户确认后执行
- G5 本地无 Rust toolchain 且**不安装**（Rust 侧验证靠静态自检/远端/CI，W11 前保留）；工具链 Go/Docker 已确认

## Acceptance Criteria

- [ ] `go version` ≥1.24 与 `docker ps` 可用（已确认，记录在案）
- [ ] `make go-test` 绿（占位测试）；`curl localhost:8080/healthz` 200
- [ ] `make go-dev` 并行拉起 Go + Vite 且互不阻塞
- [ ] 前端 `npx tsc --noEmit` 与 `npm run lint` 仍绿（未回归）
- [ ] 无系统级 PostgreSQL 安装、无 cargo/rustup 安装

## Notes

- 轻量任务 PRD-only；验证证据记录于本 child 目录或会话记录。
- 回滚：每步可运行状态；依赖 parent `pre-go-migration` tag 与逐波 commit。
