# W10 前端 API 层替换（invoke→HTTP）并移除 Tauri 依赖

> Parent: `09-08-go-backend-migration`。本 child 对应波次 **W10**（见 parent `implement.md`）。
> 依赖：W3–W9 后端 API 已就绪（已确认）。产出后 W11 部署方可开始。

## Goal

将现有 React 前端从 Tauri `invoke()` IPC 迁移到 Go 后端 REST API，使前端脱离 Tauri 运行时、在浏览器中可用：
- 新增统一 HTTP client（`src/lib/api/client.ts`）与按域拆分的 API 模块（`src/lib/api/*.ts`）
- 将 68 处 `invoke` 调用点（12 文件、51 个唯一命令）替换为对应 HTTP 调用
- 替换 Tauri 事件（`pomodoro-tick` / `sync-finished` / `ai-brief-generated`）为本地时钟 + 拉取 + SSE
- 新增登录页与鉴权门禁（后端为单账号 JWT）
- 移除全部 `@tauri-apps/*` 依赖、`tauri-events`、Android 返回键、系统通知权限 UI
- 配置 `VITE_API_BASE_URL`（dev 走 Vite 代理 / prod 同源）

## Requirements

### R1 统一 HTTP client 与鉴权
- R1.1 `src/lib/api/client.ts`：基于 `fetch` 封装 `apiFetch<T>(path, init)`，统一注入 `Authorization: Bearer <token>`、`Content-Type: application/json`、解析 JSON、错误归一化
- R1.2 Token 存储于 `localStorage`（键名固定，如 `kairos_token`）；请求携带；`401` 时清除 token 并跳转登录页
- R1.3 错误归一化：后端错误体为 `{ "error": "..." }`，client 必须抛出 `Error(message)`，使现有 `userErrorMessage()` 直接可用
- R1.4 base URL：`import.meta.env.VITE_API_BASE_URL`，缺省为空（同源，走 Vite `/api` 代理或 nginx 反代）

### R2 认证流程（新增）
- R2.1 新增登录页组件（用户名 + 密码），调用 `POST /api/auth/login`
- R2.2 登录成功存储 token 并进入主界面；提供登出（清除本地 token，调用 `POST /api/auth/logout`）
- R2.3 `App.tsx` 增加鉴权门禁：无 token 时渲染登录页；可选启动时 `GET /api/auth/me` 校验 token 有效性
- R2.4 登录 UI 遵循现有设计语言（AcrylicPanel / Button / 现有主色），不引入新依赖

### R3 按域 API 模块（对齐后端契约）
按 parent design §5 与后端实际路由，新增以下模块（函数签名保持语义化，字段保持 snake_case）：
- R3.1 `auth.ts`：`login` / `logout` / `me`
- R3.2 `tasks.ts`：`listTasks(filters)` / `createTask` / `updateTask` / `deleteTask` / `completeDailyTask` / `uncompleteDailyTask`
- R3.3 `courses.ts`：`listCourses` / `getWeekSchedule` / `createCourse` / `updateCourse` / `deleteCourse` / `resetSemesterDates` / `importCoursesFromText`
- R3.4 `exams.ts`：`listExams` / `createExam` / `updateExam` / `deleteExam` / `importExamsFromText`
- R3.5 `pomodoro.ts`：`getState` / `start` / `pause` / `reset` / `interrupt` / `getConfig` / `updateConfig` / `listProfiles` / `createProfile` / `updateProfile` / `deleteProfile` / `finishPhase`
- R3.6 `calendar.ts`：`getCalendarWeek(cmd)`；`briefing.ts`：`getTodayBriefing`
- R3.7 `sync.ts`：`getConfig` / `updateConfig` / `testConnection` / `syncNow`；`ai.ts`：`getConfig` / `updateConfig` / `getMorningBrief` / `generateMorningBriefStream` / `getRecoveryKey` / `setRecoveryKey`
- R3.8 `notify.ts`：`getConfig` / `updateConfig`
- R3.9 `semester.ts`：`listTermPhases` / `listSemesterContexts` / `getCurrentPhaseStatus` / `createTermPhase` / `updateTermPhase` / `deleteTermPhase`
- R3.10 **入参包装键解包**：原 invoke 的 `{ cmd }` / `{ req }` / `{ config }` / `{ hexKey }` 等包装键是 Tauri 命令参数名，HTTP 侧必须**解包为直接 body**；`update_pomodoro_config` 的 body 为**扁平 PomodoroConfig**；`set_ai_sync_recovery_key` 的 body 字段为 `recovery_key`（映射自 `hexKey`）

### R4 组件替换（68 处 invoke）
- R4.1 12 个数据触达文件全部切换为 API 模块调用，删除 `@tauri-apps/api/core` 的 `invoke` import
- R4.2 保持组件结构、样式、交互不变；仅替换数据访问层
- R4.3 保持既有错误处理与 loading 语义；错误信息经 `userErrorMessage` 呈现

