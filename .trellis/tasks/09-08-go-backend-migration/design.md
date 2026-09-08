# Kairos 后端迁移 - 技术设计

> 决策完备的技术架构：Go 后端 + PostgreSQL + 纯 Web 前端 + SMTP 邮件通知 + 单用户部署。
> 本文件是 children 任务的契约来源；child 细化时不得违反此处边界。

## 1. 技术栈

| 层 | 选型 | 理由 |
|---|---|---|
| 语言/运行时 | Go 1.24+ | 单二进制、并发模型贴合集中调度 |
| Web 框架 | chi（`github.com/go-chi/chi/v5`） | 标准库风格、轻量、中间件链 |
| 路由/中间件 | chi + 自研（auth/log/recover/request-id） | 避免重框架 |
| SQL 访问 | `github.com/jackc/pgx/v5`（pgxpool）+ `sqlc` | pgx 原生性能；sqlc 编译期校验，贴合现有手写 SQL 风格 |
| 迁移 | `golang-migrate`（或自研 embed 迁移器） | 版本化 schema，镜像 Rust migrations.rs |
| 认证 | `github.com/golang-jwt/jwt/v5` + bcrypt（`golang.org/x/crypto/bcrypt`） | 单账号 JWT |
| 密码学 | `crypto/aes`、`crypto/cipher`（AES-256-GCM）、`crypto/pbkdf2`、sha256、md5 | AI key 加密 + WebDAV 设置同步 |
| 邮件 | `net/smtp`（或 `github.com/wneessen/go-mail`） | SMTP 推送；go-mail 支持 TLS/队列重试更稳 |
| HTTP 客户端 | `net/http` | WebDAV / DeepSeek / SSE 调用 |
| 流式 | SSE（`text/event-stream`）+ fetch ReadableStream | 替代 Tauri Channel |
| 日志 | `log/slog`（Go 标准库） | 结构化日志 |
| 测试 | 标准 `testing` + `github.com/stretchr/testify` + `httptest` | tests-after，行为对齐 Rust 测试 |
| 静态检查 | gofumpt + golangci-lint v2 | 质量门 |
| 前端 | 保留 Vite + React 19 + TS；`@tanstack/react-query`（可选，见 R11） | 最小改动替换 invoke 层 |

## 2. 系统架构

```
浏览器 (React SPA, Vite build)
   │  HTTPS
   ▼
nginx (TLS 终止, 静态托管 dist/, /api 反代到 Go)
   │
   ▼
Go API (chi)
   ├─ 认证中间件 (JWT)
   ├─ 业务 handler (tasks/courses/exams/semester/pomodoro/calendar/sync/ai/notify)
   ├─ 集中调度器 (goroutine + timer): 番茄钟阶段/考试/每日任务/remind_at/AI 7:00
   │      └─ SMTP 邮件推送
   └─ PostgreSQL (pgxpool)
```

- 单进程 Go 服务承载 HTTP + 调度器；无消息队列、无 Redis（单用户规模）
- 前端与 API 同源部署（nginx 反代 `/api`），避免 CORS

## 3. Go 项目结构

新增 `server/` 目录（与 `src/`、`src-tauri/` 平级），后续可演进为 monorepo 顶层：

```
server/
  cmd/api/main.go          # 入口：配置加载、DB 连接、调度器启动、路由注册
  internal/
    config/                # 环境变量配置 (DB/SMTP/JWT/账号)
    httpapi/               # chi 路由 + 中间件 + handler 装配
      middleware/          # auth / logger / recover / requestid
      handlers/            # 每模块一个 handler 文件
      dto/                 # 请求/响应结构体（对齐前端 src/types）
    domain/                # 业务规则（可纯函数单测）
      pomodoro/            # 引擎（对齐 timer.rs）
      calendar/            # 周课表/日历聚合（对齐 schedule.rs）
      termphase/           # 学期阶段判定（对齐 term_phase.rs）
      importer/            # 文本导入解析（对齐 importers.rs）
      sync/                # 快照导出/导入 LWW + WebDAV client（对齐 sync/）
      notify/              # SMTP + 调度规则
    store/                 # pgx 数据访问（sqlc 生成）
      queries/             # .sql 查询文件
      migrations/          # golang-migrate SQL 迁移
    app/                   # 应用装配/依赖注入（Store + Notifier + Scheduler）
```

前端改动仅涉及：新增 `src/lib/api/`（HTTP client 层）、`src/types/` 微调、各组件 import 替换；不动组件结构。

## 4. 数据模型（PostgreSQL 映射）

