# Kairos 后端迁移：Rust/Tauri → Go + 纯 Web B/S

## Goal

将 Kairos 从 Tauri 桌面/移动应用（React 19 + Rust + SQLite，Windows/Android）迁移为**纯 Web B/S 架构**：
- 前端：保留现有 React 代码库，浏览器访问，由 Vite 构建产物静态托管
- 后端：Go 服务，REST API + 集中调度 + SMTP 邮件通知，部署到远程 Docker
- 数据：PostgreSQL（单用户个人部署），从零开始（不迁移现有 SQLite 数据）
- 开发：WSL（Linux）环境
- **彻底移除 Rust/Tauri 后端**（src-tauri 退役）

## 背景与约束

**现状（盘点结论）**
- Rust 后端 61 个 `#[tauri::command]`（12 个命令文件）+ 5 个后台线程（番茄钟 tick 每秒、考试通知、每日提醒、AI 7:00、自动同步 15min）
- SQLite 14 张业务表 + `_migrations`；2 条真实外键；软删除全局模式；`sync_id` 部分唯一索引（跨设备 LWW 合并键）
- 前端 14 个文件依赖 Tauri，59 个唯一 command；4 个后端事件（pomodoro-tick / sync-finished / ai-brief-generated / lzu-auto-import）；AI 流式用 `Channel`
- 7 种时间格式并存（UTC ISO8601、RFC3339、纯日期 +08:00、`remind_at` 空格分隔 +08:00、`HH:MM`、`HH:mm`、SQLite datetime）
- JSON 全部以 TEXT 存储且无校验；`''` 与 NULL 混用表示"无值"

**已确认决策（用户拍板）**
1. 纯 Web，彻底放弃 Tauri（含 Android/桌面安装包）
2. 通知：Go 后端集中调度 → **SMTP 邮件推送**（无系统通知、无 Web Push、无常驻 WebSocket 事件总线）
3. 数据库：**PostgreSQL**
4. 前端：保留现有 React，invoke → HTTP client，事件改轮询/SSE/本地时钟
5. 部署：**单用户个人部署**，docker compose + HTTPS + 单一账号 JWT
6. 数据：**从零开始**
7. **LZU 模块整体剔除**（登录/校园卡/服务目录/课表自动导入/EasyTong/AES-MD5 协议，及其前端页面；登录 bug 系外部接口变化，不作为迁移目标）
8. 工作流：Trellis（本任务为 parent，children 独立规划/实现/归档）

## Requirements

### R1 认证
- R1.1 单一账号登录（用户名 + bcrypt 密码，由环境变量/启动配置提供）
- R1.2 JWT 签发与校验（access token），nginx 终止 TLS
- R1.3 未认证请求返回 401；前端登录页

### R2 待办任务（对齐现有 tasks 语义）
- R2.1 任务 CRUD：title、description、status(todo/in_progress/done)、priority(high/medium/low)、due_date、tags(JSON)
- R2.2 筛选与排序（status/priority 过滤 + 白名单排序），对齐 `get_all_tasks`
- R2.3 每日任务：is_daily + last_completed_date + reminder_time；complete/uncomplete_daily_task
- R2.4 一次性提醒：remind_at（`YYYY-MM-DD HH:MM` +08:00）；到点触发 R8 邮件
- R2.5 软删除

### R3 课程
- R3.1 课程 CRUD：name、day_of_week(1-7)、start_time/end_time(HH:mm)、location、teacher、color、semester、week_pattern(如 "1-16"/"2-18双")、semester_start_date
- R3.2 按学期过滤；批量重置学期起始日
- R3.3 文本导入（解析 + 去重），对齐 `import_courses_from_text`
- R3.4 软删除

### R4 考试
- R4.1 考试 CRUD：course_name、exam_datetime(RFC3339)、exam_end_datetime、location、notes、course_id(逻辑关联 courses)、semester
- R4.2 文本导入（解析 + 去重），对齐 `import_exams_from_text`
- R4.3 到点/偏移触发邮件提醒（R8）
- R4.4 软删除

### R5 学期与阶段
- R5.1 学期上下文（term_label 唯一、start_date、current_week、total_weeks）
- R5.2 学期阶段：teaching/exam/break，start_week/end_week、affects_courses、affects_exam_notifications、pomodoro_profile(按名称引用)
- R5.3 当前阶段状态判定（按日期/周），对齐 `get_current_phase_status`
- R5.4 默认阶段初始化（幂等）

### R6 番茄钟
- R6.1 引擎：work/short_break/long_break 阶段、sessions_before_long_break、暂停/重置/中断处理
- R6.2 配置（pomodoro_config 单例）+ 配置档（pomodoro_profiles，内置 default/intense/relaxed 保护）
- R6.3 会话记录（started_at/ended_at/session_type/task_id）+ 按日完成轮数统计
- R6.4 **UI 时钟由前端本地 setInterval 驱动**；阶段结束由前端调后端落库，后端校验并触发邮件
- R6.5 运行态恢复：页面挂载时拉取后端状态（phase/remaining/is_running）继续显示

### R7 日历与周视图
- R7.1 周课表：课程 + 考试，按学期/周索引聚合
- R7.2 日历周：课程 + 考试 + 有截止日待办聚合（周/月视图）

### R8 邮件通知（SMTP）
- R8.1 SMTP 配置（host/port/user/pass/from）经环境变量
- R8.2 集中调度：Go 协程定时器，触发点：
  - 番茄钟阶段结束
  - 考试（按 notification_config.exam_offsets_json 偏移，如 [1440,60] 分钟）
  - 每日任务提醒（reminder_time）
  - 一次性任务 remind_at
  - AI 晨报每日 7:00（+08:00）
