# W11 部署 - 技术设计

> 契约来源：`server/internal/config/config.go`（环境变量）、`server/cmd/api/main.go`（启动流程）、`server/internal/httpapi/router.go`（路由）。

## 1. 部署拓扑

```
浏览器 ──HTTPS──▶ web (nginx:443, TLS 终止)
                     ├─ /            → /usr/share/nginx/html (React dist, SPA 回退)
                     └─ /api/  ──▶ api:8080 (Go, chi)
                                     ├─ 业务 handler + JWT
                                     ├─ 调度器 (goroutine) ──▶ SMTP
                                     └─ pgxpool ──▶ db:5432 (postgres:16)
```
- 单机 docker compose；仅 `web` 暴露宿主端口 80/443。
- `api`/`db` 仅在内网可达。

## 2. 文件清单（新增/修改/删除）

| 文件 | 动作 | 说明 |
|---|---|---|
| `server/Dockerfile` | 新增 | 多阶段构建 Go API |
| `deploy/Dockerfile.web` | 新增 | 多阶段构建前端 + nginx |
| `deploy/nginx.conf` | 新增 | TLS + SPA + /api 反代 + SSE |
| `compose.yaml` | 新增 | db/api/web 编排 |
| `.env.example` | 新增 | 全部环境变量模板 |
| `.dockerignore` | 新增 | 构建上下文排除 |
| `server/.dockerignore` | 新增 | 同上（api 构建上下文） |
| `deploy/certs/.gitkeep` | 新增 | 证书挂载目录占位（内容 gitignore） |
| `Makefile` | 重写 | Go + 前端 + compose |
| `README.md` | 更新 | 技术栈/环境/命令/部署 |
| `src-tauri/` | 删除 | Rust/Tauri 退役 |
| `scripts/android-*.sh` | 删除 | Tauri/Android 脚本 |
| `.gitignore` | 追加 | `deploy/certs/*`（保留 .gitkeep） |

## 3. 环境变量（`server/internal/config/config.go` 为准）

| 变量 | 必填 | 说明 / 默认 |
|---|---|---|
| `DATABASE_URL` | ✅ | PG 连接串；compose 内 `postgres://kairos:<pw>@db:5432/kairos?sslmode=disable` |
| `JWT_SECRET` | ✅ | ≥32 字节；HS256 签名 |
| `APP_USER` | ✅ | 单账号用户名 |
| `APP_PASSWORD_HASH` | ✅ | bcrypt 哈希（`make gen-password` 生成） |
| `KAIROS_API_PORT` | ✅ | API 监听端口，默认 8080 |
| `KAIROS_DATA_DIR` | ✅ | 服务端数据目录（AI key 密文 + DEK），默认 `./data`；compose 挂 `/data` |
| `SMTP_HOST` | ➖ | 为空则禁用邮件（调度器仅记日志） |
| `SMTP_PORT` | ➖ | 默认 587 |
| `SMTP_USER` / `SMTP_PASS` | ➖ | SMTP 凭据 |
| `SMTP_FROM` / `SMTP_TO` | ➖ | 发件人 / 收件人 |
| `SMTP_TLS` | ➖ | 默认 `starttls` |

支持别名前缀：`KAIROS_*` / `APP_*`（如 `KAIROS_DB_URL`、`APP_JWT_SECRET`），裸名 `DATABASE_URL`/`JWT_SECRET` 亦可。本任务统一用裸名 + `APP_*` + `KAIROS_*`（见上方表）。

可选 AI（不配置则 AI 晨报不可用但不影响其他功能）：AI 的 base_url/model/api_key 经 `PATCH /api/ai/config` 在应用内设置（key 加密存库），**不走环境变量**；仅 `KAIROS_DATA_DIR` 提供其密钥文件目录。

## 4. Dockerfile 设计

### `server/Dockerfile`
```dockerfile
# build
FROM golang:1.25-alpine AS build
WORKDIR /src
RUN apk add --no-cache ca-certificates tzdata
COPY go.mod go.sum ./
RUN go mod download
COPY . .
RUN CGO_ENABLED=0 go build -trimpath -ldflags="-s -w" -o /out/kairos-api ./cmd/api

# run
FROM alpine:3.20
RUN apk add --no-cache ca-certificates tzdata
ENV TZ=Asia/Shanghai
WORKDIR /app
COPY --from=build /out/kairos-api /usr/local/bin/kairos-api
RUN mkdir -p /data
EXPOSE 8080
ENTRYPOINT ["kairos-api"]
```
- `main.go` 会尝试 `config.LoadEnv(".env", "../.env")`，容器内无 `.env`（由 env 注入），非错误。
- 迁移在启动时自动执行（`migrate.Up`）。