### 4.1 原则
- **时间统一 UTC 存储（TIMESTAMPTZ）**，应用层以 +08:00 转换展示；修复 Rust 时代 7 种格式并存问题
- 纯日期（due_date / date_key / 学期日期）用 `DATE`；业务时刻（start_time/end_time/reminder_time）用 `TIME`
- JSON 用 `JSONB`（tags、exam_offsets、notification_rules、profile）
- 软删除保留 `deleted_at TIMESTAMPTZ NULL`；**物理 FK 改为 ON DELETE SET NULL 语义保留**（与现有 SQLite 一致）
- 布尔用 `BOOLEAN`
- `'' vs NULL` 统一为 NULL（空值用 NULL，不再用空串）
- 主键：现有本地自增 `INTEGER PRIMARY KEY` → `BIGSERIAL`；**保留 `sync_id TEXT UNIQUE`（NULL 允许）**作为跨设备合并键

### 4.2 表映射（14 张 → 12 张，剔除 lzu_session）
| Rust 表 | PG 表 | 关键调整 |
|---|---|---|
| pomodoro_config | pomodoro_config | 单例 id=1；auto_start_next_phase BOOLEAN |
| pomodoro_sessions | pomodoro_sessions | started_at/ended_at TIMESTAMPTZ；task_id FK SET NULL |
| tasks | tasks | due_date DATE；tags JSONB；remind_at TIMESTAMPTZ（归一化）；reminder_time TIME；is_daily BOOLEAN |
| courses | courses | start/end_time TIME；semester_start_date DATE；week_pattern TEXT |
| exams | exams | exam_datetime/end TIMESTAMPTZ；semester TEXT；course_id FK SET NULL |
| sync_config | sync_config | 单例 id=1；last_sync_at TIMESTAMPTZ；etag/device/dataset TEXT |
| notification_config | notification_config | exam_offsets JSONB；enabled BOOLEAN |
| pomodoro_runtime_state | pomodoro_runtime_state | date_key DATE；last_seen_at TIMESTAMPTZ；active_session_id 软引用 |
| semester_context | semester_context | start_date DATE；UNIQUE(source, term_label) |
| term_phases | term_phases | start/end_week INT；pomodoro_profile TEXT（名称引用）；notification_rules JSONB；sync_id UNIQUE |
| pomodoro_profiles | pomodoro_profiles | name UNIQUE；时长 INT；is_builtin BOOLEAN |
| ai_config | ai_config | api_key_encrypted TEXT（AES-256-GCM 密文）；sync_enabled BOOLEAN |
| ai_morning_brief | ai_morning_brief | date DATE UNIQUE；markdown TEXT；source/model TEXT |
| lzu_session | **❌ 剔除** | — |
| _migrations | schema_migrations | golang-migrate 管理 |

### 4.3 索引/约束对齐
- 部分唯一索引（sync_id WHERE 非空非''）→ PG `CREATE UNIQUE INDEX ... ON t(sync_id) WHERE sync_id IS NOT NULL AND sync_id <> ''`
- 保留 CHECK：status/priority/phase_type/day_of_week 1-7/时长下限/end_week≥start_week
- 索引：semester_context(source, refreshed_at)、term_phases(term_label, sort_order)、ai_morning_brief(date)

## 5. API 契约（REST 映射）

> 对齐现有 command 语义；剔除全部 `lzu_*`。命名沿用前端 snake_case 字段（`src/types/*` 为准）。

### 认证
| 方法/路径 | 语义 | 对应 command |
|---|---|---|
| POST /api/auth/login | 登录，返回 JWT + 用户信息 | （新增） |
| POST /api/auth/logout | 登出（可选黑名单，单用户可简化） | （新增） |
| GET /api/auth/me | 当前用户 | lzu_get_auth_status 对应物 |

### 待办 tasks
| 方法/路径 | 语义 | 对应 command |
|---|---|---|
| GET /api/tasks?status=&priority=&sort= | 列表筛选排序 | get_all_tasks |
| POST /api/tasks | 创建 | create_task |
| PATCH /api/tasks/{id} | 更新 | update_task |
| DELETE /api/tasks/{id} | 软删 | delete_task |
| POST /api/tasks/{id}/complete | 完成 | complete_daily_task（含普通任务 done） |
| POST /api/tasks/{id}/uncomplete | 取消今日完成 | uncomplete_daily_task |

### 课程 courses
| GET /api/courses?semester= | 列表 | get_all_courses |
| POST /api/courses | 创建 | create_course |
| PATCH /api/courses/{id} | 更新 | update_course |
| DELETE /api/courses/{id} | 软删 | delete_course |
| POST /api/courses/import-text | 文本导入 | import_courses_from_text |
| POST /api/courses/reset-semester-dates | 批量重置起始日 | reset_all_semester_start_dates |

### 考试 exams
| GET /api/exams | 列表 | get_all_exams |
| POST /api/exams | 创建 | create_exam |
| PATCH /api/exams/{id} | 更新 | update_exam |
| DELETE /api/exams/{id} | 软删 | delete_exam |
| POST /api/exams/import-text | 文本导入 | import_exams_from_text |

