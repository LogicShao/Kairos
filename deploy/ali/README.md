# 部署到 ali 服务器（1Panel + OpenResty）

面向服务器 `ali`（Ubuntu 22.04 / Docker 29 / 1Panel 管理 OpenResty）的部署说明。
**本目录的脚本由你手动执行，不自动部署。**

## 架构

```
浏览器 ──HTTPS──► ali:443（1Panel OpenResty，TLS 终止）
                      └─ / ──► 127.0.0.1:18081（kairos-web，HTTP）
                                  ├─ /        React 静态
                                  └─ /api/ ──► kairos-api:8080 ──► kairos-db:5432
```

- 与服务器上已有服务一致：应用容器只暴露 `127.0.0.1:<端口>`，80/443 由 1Panel 接管，证书由 1Panel 管理。
- `api` / `db` 仅容器内网可达，不映射宿主端口。

## 端口与命名

| 项 | 值 | 说明 |
|---|---|---|
| web 宿主端口 | `127.0.0.1:18081` | 可用 `KAIROS_WEB_PORT` 覆盖（18080 已被 Misaka 占用） |
| 域名 | `kairos.misaka-net.top` | 按需替换 |
| 容器名 | `kairos-web` / `kairos-api` / `kairos-db` | |
| 数据卷 | `kairos_pg_data` / `kairos_ai_data` | `docker volume ls` 可见 |

## 前置

- 服务器已装 Docker + Compose（已有）。
- 你对该服务器有 sudo（`docker` 组当前无成员，脚本会自动检测用 `docker` 或 `sudo docker`）。

## 步骤

### 1. DNS
在 `misaka-net.top` 添加 A 记录：`kairos` → 服务器公网 IP。（不上 CF，直接解析真实 IP。）

### 2. 推送代码（本地执行）
在本机仓库根执行（rsync 直传，服务器无需 git）：
```bash
bash deploy/ali/rsync-push.sh          # 仅推送
bash deploy/ali/rsync-push.sh all      # 推送 + 远端部署（一键）
```
默认推送到 `ali:~/proj/Kairos/`，排除 `.git`/`node_modules`/`dist`/`.env` 等。
`all` 模式会用 `ssh` 触发远端 `deploy/ali/deploy.sh`（要求远端 docker 无需交互密码，或已配好权限）。

### 3. 配置 `.env`
```bash
cp .env.example .env
```
至少填写：
- `POSTGRES_PASSWORD`（强随机）
- `DATABASE_URL`（保持 host 为 `db`）：`postgres://kairos:<同上密码>@db:5432/kairos?sslmode=disable`
- `JWT_SECRET`（`openssl rand -hex 32`）
- `APP_USER` / `APP_PASSWORD_HASH`（`make gen-password PASSWORD=...` 生成哈希）
- SMTP（可选，不填则不发邮件）

> `.env` 不要提交；`compose.ali.yml` 会读取仓库根的 `.env`。

### 4. 部署（服务器上）
若第 2 步用了 `all` 模式，本步已自动完成；否则在服务器执行：
```bash
cd ~/proj/Kairos && bash deploy/ali/deploy.sh
# 若无 docker 权限：sudo -E bash deploy/ali/deploy.sh
```
脚本会：校验 compose → 构建并启动 → 轮询健康检查（web 200 且 `/api/auth/me` 401）→ 打印结果。

### 5. 1Panel 建站 + 反代（关键一步，你手动做）
1. 1Panel → **网站** → 创建网站 → 类型选**反向代理**
2. 主域名：`kairos.misaka-net.top`
3. 代理地址：`http://127.0.0.1:18081`（保留 Host、开启 WebSocket 转发，如界面有该项）
4. 保存后 → **HTTPS**：申请证书（Let's Encrypt）或选用现有证书，开启强制 HTTPS
5. 无需改 OpenResty 文件（1Panel 会生成 `conf.d/kairos.misaka-net.top.conf`）

### 6. 验证
- 本机：`curl -I http://127.0.0.1:18081/` → 200
- 公网：`https://kairos.misaka-net.top` → 出现登录页，用 `APP_USER` / 密码进入

## 常用运维

```bash
# 查看状态 / 日志（按你们的 docker 权限决定是否 sudo）
docker compose -f compose.ali.yml ps
docker compose -f compose.ali.yml logs -f --tail=200
docker compose -f compose.ali.yml down          # 停（保留数据卷）
docker compose -f compose.ali.yml up -d --build # 更新
```

- **数据备份**：`kairos_pg_data`（业务数据）与 `kairos_ai_data`（AI key 密文 + 同步 DEK）。
- **更新**：本机改完后跑 `bash deploy/ali/rsync-push.sh all`（推送 + 重建）。
- **回滚**：本机切回旧版本后重跑 `bash deploy/ali/rsync-push.sh all`；数据卷不受影响。

## 注意

- 服务器资源：2 vCPU / 3.4G / 约 15G 可用磁盘。已为容器设置内存上限：`db 192M`（PG `shared_buffers=64MB`、最多 20 连接）、`api 128M`、`web 64M`，合计 ≈ 384M。
- 首次构建（Go 编译 + Node/Vite 构建）会**瞬时**占用较多 CPU/内存（0.5–1.5G），属一次性，构建完成即释放。
- 若与 1Panel 的 PostgreSQL 端口冲突：本方案的 `kairos-db` **不映射宿主端口**，不会与 `127.0.0.1:5432` 冲突。
- 私密性：这是**未上 CF 的真实 IP 暴露**部署；按前述讨论，建议后续再加统一认证（Authelia）或 CF Tunnel，本次不做。