- R8.3 邮件文案含模块上下文；发送失败记录日志并重试有限次
- R8.4 通知配置（enabled + exam_offsets_json）CRUD

### R9 WebDAV 同步（核心保留）
- R9.1 快照导出/导入 v2 协议（LWW 合并、sync_id 匹配、墓碑传播）
- R9.2 WebDAV 客户端（Basic Auth、ETag 条件上传、超时）
- R9.3 手动同步端点；**取消常驻自动同步线程**（B/S 下改为页面打开时触发 + 手动）
- R9.4 AI 设置信封加密同步（AES-256-GCM + PBKDF2 + DEK 恢复密钥）

### R10 AI 晨报
- R10.1 配置（enabled/base_url/model/api_key 密文）+ 恢复密钥
- R10.2 生成：AI 提供商（DeepSeek API，SSE 流式）+ 规则降级
- R10.3 按日缓存；7:00 自动生成并邮件推送（R8）
- R10.4 今日概览聚合（课程/待办/考试/番茄钟/阶段）对齐 `get_today_briefing`

### R11 前端改造
- R11.1 统一 HTTP client 替换 `@tauri-apps/api/core` invoke；REST 端点对齐现有 command 语义与 `src/types/*`
- R11.2 事件替换：`pomodoro-tick` → 前端本地时钟 + 拉取校准；`sync-finished`/`ai-brief-generated` → 页面挂载拉取 + 操作后刷新
- R11.3 AI 流式：`Channel` → SSE（fetch ReadableStream / EventSource）
- R11.4 剪贴板读取改 `navigator.clipboard.readText`（已有兜底）
- R11.5 移除 LZU 全部前端（LzuServicesPage、LzuImportPanel、lzu.ts 类型、相关入口）
- R11.6 移除 Android 返回键/exit_app 逻辑
- R11.7 环境配置：API base URL（dev 代理 / prod 同源）

### R12 部署
- R12.1 docker compose：Go API + PostgreSQL + 前端静态（nginx）+ TLS 证书挂载
- R12.2 前端 Vite build 产物由 nginx 托管，SPA 路由回退
- R12.3 配置经环境变量/`.env`（DB 连接、SMTP、JWT secret、账号密码）

### R13 WSL 开发环境
- R13.1 WSL（Ubuntu）下 Go toolchain、PostgreSQL、Docker 可用
- R13.2 开发模式：Go 后端 + Vite 前端 + 本地 PostgreSQL，热重载
- R13.3 Makefile 迁移命令（build/test/lint/dev）适配 Go + 前端

## 非功能需求

- **安全**：HTTPS（nginx TLS）；JWT 密钥环境变量；密码 bcrypt；API key 加密存储（AES-256-GCM）
- **时区**：业务时间统一 +08:00（China）；DB 用 TIMESTAMPTZ；`remind_at` 归一化为 UTC 存储
- **质量**：Go 代码通过 `gofumpt` + `golangci-lint`；核心模块有 Go 单测（tests-after，行为对齐 Rust 现有测试）；前端 `tsc --noEmit` + eslint
- **性能**：日历/周视图聚合单次查询或并行查询；邮件发送不阻塞 API
- **可运维**：结构化日志；健康检查端点；docker compose 一键起停

## Scope OUT（Must NOT have）

- ❌ LZU 全部（登录/校园卡/服务目录/课表导入/EasyTong/AES-MD5）及其前端
- ❌ 系统通知（tauri-plugin-notification / Windows toast）——一律走 SMTP 邮件
- ❌ 常驻后台事件总线（WebSocket 广播）——用轮询/SSE/本地时钟
- ❌ Android/Tauri 壳、onBackButtonPress、exit_app、桌面安装包（NSIS/MSI/deb/rpm/APK）
- ❌ 多用户/多租户、注册体系
- ❌ SQLite 数据迁移脚本（从零开始）
- ❌ 现有 Rust 代码改造（LZU 登录 bug 不修）
- ❌ 前端组件/视觉重写（仅替换 API 层与移除 LZU）

## Acceptance Criteria（parent 全局验收）

- [ ] AC1 后端 Go 服务在 WSL 下 `go test ./...` 全绿；`golangci-lint run` 无 error
- [ ] AC2 前端 `tsc --noEmit` + `eslint .` 通过；无任何 `@tauri-apps` import 残留
- [ ] AC3 REST API 覆盖全部保留功能（R2-R10），Postman/curl 冒烟通过认证→CRUD→聚合
- [ ] AC4 邮件通知在开发环境（MailHog 或真实 SMTP）实测触发：番茄钟阶段结束、考试偏移、每日任务、remind_at、AI 7:00
- [ ] AC5 docker compose up 后浏览器访问（HTTPS）完成登录、任务/课程/考试 CRUD、日历查看、番茄钟开始/结束落库、手动同步、AI 晨报生成
- [ ] AC6 现有 React 页面全部可用（除已移除的 LZU），数据持久化到 PostgreSQL
- [ ] AC7 无 Rust 构建依赖：仓库中 src-tauri 不再参与构建；README/Makefile 更新
- [ ] AC8 WSL 开发环境可一键启动（make dev）本地跑通

## Notes

- Parent 任务：总需求/总体架构/分波执行计划在此三件套（prd/design/implement）
- Children 任务：各模块独立 prd/design/implement，按 implement.md 分波顺序推进
- 子任务验收不得与 parent 验收冲突；parent 负责最终集成验收（AC1-AC8）