### R5 事件替换
- R5.1 `pomodoro-tick`：改为**前端本地时钟**（`setInterval`）驱动显示；页面挂载时 `GET /api/pomodoro/state` 校准；阶段到点调用 `POST /api/pomodoro/finish-phase` 落库并据响应更新
- R5.2 `sync-finished`：`sync_now` 为请求-响应，结果直接在响应中返回；`SyncSettings` / `PomodoroTimer` 改为同步完成后刷新配置（去掉事件监听）
- R5.3 `ai-brief-generated`：`AiBriefCard` 改为挂载时 `GET /api/ai/morning-brief` 拉取（去掉事件监听）
- R5.4 AI 流式：`Channel` → **SSE**；`generateMorningBriefStream` 用 `fetch` + `ReadableStream` 读取 `text/event-stream`，逐块解析 `data: {"delta":"..."}`，`data: [DONE]` 结束，`data: {"error":"..."}` 报错（因需带 JWT header，不用原生 EventSource）

### R6 移除 Tauri
- R6.1 删除 `src/lib/tauri-events.ts` 及其 3 个消费者中的导入/调用
- R6.2 删除 `src/hooks/use-android-back.ts` 及 `App.tsx` 中的调用
- R6.3 移除 `NotificationSettings` 的「请求通知权限」UI 与 `request_notification_permission` 调用（PRD 已砍系统通知，后端无此端点）；保留通知开关与考试偏移配置
- R6.4 剪贴板：删除 `@tauri-apps/plugin-clipboard-manager` 的 `readText` 分支，保留现有 `navigator.clipboard.readText()` 回退（`ExamList` / `CourseSchedule`）
- R6.5 `package.json` 移除 `@tauri-apps/api`、`@tauri-apps/plugin-clipboard-manager`、`@tauri-apps/plugin-notification`；清理 `src-tauri/capabilities` 相关权限（若仍被引用）

### R7 环境与配置
- R7.1 确认 `vite.config.ts` `/api` 代理可用（已存在，target 默认 `http://localhost:8080`，支持 `VITE_API_PROXY_TARGET` 覆盖）
- R7.2 清理 `envPrefix` 中的 `TAURI_` 与 `server.host` 的 `TAURI_DEV_HOST`（若 R6 后无引用）
- R7.3 新增 `src/vite-env.d.ts` 声明 `VITE_API_BASE_URL`（若不存在）

## 非功能需求

- **不改变视觉**：本次仅替换数据层与移除平台依赖，不做组件/样式重写
- **类型安全**：`npx tsc --noEmit` 零错误；不使用 `as any` / `@ts-ignore`
- **无残留**：`grep -rn "@tauri-apps" src/` 零命中
- **可回退**：单文件级替换，保持每步可编译

## Scope OUT（Must NOT have）

- ❌ 组件重写 / 视觉调整 / 新增功能
- ❌ 修复后端 API（后端视为契约，只消费）
- ❌ 多用户/注册/刷新 token 体系
- ❌ 系统通知 / Web Push / WebSocket 事件总线
- ❌ `src-tauri/` 目录删除（属 W11，AC7）

## Acceptance Criteria

- [ ] AC1 `npx tsc --noEmit` 通过
- [ ] AC2 `npm run lint` 通过
- [ ] AC3 `grep -rn "@tauri-apps" src/` 与 `grep -rn "invoke(" src/` 均零命中
- [ ] AC4 51 个唯一命令全部有对应 API 函数；无遗漏（逐一对照 `implement.jsonl` 命令清单）
- [ ] AC5 登录页可用：正确凭据进入主界面，错误凭据提示错误；无 token 时被门禁拦截
- [ ] AC6 断网/401 时 `userErrorMessage` 呈现后端 `{error}` 文案
- [ ] AC7 浏览器中可用（配合本地 Go 后端 + dev PG）：登录 → 任务/课程/考试 CRUD → 日历查看 → 番茄钟开始/结束落库 → 手动同步
- [ ] AC7b（**可选，非阻塞**）AI 晨报 SSE 流式；需外部 AI Key，见下
- [ ] AC8 `package.json` 无任何 `@tauri-apps/*` 依赖

## 可选功能（AI 晨报）

> 用户决定：**AI 晨报按选用选项处理，不作为 W10 验收门禁**。

- AI 相关代码（`ai.ts`、`AiSettings`、`AiBriefCard`、SSE）**照常迁移**，否则会残留 `invoke`（违反 AC3/AC8）；这属于"必须迁移"，不是"必须可用"
- AI **可用性验证是可选的**：需要 DeepSeek（或兼容）API Key，此时再请用户提供
- AI 功能保持既有 `enabled` 开关；未配置 Key 时 UI 应正常降级（`api_key_configured=false`），不阻塞其他功能
- 其他外部依赖（远端服务器 / 域名 / TLS 证书 / SMTP 凭据）在推进到相应步骤时提醒用户准备

## Notes

- 后端契约来源：`server/internal/httpapi/router.go` + `server/internal/httpapi/dto/*.go` + 各 handler（已核对）
- 命令清单与调用点：见 `check.jsonl` / `implement.jsonl` 引用的 research；68 处 / 51 唯一命令
- 番茄钟后端 `State` 与前端 `PomodoroState` 字段完全一致，无需映射
- AI 恢复密钥存在双端点（`/api/ai/sync-recovery-key` 与 `/api/sync/ai-recovery-key`），本任务统一用 `/api/ai/sync-recovery-key`
