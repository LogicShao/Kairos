# Kairos 后端迁移 - 执行计划

> 有序分波推进，每波对应 child 任务；完成前置波后方可 start 下一波。
> 评审门：每波结束需 `trellis-check` 复核 + 依赖波确认，才进入下一波。

## 波次总览

| 波 | Child 任务 | 交付物 | 验证命令（WSL） |
|---|---|---|---|
| W1 | 09-08-wsl-dev-env | WSL 工具链就绪、Makefile dev 骨架 | `go version` / `docker ps` / `psql --version` |
| W2 | 09-08-postgres-schema | server/internal/store/migrations、sqlc 生成 | `golang-migrate` up + `sqlc generate` + `go test ./internal/store/...` |
| W3 | 09-08-go-backend-auth | server 骨架、config、JWT、/healthz、/api/auth/* | `go test ./...` + `curl /healthz` |
| W4 | 09-08-go-backend-core | tasks/courses/exams/term-phase handler+store+domain | `go test ./internal/...` + curl CRUD 冒烟 |
| W5 | 09-08-go-backend-calendar | 周课表/日历/今日概览聚合 | `go test ./internal/domain/calendar/...` + curl |
| W6 | 09-08-go-backend-pomodoro | 引擎、运行态、session、finish-phase、配置档 | `go test ./internal/domain/pomodoro/...` + curl |
| W7 | 09-08-go-backend-sync | 快照导出/导入、WebDAV client、手动同步端点 | `go test ./internal/domain/sync/...` + curl test |
| W8 | 09-08-go-backend-ai | AI 配置、晨报生成 SSE、规则降级、今日概览聚合 | `go test ./internal/domain/ai/...` + SSE curl |
| W9 | 09-08-go-backend-notifications | SMTP mailer、集中调度（考试/每日/remind_at/AI 7:00/番茄钟） | MailHog 实测 + `go test ./internal/domain/notify/...` |
| W10 | 09-08-frontend-api-layer | src/lib/api/、组件替换、LZU 移除、tauri 依赖清零 | `npx tsc --noEmit` + `npm run lint` + 浏览器冒烟 |
| W11 | 09-08-deployment | compose.yaml、nginx、TLS、.env 模板、README/Makefile 更新 | `docker compose up` + 浏览器 HTTPS 全流程 AC5 |

## 波次详细步骤（每波 = 一个 child 的 implement.md 输入）

### W1 wsl-dev-env
1. WSL2 Ubuntu 下安装：Go 1.24+（官方 tarball 至 /usr/local）、Docker（Docker Desktop WSL2 backend 或 docker-ce）、PostgreSQL（`sudo apt install postgresql`）
2. `createdb kairos_dev`；本地 PG 运行；创建 dev 用户
3. 仓库克隆/移动到 WSL 原生路径（建议 `~/proj/Kairos`）
4. 根 `Makefile` 增加 `go-*` 目标（build/test/lint/dev）与 `server/` 关联；保留前端目标
5. 验证：`make dev` 能同时启动 Go（占位）+ Vite

### W2 postgres-schema
1. `server/internal/store/migrations/` 编写 golang-migrate SQL：按 design 4.2/4.3 建 13 张表 + 索引 + CHECK + 部分唯一索引 + 种子（pomodoro_profiles 3 内置档、单例表默认行）
2. 时间归一化：TIMESTAMPTZ/DATE/TIME；空值统一 NULL
3. `server/db/queries/` 编写 sqlc 查询（tasks/courses/exams/term_phases/pomodoro/semester/sync/ai/notify）
4. `sqlc.yaml` + `sqlc generate`；生成 store 层
5. 测试：迁移幂等性 + 关键查询单测（对齐 Rust db/* 测试行为）
6. 验证：`make go-test` 通过；`psql` 检查表结构

### W3 go-backend-auth
1. `server/cmd/api/main.go`、`internal/config`（DB/SMTP/JWT/APP_USER/APP_PASSWORD_HASH 环境变量加载）
2. 中间件：logger（slog）/recover/requestid/auth（JWT Bearer）
3. 认证：`POST /api/auth/login`（bcrypt 校验）、`POST /api/auth/logout`（可选 token 黑名单，简化：前端清 token）、`GET /api/auth/me`、`GET /healthz`
4. 初始账号：密码哈希生成工具（`make gen-password` 输出 bcrypt hash 写 .env）
5. 测试：登录成功/失败/401/健康检查
6. 验证：`curl -X POST /api/auth/login` 拿 JWT → 带 token 访问受保护端点

### W4 go-backend-core
1. domain 纯逻辑：importer（课程/考试文本解析，对齐 importers.rs 测试）、termphase 判定（对齐 term_phase.rs）
2. store 补齐：tasks（含筛选排序/每日任务/remind_at 归一化）、courses（周规则字段/批量重置）、exams、term_phases（默认阶段种子/当前状态）
3. handler + DTO（对齐 design §5 契约与前端 src/types/*）
4. 测试：每模块 CRUD + 边界（软删、筛选、导入去重）
5. 验证：curl 冒烟 CRUD；`go test ./...`

### W5 go-backend-calendar
1. domain/calendar：周课表聚合（课程周规则 + 考试）、日历日聚合（+有截止待办）、学期周索引
2. handler：`GET /api/calendar/week|day`、`GET /api/briefing/today`
3. 测试：对齐 schedule.rs 现有测试场景（周模式匹配、单双周、假期）
4. 验证：curl 周/日/今日概览

### W6 go-backend-pomodoro
1. domain/pomodoro：引擎（阶段机、暂停/重置/中断、时长校验），对齐 timer.rs
2. store：config 单例、runtime_state、sessions（开始/结束/按日统计）、profiles（内置保护）
3. handler：design §5 pomodoro 端点 + `POST /api/pomodoro/finish-phase`（落库 + 触发邮件钩子，钩子由 W9 实现，先留接口）
4. 测试：引擎状态机全场景 + API
5. 验证：curl 开始→结束落库；完成轮数统计正确

### W7 go-backend-sync
1. domain/sync：v2 快照协议（export_all/import_all：LWW 合并、sync_id 匹配、墓碑传播、etag 条件上传），对齐 sync/exporter.rs
2. WebDAV client（Basic Auth、ETag、超时），对齐 sync/webdav.rs
3. handler：config/test/now；AI 设置加密同步（AES-256-GCM+PBKDF2+DEK），对齐 sync/ai_settings.rs
4. 测试：LWW 冲突合并、etag、AI 设置加解密 roundtrip
5. 验证：`go test ./internal/domain/sync/...` + curl test/now

### W8 go-backend-ai
1. domain/ai：provider 接口（DeepSeek 非流式 + SSE 流式）、提示词/输出校验（对齐 ai/prompt.rs）、规则降级（对齐 ai/rule.rs）、API key AES-256-GCM 加解密
2. 晨报：按日缓存、in-flight 护栏、`POST /api/ai/morning-brief/generate` SSE 流式、今日概览聚合复用 W5
3. handler：config、recovery-key、morning-brief（get/generate）
4. 测试：流式分块输出、规则降级、缓存幂等
5. 验证：curl SSE 流式；配置读写

### W9 go-backend-notifications
1. notify/mailer：SMTP（go-mail 或 net/smtp）、中文模板、失败重试、发送 goroutine 池
2. notify/scheduler：集中调度注册（考试偏移、每日 reminder_time、一次性 remind_at、AI 7:00 +08:00、番茄钟 finish-phase 钩子），启动时重算 + 增删改后重算
3. handler：`GET/PATCH /api/notify/config`（exam_offsets 变更重算）
4. 测试：调度时间计算（对齐 notifications/* 测试）、邮件发送（MailHog SMTP）
5. 验证：MailHog 收件箱收到各类型邮件

### W10 frontend-api-layer
1. `src/lib/api/client.ts` + 模块 API 封装（对齐 design §9）
2. 逐组件替换 invoke → api 调用；事件替换（pomodoro 本地时钟 + finish-phase、sync 响应刷新、ai 挂载拉取/SSE）
3. 剪贴板改 navigator；移除 LZU/Android 返回键/tauri-events；`src/types` 剔除 lzu
4. `VITE_API_BASE_URL` 环境；`@tauri-apps/*` 依赖从 package.json 移除
5. 验证：`npx tsc --noEmit` + `npm run lint` + 浏览器全流程冒烟；grep 无 `@tauri-apps`

### W11 deployment
1. Dockerfile：api（多阶段 Go）、web（nginx 托管 dist + 反代 /api）
2. compose.yaml：db(healthcheck)/api/web + networks/volumes；.env.example
3. nginx.conf：TLS（挂载证书）、SPA 回退、/api 反代、HTTP→HTTPS
4. 更新根 Makefile（build/deploy/verify 指向 Go+前端）与 README（移除 Rust/Tauri 说明）
5. 验证：`docker compose up` → 浏览器 HTTPS 登录→全模块冒烟（AC5）

## 依赖矩阵

| 波 | 依赖 | 说明 |
|---|---|---|
| W2 | W1 | 需 WSL/Go/PG |
| W3 | W2 | 需 schema + store |
| W4 | W3 | 需骨架/认证 |
| W5 | W4 | 需课程/考试/待办数据 |
| W6 | W4 | 需任务关联 |
| W7 | W4 | 需核心表导出 |
| W8 | W4,W5 | 需概览聚合 |
| W9 | W6,W4,W8 | 需触发点 |
| W10 | W3-W9 | 需 API 就绪 |
| W11 | W1-W10 | 需全部 |

## 评审门（每波结束）

1. 该波 child 的 `implement.jsonl`/`check.jsonl` 已配置（W1 后逐波补充）
2. `trellis-check` 复核：规范符合（Go 侧按 Go 层 spec，前端按 frontend spec）、lint/typecheck/test 绿
3. 该波 curl/浏览器冒烟证据落 `.trellis/tasks/<child>/research/` 或会话记录
4. 依赖下一波所需的 API/数据就绪后，才 `task.py start` 下一波 child

## 回滚点

- 每波完成 = 可运行状态（可提交）
- `git tag pre-go-migration` 于迁移开始前，标记旧 Tauri 版本；随时可切回
- 迁移期间 src-tauri 保留，直到 W10 前端替换完成、W11 部署验证通过后再移除/归档
- 单波失败：修复该波 child（`task.py start` 该 child → 修复 → check）→ 不阻塞下一波环境准备

## 完成条件（对应 prd AC1-AC8）

- AC1 `go test ./...` + golangci-lint 绿（W1-W9 累积）
- AC2 `tsc --noEmit` + eslint 绿 + 无 @tauri-apps 残留（W10）
- AC3 REST 冒烟全通过（W3-W9 累积）
- AC4 邮件实测（MailHog → 生产 SMTP 配置）（W9）
- AC5 docker compose HTTPS 全流程（W11）
- AC6 页面可用 + 数据持久化（W10+W11）
- AC7 src-tauri 不再参与构建；README/Makefile 更新（W11）
- AC8 WSL make dev 一键启动（W1）
