# W11 部署拓扑（docker compose + nginx + HTTPS）与 Rust 移除

> Parent: `09-08-go-backend-migration`。对应波次 **W11**（最后一波）。
> 依赖：W10 前端 API 层已完成并提交。本任务完成后 parent AC1–AC8 收口。

## Goal

把 Kairos 以容器方式部署为纯 Web B/S 应用，并彻底移除 Rust/Tauri 遗留：

- **docker compose** 编排：PostgreSQL + Go API + nginx（静态托管 + TLS 终止 + `/api` 反代）
- **所有敏感配置经 `.env`/环境变量注入**（DB、JWT secret、账号、SMTP、AI key），不硬编码、不入库、不进 git
- **TLS 证书与域名由用户自行准备并挂载**（本任务只提供挂载点与 nginx 配置）
- 重写 `Makefile` 与 `README.md` 为 Go + 前端；**删除 `src-tauri/`、Cargo 文件、Rust 相关脚本与构建目标**

## Requirements

### R1 容器化
- R1.1 `server/Dockerfile`：多阶段（`golang:1.25-alpine` 构建 → 精简运行镜像），产出 `kairos-api` 二进制；运行镜像含时区数据（业务 +08:00）
- R1.2 `deploy/Dockerfile.web`：多阶段（`node:22-alpine` 构建前端 `dist` → `nginx:alpine` 托管）
- R1.3 `.dockerignore`（根 + `server/`），排除 `node_modules`、`dist`、`.git`、`src-tauri` 等

### R2 compose 编排
- R2.1 `compose.yaml`：服务 `db`（postgres:16-alpine + 持久卷 + healthcheck）、`api`（依赖 db healthy）、`web`（依赖 api，暴露 80/443）
- R2.2 `api` 用 `env_file: .env` 注入全部配置；`KAIROS_DATA_DIR` 挂载持久卷（AI 密钥/DEK 文件）
- R2.3 `api` healthcheck 打 `/healthz/live`；`web` 挂载 `deploy/nginx.conf` 与证书目录
- R2.4 网络：内部网络；仅 `web` 暴露宿主端口；`api`/`db` 不暴露宿主端口

### R3 nginx / TLS
- R3.1 `deploy/nginx.conf`：80 → 301 跳转 https；443 ssl 证书从挂载目录读取（路径可配置）
- R3.2 SPA 回退：`try_files $uri /index.html`；静态资源长缓存
- R3.3 `/api/` 反代到 `http://api:8080`（保留前缀）；SSE 路径关闭 `proxy_buffering` 并放宽读超时（AI 晨报流式）
- R3.4 安全头（`X-Content-Type-Options`、`X-Frame-Options` 等）；HSTS 可选
- R3.5 证书目录不纳入版本控制（`.gitignore`）；提供说明文档，用户放入 `fullchain.pem`/`privkey.pem`

### R4 配置模板
- R4.1 `.env.example`（仓库根）：列出全部键与占位值/说明，含 `DATABASE_URL`、`JWT_SECRET`、`APP_USER`、`APP_PASSWORD_HASH`、`KAIROS_API_PORT`、`KAIROS_DATA_DIR`、`SMTP_HOST/PORT/USER/PASS/FROM/TO/TLS`
- R4.2 `.env` 已被 `.gitignore` 忽略；`.env.example` 例外保留（已配置）
- R4.3 提供生成账号密码哈希的方式：`make gen-password PASSWORD=...`（已有 `cmd/genpassword`）

### R5 构建脚本与文档
- R5.1 重写 `Makefile`：`dev`（本地 dev PG 容器 + Go + Vite）、`build`、`check`、`test`、`lint`、`clean`、`verify`、`gen-password`、`up`/`down`/`logs`/`ps`（compose）；**移除全部 cargo/Rust/Android 目标**
- R5.2 更新 `README.md`：技术栈改为 Go + PostgreSQL + React；开发环境改为 Go/Node/Docker；新增部署章节（.env 配置、证书放置、`docker compose up`）；移除 Tauri/Rust/Android 描述
- R5.3 `.env` 中 `DATABASE_URL`：compose 内主机为服务名 `db`；本地 dev 为 `localhost`

### R6 移除 Rust
- R6.1 删除 `src-tauri/` 整目录（含 `Cargo.toml`/`Cargo.lock`/`tauri.conf.json`/`gen/android`）
- R6.2 删除 `scripts/android-*.sh`（Tauri/Android 工具脚本）
- R6.3 确认无任何 Rust/Tauri 引用残留（Makefile/README/package.json/CI）
  - `.github/workflows/release.yml` 已于 W1.5 移除（本任务复核）

## 非功能需求

- **安全**：secret 仅 `.env`；TLS 终止于 nginx；证书与 `.env` 均不入库；运行镜像非 root（可行时）
- **可运维**：`docker compose up -d` 一键起停；健康检查；结构化日志到 stdout
- **离线友好**：构建镜像时可访问依赖源；运行时不依赖外部 CDN
- **可回滚**：`pre-go-migration` tag 保留旧版本

## Scope OUT（Must NOT have）

- ❌ 申请/签发 TLS 证书、配置 DNS、租用服务器（用户自理）
- ❌ CI/CD 流水线（本任务不做；`.github` 当前为空）
- ❌ 多环境（staging/prod 分离）、K8s、自动扩缩容
- ❌ SMTP/AI 服务的实际凭据（由用户填入 `.env`）
- ❌ 数据库备份策略（单用户个人部署，超出本次范围）

## Acceptance Criteria（对应 parent AC5/AC7/AC8）

- [ ] AC1 `docker compose config` 校验通过（语法与变量引用无误）
- [ ] AC2 `docker compose build` 成功（api 与 web 镜像均构建通过）——或在不联网时至少 `Dockerfile` 静态复核通过
- [ ] AC3 `go build ./...` 通过；`npm run build` 通过
- [ ] AC4 `make verify` 在新 Makefile 下通过（check+lint+test+audit，无 Rust 目标）
- [ ] AC5 `.env.example` 覆盖全部运行所需键；`grep -rn "src-tauri\|cargo\|tauri" Makefile package.json README.md` 仅剩无关命中
- [ ] AC6 `src-tauri/` 目录不存在；`scripts/android-*.sh` 不存在
- [ ] AC7 部署文档说明：把 `.env`（含 SMTP/账号/JWT）与证书放入指定位置后 `docker compose up -d` 可启动（用户环境实测）
- [ ] AC8 浏览器经 HTTPS 完成登录与主流程（用户提供服务器/证书后实测；本地以 `http://localhost` + 自签或反代验证）

## Notes

- TLS 证书路径在 `nginx.conf` 中以固定挂载点（如 `/etc/nginx/certs/`）引用，用户把证书文件放入宿主对应目录即可
- `KAIROS_DATA_DIR` 必须持久化（AI API key 密文与同步 DEK 存在磁盘），compose 用命名卷挂载
- 本地验证不承载部署：`make dev` 仅起 dev PG 容器 + Go + Vite