### 学期与阶段 semester
| GET /api/semesters | 上下文列表 | get_semester_contexts |
| GET /api/term-phases?term_label= | 阶段列表 | get_term_phases |
| POST /api/term-phases | 创建 | create_term_phase |
| PATCH /api/term-phases/{id} | 更新 | update_term_phase |
| DELETE /api/term-phases/{id} | 软删 | delete_term_phase |
| GET /api/term-phases/current-status | 当前阶段状态 | get_current_phase_status |

### 番茄钟 pomodoro
| GET /api/pomodoro/state | 运行态 | get_pomodoro_state |
| POST /api/pomodoro/start | 开始 | start_pomodoro |
| POST /api/pomodoro/pause | 暂停 | pause_pomodoro |
| POST /api/pomodoro/reset | 重置 | reset_pomodoro |
| POST /api/pomodoro/interrupt | 中断处理 | resolve_pomodoro_interruption |
| GET /api/pomodoro/config | 配置 | get_pomodoro_config |
| PATCH /api/pomodoro/config | 更新配置 | update_pomodoro_config |
| GET /api/pomodoro/profiles | 配置档列表 | get_pomodoro_profiles |
| POST /api/pomodoro/profiles | 创建档 | create_pomodoro_profile |
| PATCH /api/pomodoro/profiles/{id} | 更新档 | update_pomodoro_profile |
| DELETE /api/pomodoro/profiles/{id} | 删档 | delete_pomodoro_profile |
| POST /api/pomodoro/finish-phase | 阶段结束落库（前端本地时钟到点上报） | （新增，替代 tick 事件） |

### 日历 calendar
| GET /api/calendar/week?semester=&week_index= | 周视图（课程+考试） | get_week_schedule |
| GET /api/calendar/day?date= | 日历日（课程+考试+待办） | get_calendar_week 对应日 |
| GET /api/briefing/today | 今日概览 | get_today_briefing |

### 同步 sync
| GET /api/sync/config | 配置 | get_sync_config |
| PATCH /api/sync/config | 更新 | update_sync_config |
| POST /api/sync/test | 连接测试 | test_sync_connection |
| POST /api/sync/now | 手动同步 | sync_now |
| GET/POST /api/sync/ai-recovery-key | 恢复密钥读写 | get/set_ai_sync_recovery_key |

### AI 晨报
| GET /api/ai/config | 配置 | get_ai_config |
| PATCH /api/ai/config | 更新 | update_ai_config |
| GET /api/ai/morning-brief?date= | 今日缓存 | get_ai_morning_brief |
| POST /api/ai/morning-brief/generate | 生成（SSE 流式） | generate_ai_morning_brief_streaming |
| POST /api/ai/morning-brief/generate?sync=true | 非流式生成 | generate_ai_morning_brief |

### 通知 notify
| GET /api/notify/config | 配置 | get_notification_config |
| PATCH /api/notify/config | 更新 | update_notification_config |

## 6. 认证设计

- 单账号：用户名 + bcrypt 密码哈希，来源为环境变量（`APP_USER`/`APP_PASSWORD_HASH`，密码首次生成哈希写入 `.env`）
- JWT：HS256，secret 环境变量（`JWT_SECRET`，>=32 字节），有效期（如 7 天）
- 中间件校验 `Authorization: Bearer`；`/api/auth/*` 与健康检查放行
- 前端：登录页存储 token（localStorage），HTTP client 统一注入；401 触发重登

## 7. 通知/调度设计（SMTP）

- 单进程内调度器：`app.Scheduler`，持有 DB store + `notify.Mailer`
- 触发模型：
  - **番茄钟**：前端到点调用 `POST /api/pomodoro/finish-phase` → 后端校验阶段时长 → 落库 → 异步 `go mailer.Send(...)`
  - **考试**：启动时 + 每次考试 CRUD 后重算最近偏移时刻，`time.AfterFunc` 注册，到点发邮件
  - **每日任务**：启动时注册 reminder_time；**一次性任务**：启动时注册最近 remind_at
  - **AI 7:00**：启动时计算到下一 7:00（+08:00）的时长注册 ticker，到点生成+发邮件
- 邮件内容：中文模板（阶段/考试/任务/AI 摘要 markdown 转文本）；发送失败 `slog.Error` + 单次重试
- 用 `go-mail`（或 net/smtp + 简单队列 channel），发送 goroutine 池（容量小，如 8），避免阻塞 HTTP

## 8. 实时/事件设计（替代 Tauri 事件）

