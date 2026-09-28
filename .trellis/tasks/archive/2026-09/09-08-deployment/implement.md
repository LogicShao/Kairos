# W11 部署 - 执行计划

> 复杂任务，`prd.md` + `design.md` + 本文件评审后 `task.py start` 再实现。

## 工作包（WP）

### WP1 容器化文件 `[基础]`
1. `server/Dockerfile`（多阶段 Go 构建；`TZ=Asia/Shanghai`；`/data` 目录）
2. `deploy/Dockerfile.web`（node 构建 → nginx 托管）
3. `.dockerignore` + `server/.dockerignore`
4. `deploy/certs/.gitkeep` + `.gitignore` 追加 `deploy/certs/*`（保留 `.gitkeep`）
- 验证：`docker build -f server/Dockerfile server/` 与 `docker build -f deploy/Dockerfile.web .`（或静态复核）

### WP2 compose + nginx `[依赖 WP1]`
1. `compose.yaml`：db/api/web、卷、healthcheck、内网隔离、`env_file: .env`
2. `deploy/nginx.conf`：TLS、SPA 回退、`/api/` 反代、SSE 关闭缓冲、安全头、静态缓存
- 验证：`docker compose config`

### WP3 配置模板
1. `.env.example`：全部键 + 占位值与说明
2. 确认 `.env` 被忽略、`.env.example` 保留
- 验证：`grep` 覆盖全部 config 键

### WP4 构建脚本与文档 `[独立]`
1. 重写 `Makefile`（Go + 前端 + compose；删除 Rust/Android 目标）
2. 更新 `README.md`（技术栈/环境/命令/架构/部署章节；移除 Tauri/Rust/Android）
- 验证：`make -n <targets>`；`grep -rn "cargo\|src-tauri\|tauri" Makefile README.md` 仅剩无关命中

### WP5 移除 Rust `[独立，最后]`
1. 删除 `src-tauri/`
2. 删除 `scripts/android-*.sh`
3. 复核 `.github`（应为空/无 release.yml）
- 验证：`test ! -d src-tauri && echo ok`；`ls scripts/`

### WP6 验证
1. `docker compose config` → 0
2. `docker compose build` → 0（本地有 Docker）
3. `go build ./...` + `npm run build` → 0
4. `make verify` → 0
5. 残留检查：`grep -rn "src-tauri\|cargo tauri" Makefile package.json README.md`
6. （可选）`docker compose up -d` 本地 http 冒烟后 `down`
- Spec 更新（Phase 3.3）：部署相关约定写入 `.trellis/spec/`
- 提交（Phase 3.4）：分批提交

## 依赖矩阵

| WP | 依赖 |
|---|---|
| WP1 | — |
| WP2 | WP1（Dockerfile 路径被 compose 引用） |
| WP3 | — |
| WP4 | — |
| WP5 | — |
| WP6 | WP1–WP5 |

## 验证命令

```bash
docker compose config
docker compose build
cd server && go build ./...
npm run build
make verify
grep -rn "src-tauri\|cargo\|tauri" Makefile README.md package.json
test ! -d src-tauri && echo "src-tauri removed"
```

## 回滚点

- `pre-go-migration` tag；`src-tauri` 可由 git 恢复
- 每个 WP 可单独提交/回退

## 用户需自行完成（不在本任务）

- 准备服务器/域名/DNS、签发/放置 TLS 证书到 `deploy/certs/`
- 填写 `.env`（DB 密码、JWT secret、账号、SMTP 凭据）
- 在服务器上 `docker compose up -d`

## 完成条件（对应 prd AC1–AC8）

- AC1 compose config 通过；AC2 compose build 通过；AC3 go build + npm build；AC4 make verify
- AC5 .env.example 完整；AC6 src-tauri 与 android 脚本删除
- AC7 文档说明部署步骤；AC8 用户环境 HTTPS 实测