### `deploy/Dockerfile.web`（构建上下文 = 仓库根）
```dockerfile
FROM node:22-alpine AS build
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci
COPY . .
RUN npm run build

FROM nginx:alpine
COPY deploy/nginx.conf /etc/nginx/conf.d/default.conf
COPY --from=build /app/dist /usr/share/nginx/html
EXPOSE 80 443
```

## 5. nginx 设计（`deploy/nginx.conf`）

- `server { listen 80; return 301 https://$host$request_uri; }`
- `server { listen 443 ssl; ssl_certificate /etc/nginx/certs/fullchain.pem; ssl_certificate_key /etc/nginx/certs/privkey.pem; ... }`
- `root /usr/share/nginx/html; location / { try_files $uri /index.html; }`
- `location /api/ { proxy_pass http://api:8080; proxy_set_header Host $host; ... proxy_read_timeout 300s; }`
- AI 流式：`location /api/ai/morning-brief/generate { proxy_buffering off; proxy_cache off; proxy_read_timeout 600s; }`
- 静态资源：`location ~* \.(js|css|woff2|webp|svg)$ { expires 30d; add_header Cache-Control "public, immutable"; }`
- 安全头：`X-Content-Type-Options nosniff`、`X-Frame-Options DENY`、`Referrer-Policy strict-origin-when-cross-origin`

## 6. compose 设计（`compose.yaml`）

```yaml
services:
  db:
    image: postgres:16-alpine
    environment: { POSTGRES_USER, POSTGRES_PASSWORD, POSTGRES_DB }
    volumes: [pg_data:/var/lib/postgresql/data]
    healthcheck: pg_isready
    # 不暴露宿主端口
  api:
    build: { context: ./server }
    env_file: [.env]
    environment: { KAIROS_DATA_DIR: /data }
    depends_on: { db: { condition: service_healthy } }
    volumes: [ai_data:/data]
    healthcheck: wget -qO- http://localhost:8080/healthz/live
    # 不暴露宿主端口
  web:
    build: { context: ., dockerfile: deploy/Dockerfile.web }
    ports: ["80:80", "443:443"]
    volumes:
      - ./deploy/certs:/etc/nginx/certs:ro
    depends_on: [api]
volumes: { pg_data: {}, ai_data: {} }
```
- `POSTGRES_*` 与 `DATABASE_URL` 必须一致（`db` 服务名）。
- 证书目录 `./deploy/certs` 只读挂载。

## 7. Makefile 设计（重写）

保留：`help`、`gen-password`、`go-*`。新增：`up`/`down`/`logs`/`ps`/`build-web`/`db-dev`/`db-stop`。重写：`dev`、`build`、`check`、`test`、`lint`、`clean`、`verify`、`bump`（仅 package.json）。
删除：`CARGO`/`MANIFEST`、`cargo tauri` 目标、`android-*`、`check-all`。

## 8. README 设计（重写相关章节）

- 技术栈表：应用壳→纯 Web；后端→Go 1.25（chi/pgx/sqlc）；数据库→PostgreSQL 16；同步→WebDAV；通知→SMTP 邮件
- 开发环境：Go 1.25 / Node 18+ / Docker（dev PG 容器）；`make dev`
- 命令表：新 Makefile 目标
- 架构图：浏览器 → nginx → Go → PostgreSQL → SMTP/WebDAV
- 新增「部署」章节：填 `.env`、放证书、`docker compose up -d`

## 9. 兼容性与回滚

- `pre-go-migration` tag 保留；`src-tauri/` 删除可由 git 恢复
- 证书/`.env` 不入库；部署失败可回退镜像构建

## 10. 验证

- `docker compose config`（语法/变量）
- `docker compose build`（本地有 Docker）
- `go build ./...` + `npm run build`
- `make verify`
- 本地 `docker compose up -d` 冒烟（可选，用 http 或自签；用户服务器/TLS 由其配置）