| 原事件 | 替代方案 |
|---|---|
| pomodoro-tick（每秒） | 前端本地 `setInterval` 走 `PomodoroEngine` 等价前端状态机；阶段结束上报后端；页面挂载拉取 `GET /api/pomodoro/state` 校准 |
| sync-finished | `POST /api/sync/now` 同步完成后响应内返回结果；前端据此刷新（同步为请求-响应，无需事件） |
| ai-brief-generated | 页面挂载时 `GET /api/ai/morning-brief` 拉取；生成完成后响应返回结果并刷新 |
| lzu-auto-import | ❌ 剔除 |
| AI Channel 流式 | `POST /api/ai/morning-brief/generate` 响应 `text/event-stream`（SSE：`data: {"delta":"..."}`），前端 fetch ReadableStream 逐块渲染 |

## 9. 前端改造设计

- 新增 `src/lib/api/client.ts`：`fetch` 封装（base URL、JWT 注入、JSON、错误归一化为 `userErrorMessage` 兼容格式、401 处理）
- 新增 `src/lib/api/`：按模块封装（tasks/courses/exams/pomodoro/calendar/sync/ai/notify/semester），函数签名对齐原 invoke 参数/返回
- 组件替换：将 `invoke<T>("cmd", args)` 逐处替换为对应 api 函数；**不动组件结构/样式**
- 移除：`src/pages/lzu/`、`src/components/courses/import/LzuImportPanel.tsx`、`src/types/lzu.ts`、`src/hooks/use-android-back.ts`、`src/lib/tauri-events.ts`、AppShell/KairosHub 中 LZU 入口
- 剪贴板：`readText` 调用改 `navigator.clipboard.readText()`
- 环境：`VITE_API_BASE_URL`（dev 指向 `http://localhost:PORT`，prod 同源省略）
- 类型：保留 `src/types/*`（剔除 lzu.ts），微调字段（remind_at 等已由后端处理）

## 10. 部署拓扑（docker compose）

```
compose.yaml
  services:
    db:        postgres:16  (volume, healthcheck)
    api:       kairos-api (Go 镜像, 依赖 db, 环境变量注入 .env)
    web:       nginx:alpine (挂载 dist/ 与 TLS 证书, 反代 /api → api:8080)
  networks/volumes 常规
```

- 镜像构建：Go 多阶段 `golang:1.24-alpine` → `alpine`；前端 `node:22-alpine` 构建 → `nginx:alpine` 托管 + 反代
- TLS：挂载自签或 Let's Encrypt 证书（`nginx.conf` server 块），HTTP→HTTPS 重定向
- `.env`：DB URL、SMTP、JWT_SECRET、APP_USER、APP_PASSWORD_HASH、AI 默认
- 健康检查：`GET /healthz`（db ping + scheduler 状态）

## 11. WSL 开发环境

- WSL2 Ubuntu：安装 `go`（官方 tarball）、`docker`（Docker Desktop WSL2 backend 或 WSL docker）、`postgresql`（本地 dev 实例，`createdb kairos_dev`）
- 开发运行：`make dev` = 启动本地 PG → `go run ./cmd/api`（迁移自动跑）→ `npm run dev`（Vite 代理 `/api`）
- 代码位置：仓库 clone 在 WSL 内（`~/proj/Kairos`）或 Windows 侧经 WSL 访问（`/mnt/d/proj/Kairos`）——**建议 WSL 原生路径**避免文件系统性能问题
- 本地邮件调试：`MailHog`（docker run，SMTP 1025 + Web UI 8025）验证邮件触发

## 12. 兼容性与回滚

- **兼容**：前端与后端契约以 `src/types/*` 为基准，改动最小化；旧 SQLite 数据不迁移（从零开始），无双向兼容负担
- **回滚点**：每个 child 完成时保留可运行状态；整体回滚 = 保留现有 src-tauri 分支（git tag `pre-go-migration`），迁移完成后仍可切回旧版本
- **数据安全**：PostgreSQL volume 独立；AI key/密码均加密或环境变量，不入库明文

## 13. 依赖与顺序（children 关系）

| 波 | child | 依赖 |
|---|---|---|
| 1 | wsl-dev-env | 无（先行环境） |
| 2 | postgres-schema | wsl-dev-env |
| 3 | go-backend-auth（骨架+认证+健康检查） | postgres-schema |
| 4 | go-backend-core（tasks/courses/exams/term-phase） | go-backend-auth |
| 5 | go-backend-calendar + go-backend-pomodoro | go-backend-core |
| 6 | go-backend-sync + go-backend-ai | go-backend-core |
| 7 | go-backend-notifications（SMTP 调度） | go-backend-pomodoro/core/ai |
| 8 | frontend-api-layer | go-backend-core/calendar/pomodoro/sync/ai/notify（API 就绪后） |
| 9 | deployment | 全部后端 + frontend-api-layer |

> 通知 child 依赖多个后端模块（考试/番茄钟/任务/AI 触发点），排在后端主体之后；前端替换依赖全部 API 端点就绪。
