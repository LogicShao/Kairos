# Go 后端骨架与单账号认证

## Goal

建立 `server/` 的 API 装配骨架：环境变量配置加载、HTTP 中间件（logger/recover/request-id/auth JWT）、`GET /healthz`、单账号认证（`POST /api/auth/login` → JWT、`GET /api/auth/me`、`POST /api/auth/logout`）。

## Requirements

- G1 `internal/config`：从环境变量加载 DB URL / JWT secret / 账号 / API 端口，含 `.env` 支持与校验
- G2 `internal/httpapi`：chi 路由 + 中间件链（slog logger / recover / request-id / JWT auth）；`GET /healthz` 放行
- G3 认证：单账号（环境变量用户名 + bcrypt 密码哈希）`POST /api/auth/login` 校验后签发 HS256 JWT；`GET /api/auth/me` 返回当前用户；`POST /api/auth/logout`（无状态，前端清 token）
- G4 JWT secret >=32 字节，有效期（如 7 天）；未认证访问受保护端点返回 401
- G5 密码哈希生成工具（`make gen-password`，bcrypt），写入 `.env.example`（不提交真实密码）
- G6 迁移启动：main 启动时运行 embed 迁移器建表

## Acceptance Criteria

- [ ] `GET /healthz` 200（未认证可访问）
- [ ] `POST /api/auth/login` 正确凭据 → 返回 JWT + 用户信息；错误凭据 401
- [ ] 带 token 访问 `GET /api/auth/me` 200 返回当前用户；无 token/伪造 token → 401
- [ ] `go test ./internal/...` 绿（含 auth handler 单测，httptest）
- [ ] `make gen-password` 可生成 bcrypt 哈希
- [ ] dev PG 容器仅 W3 验证期间临时启动，验证后已关闭

## Notes

- 单用户部署（parent R1）：无注册、无多租户
- JWT HS256；nginx 终止 TLS（部署层，W11）
- 前端登录页 localStorage 存 token（W10）
- 本模块为**新增**（旧 Rust 用 LZU 登录，已剥离），无 Rust 行为规格可移植，按 parent design §6 设计实现
