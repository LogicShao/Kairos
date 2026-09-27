# W10 前端 API 层替换 - 技术设计

> 契约来源（已逐一核对）：`server/internal/httpapi/router.go`、`server/internal/httpapi/dto/*.go`、`server/internal/httpapi/handlers/*.go`。
> 前端现状：68 处 `invoke`、12 文件、51 唯一命令；3 个事件；1 处 `Channel` 流式；2 处剪贴板；1 处 Android 返回键。

## 1. 目录与文件结构

```
src/lib/api/
  client.ts        # fetch 封装、token 注入、错误归一化、401 处理
  auth.ts          # login/logout/me
  tasks.ts
  courses.ts
  exams.ts
  pomodoro.ts
  calendar.ts
  briefing.ts
  sync.ts
  ai.ts
  notify.ts
  semester.ts
src/components/auth/
  LoginPage.tsx    # 新增登录页
src/lib/api/sse.ts # SSE 流式读取助手（AI 晨报）
```

## 2. HTTP client 设计（`src/lib/api/client.ts`）

```ts
const TOKEN_KEY = "kairos_token"
const BASE = import.meta.env.VITE_API_BASE_URL ?? ""

export function getToken(): string | null
export function setToken(t: string): void
export function clearToken(): void

export async function apiFetch<T>(path: string, init?: RequestInit): Promise<T> {
  const headers = new Headers(init?.headers)
  if (init?.body !== undefined && !headers.has("Content-Type")) {
    headers.set("Content-Type", "application/json")
  }
  const token = getToken()
  if (token) headers.set("Authorization", `Bearer ${token}`)

  const res = await fetch(`${BASE}${path}`, { ...init, headers })

  if (res.status === 401) {
    clearToken()
    // 触发登录门禁（见 §6：通过自定义事件或 store 通知 App）
    window.dispatchEvent(new Event("kairos:unauthorized"))
    throw new Error("登录已过期，请重新登录")
  }
  if (!res.ok) {
    // 后端错误体 { error: string }；非 JSON 或缺失时回退状态码文案
    const message = await readErrorMessage(res)
    throw new Error(message)
  }
  if (res.status === 204) return undefined as T
  // AI morning-brief 缺失时后端返回 JSON null，需容忍
  return (await res.json()) as T
}
```

约定：
- 抛出 `Error`（非字符串），使现有 `userErrorMessage(error, fallback)` 返回 `error.message`
- 空 body 请求（POST 无参数）不设 `Content-Type`
- `credentials` 不启用（token 走 header，不依赖 cookie）

## 3. 命令 → 端点映射（51 唯一命令，全量）

> `body` 一律为**解包后的直接请求体**；包装键（`cmd`/`req`/`config`）是 Tauri 参数名，仅用于理解原调用，HTTP 侧不再保留。

### 3.1 认证（新增，无对应 invoke）
| API 函数 | 方法/路径 | 请求 | 响应 |
|---|---|---|---|
| `login(username, password)` | POST `/api/auth/login` | `{username,password}` | `{token, user:{username}}` |
| `logout()` | POST `/api/auth/logout` | — | `{status:"ok"}` |
| `me()` | GET `/api/auth/me` | — | `{username}` |

### 3.2 tasks
| API 函数 | 方法/路径 | 请求 | 响应 | 原命令（包装键） |
|---|---|---|---|---|
| `listTasks(f)` | GET `/api/tasks?status_filter=&priority_filter=&sort_by=&sort_order=` | query | `Task[]` | `get_all_tasks`（`filters`） |
| `createTask(cmd)` | POST `/api/tasks` | `CreateTaskRequest` | `Task`(201) | `create_task`（`cmd`） |
| `updateTask(id, cmd)` | PATCH `/api/tasks/{id}` | `UpdateTaskRequest` | `Task` | `update_task`（`id`,`cmd`） |
| `deleteTask(id)` | DELETE `/api/tasks/{id}` | — | `{status}` | `delete_task` |
| `completeDailyTask(id)` | POST `/api/tasks/{id}/complete` | — | `{status}` | `complete_daily_task` |
| `uncompleteDailyTask(id)` | POST `/api/tasks/{id}/uncomplete` | — | `{status}` | `uncomplete_daily_task` |

> 注意：`update_task` 也被「快捷完成」复用（`{ status: "done" }`），语义不变。
> query 参数名以后端为准：`status_filter` / `priority_filter` / `sort_by` / `sort_order`（与前端 `TaskFilterParams` 同名）。

### 3.3 courses
| API 函数 | 方法/路径 | 请求 | 响应 | 原命令（包装键） |
|---|---|---|---|---|
| `listCourses(f)` | GET `/api/courses?semester=` | query | `Course[]` | `get_all_courses`（`filters`） |
| `getWeekSchedule(cmd)` | GET `/api/calendar/week?semester=&week_index=&semester_start_date=` | query | `WeekScheduleResponse` | `get_week_schedule`（`cmd`） |
| `createCourse(cmd)` | POST `/api/courses` | `CreateCourseRequest` | `Course`(201) | `create_course`（`cmd`） |
| `updateCourse(id, cmd)` | PATCH `/api/courses/{id}` | `UpdateCourseRequest` | `Course` | `update_course`（`id`,`cmd`） |
| `deleteCourse(id)` | DELETE `/api/courses/{id}` | — | `{status}` | `delete_course` |
| `resetSemesterDates(date)` | POST `/api/courses/reset-semester-dates` | `{date}` | `{updated:number}` | `reset_all_semester_start_dates`（`date`） |
| `importCoursesFromText(cmd)` | POST `/api/courses/import-text` | `{text,semester,semester_start_date}` | `ImportTextResult` | `import_courses_from_text`（`cmd`） |

### 3.4 exams
| API 函数 | 方法/路径 | 请求 | 响应 | 原命令（包装键） |
|---|---|---|---|---|
| `listExams()` | GET `/api/exams` | — | `Exam[]` | `get_all_exams` |
| `createExam(cmd)` | POST `/api/exams` | `CreateExamRequest` | `Exam`(201) | `create_exam`（`cmd`） |
| `updateExam(id, cmd)` | PATCH `/api/exams/{id}` | `UpdateExamRequest` | `Exam` | `update_exam`（`id`,`cmd`） |
| `deleteExam(id)` | DELETE `/api/exams/{id}` | — | `{status}` | `delete_exam` |
| `importExamsFromText(cmd)` | POST `/api/exams/import-text` | `{text,semester}` | `ImportTextResult` | `import_exams_from_text`（`cmd`） |

### 3.5 pomodoro
| API 函数 | 方法/路径 | 请求 | 响应 | 原命令（包装键） |
|---|---|---|---|---|
| `getState()` | GET `/api/pomodoro/state` | — | `PomodoroState` | `get_pomodoro_state` |
| `start()` | POST `/api/pomodoro/start` | — | `PomodoroState` | `start_pomodoro` |
| `pause()` | POST `/api/pomodoro/pause` | — | `PomodoroState` | `pause_pomodoro` |
| `reset()` | POST `/api/pomodoro/reset` | — | `PomodoroState` | `reset_pomodoro` |
| `interrupt(action)` | POST `/api/pomodoro/interrupt` | `{action}` | `PomodoroState` | `resolve_pomodoro_interruption`（`request`） |
| `getConfig()` | GET `/api/pomodoro/config` | — | `PomodoroConfig` | `get_pomodoro_config` |
| `updateConfig(config)` | PATCH `/api/pomodoro/config` | **扁平 `PomodoroConfig`** | `PomodoroState` | `update_pomodoro_config`（`config`） |
| `listProfiles()` | GET `/api/pomodoro/profiles` | — | `PomodoroProfile[]` | `get_pomodoro_profiles` |
| `createProfile(req)` | POST `/api/pomodoro/profiles` | `CreatePomodoroProfileRequest` | `PomodoroProfile`(201) | `create_pomodoro_profile`（`req`） |
| `updateProfile(id, req)` | PATCH `/api/pomodoro/profiles/{id}` | `UpdatePomodoroProfileRequest` | `PomodoroProfile` | `update_pomodoro_profile`（`id`,`req`） |
| `deleteProfile(id)` | DELETE `/api/pomodoro/profiles/{id}` | — | `{status}` | `delete_pomodoro_profile` |
| `finishPhase(payload)` | POST `/api/pomodoro/finish-phase` | `{phase, completed_at?, task_id?}` | `PomodoroState` | （新增，替代 `pomodoro-tick`） |

> 差异点：原 invoke 对 `start/pause/reset/update_config` 声明 void，后端实际返回 `PomodoroState`；wrapper 直接返回该状态，组件可据此同帧更新（消除一次额外 `get_state`）。
> `update_pomodoro_config` 的 body **不是** `{config: ...}`，而是 `{work_seconds, short_break_seconds, long_break_seconds, sessions_before_long_break, auto_start_next_phase}`。

### 3.6 calendar / briefing
| API 函数 | 方法/路径 | 请求 | 响应 | 原命令 |
|---|---|---|---|---|
| `getCalendarWeek(cmd)` | GET `/api/calendar/week?semester=&week_index=&semester_start_date=&week_start_date=` | query | `CalendarWeekResponse` | `get_calendar_week`（`cmd`） |
| `getTodayBriefing()` | GET `/api/briefing/today` | — | `TodayBriefingResponse` | `get_today_briefing` |

> 后端另有 `/api/calendar/day`，前端当前未使用，W10 不接入（无 `get_calendar_week` 之外的日历命令）。
> `week_start_date` 后端 Week handler 未读取（仅 day）；为兼容可一并带上，后端忽略。

### 3.7 sync / ai
| API 函数 | 方法/路径 | 请求 | 响应 | 原命令（包装键） |
|---|---|---|---|---|
| `getSyncConfig()` | GET `/api/sync/config` | — | `SyncConfig` | `get_sync_config` |
| `updateSyncConfig(config)` | PATCH `/api/sync/config` | `{server_url,username,auto_sync,password?}` | `SyncConfig` | `update_sync_config`（`config`） |
| `testSyncConnection()` | POST `/api/sync/test` | — | `boolean` | `test_sync_connection` |
| `syncNow()` | POST `/api/sync/now` | — | `SyncResult` | `sync_now` |
| `getAiRecoveryKey()` | GET `/api/ai/sync-recovery-key` | — | `{recovery_key: string\|null}` | `get_ai_sync_recovery_key` |
| `setAiRecoveryKey(hexKey)` | POST `/api/ai/sync-recovery-key` | **`{recovery_key: hexKey}`** | `{status}` | `set_ai_sync_recovery_key`（`hexKey`） |
| `getAiConfig()` | GET `/api/ai/config` | — | `AiConfig` | `get_ai_config` |
| `updateAiConfig(req)` | PATCH `/api/ai/config` | `UpdateAiConfigRequest` | `AiConfig` | `update_ai_config`（`req`） |
| `getMorningBrief(date?)` | GET `/api/ai/morning-brief?date=` | query | `AiMorningBrief \| null` | `get_ai_morning_brief` |
| `generateMorningBriefStream({force,onDelta})` | POST `/api/ai/morning-brief/generate?force=` | SSE | `AiMorningBrief` | `generate_ai_morning_brief_streaming`（`force`,`channel`） |

> `get_ai_sync_recovery_key` 原返回 `string|null`；后端实际包成 `{recovery_key}`，wrapper 需解包返回 `string|null`，避免改动组件。
> `set_ai_sync_recovery_key` 前端 `hexKey` → 后端字段 `recovery_key`。
> `update_sync_config` 的 body 字段名 `server_url/username/auto_sync/password`；前端 `UpdateSyncConfigRequest` 同名（`password?`）。

### 3.8 notify / semester
| API 函数 | 方法/路径 | 请求 | 响应 | 原命令（包装键） |
|---|---|---|---|---|
| `getNotifyConfig()` | GET `/api/notify/config` | — | `NotificationConfig` | `get_notification_config` |
| `updateNotifyConfig(req)` | PATCH `/api/notify/config` | `UpdateNotificationConfigRequest`（**直接 body，无 `req` 包装**） | `NotificationConfig` | `update_notification_config`（`req`） |
| `listSemesterContexts()` | GET `/api/semesters` | — | `SemesterContext[]` | `get_semester_contexts` |
| `listTermPhases(termLabel)` | GET `/api/term-phases?term_label=` | query | `TermPhase[]` | `get_term_phases`（`termLabel`） |
| `getCurrentPhaseStatus(source)` | GET `/api/term-phases/current-status?source=` | query | `CurrentPhaseStatus` | `get_current_phase_status`（`source`） |
| `createTermPhase(req)` | POST `/api/term-phases` | `CreateTermPhaseRequest` | `TermPhase`(201) | `create_term_phase`（`req`） |
| `updateTermPhase(id, req)` | PATCH `/api/term-phases/{id}` | `UpdateTermPhaseRequest` | `TermPhase` | `update_term_phase`（`id`,`req`） |
| `deleteTermPhase(id)` | DELETE `/api/term-phases/{id}` | — | `{status}` | `delete_term_phase` |

> 已移除：`request_notification_permission`（后端无端点）、`exit_app`（Android 返回键移除）。

### 3.9 无对应端点的命令（必须删除调用）
| 原命令 | 处理 |
|---|---|
| `request_notification_permission` | 删除调用与「请求通知权限」UI（R6.3） |
| `exit_app` | 删除 `use-android-back.ts` 及 App 调用（R6.2） |

## 4. 事件替换设计

| 原事件 | 现状订阅点 | 替换方案 |
|---|---|---|
| `pomodoro-tick` | `PomodoroTimer.tsx:91` | 前端本地 `setInterval(1000)` 递减 `remaining`；挂载 `GET /api/pomodoro/state` 校准；到 0 调 `finishPhase` 并据响应进入下一阶段 |
| `sync-finished` | `PomodoroTimer.tsx:116`、`SyncSettings.tsx:117` | 无事件；`syncNow()` 返回 `SyncResult`，完成后本地刷新 `getSyncConfig()` / `getState()` |
| `ai-brief-generated` | `AiBriefCard.tsx:276` | 无事件；挂载时 `getMorningBrief()` 拉取；生成流程走 SSE，结束即刷新 |

番茄钟本地时钟要点（对齐 parent design §8）：
- `PomodoroState.remaining_seconds` 为后端剩余秒；前端以本地 `setInterval` 递减展示
- 阶段到点：调用 `finishPhase({ phase, completed_at: new Date().toISOString() })`
- 页面挂载 / 窗口聚焦时重新 `getState()` 校准；`is_running=false` 时不递减
- `auto_start_next_phase` 语义保持：由 `finishPhase` 响应决定下一阶段状态

## 5. SSE 流式设计（AI 晨报）

```ts
export async function generateMorningBriefStream(opts: {
  force?: boolean
  onDelta: (delta: string) => void
  signal?: AbortSignal
}): Promise<void> {
  const qs = opts.force ? "?force=true" : ""
  const res = await fetch(`${BASE}/api/ai/morning-brief/generate${qs}`, {
    method: "POST",
    headers: { Authorization: `Bearer ${getToken() ?? ""}` },
    signal: opts.signal,
  })
  if (!res.ok) throw new Error(await readErrorMessage(res))
  const reader = res.body!.getReader()
  const decoder = new TextDecoder()
  let buf = ""
  for (;;) {
    const { done, value } = await reader.read()
    if (done) break
    buf += decoder.decode(value, { stream: true })
    // 按 "\n\n" 切分事件；解析 data: 行
    //   data: {"delta":"..."}  -> onDelta
    //   data: {"error":"..."}  -> throw
    //   data: [DONE]           -> 结束
  }
}
```

`AiBriefCard` 改造：删除 `Channel`，`handleGenerate` 调 `generateMorningBriefStream({ force: true, onDelta: setStreamingText(prev => prev + d) })`；结束后 `getMorningBrief()` 拉取最终记录。

## 6. 认证与门禁设计

- `App.tsx`：顶层 `authed` 状态（初始 `getToken() !== null`）
- 无 token → 渲染 `<LoginPage onSuccess={() => setAuthed(true)} />`
- 监听 `window` 的 `"kairos:unauthorized"` 事件 → `setAuthed(false)`（client 在 401 时派发）
- `LoginPage`：表单 → `login()` → `setToken(res.token)` → `onSuccess()`
- 登出入口：置于 `KairosHub`（现有设置入口区），调用 `logout()` + `clearToken()` + 回登录页

## 7. 逐文件改造清单（12 个 invoke 文件 + 横切）

| 文件 | 涉及的 API 模块 | 特殊处理 |
|---|---|---|
| `src/lib/tauri-events.ts` | — | **删除**（消费者见下） |
| `src/hooks/use-android-back.ts` | — | **删除**；`App.tsx` 移除 import 与调用 |
| `src/pages/today/TodayPage.tsx` | `briefing` | 1 处 |
| `src/pages/today/AiBriefCard.tsx` | `ai` | 删 `Channel`/事件；SSE；共 3 处 |
| `src/components/todo/TaskList.tsx` | `tasks` | 8 处；完成/取消每日任务 |
| `src/components/courses/CourseSchedule.tsx` | `courses` | 7 处；剪贴板去 Tauri 分支 |
| `src/components/exams/ExamList.tsx` | `exams` | 6 处；剪贴板去 Tauri 分支 |
| `src/components/calendar/CalendarView.tsx` | `calendar` | 2 处（周/月视图） |
| `src/components/pomodoro/PomodoroTimer.tsx` | `pomodoro` | 8 处；本地时钟 + `finishPhase`；删 2 个事件监听 |
| `src/components/sync/SyncSettings.tsx` | `sync` | 8 处；删 `sync-finished` 监听，改响应刷新 |
| `src/components/settings/AiSettings.tsx` | `ai`,`sync` | 7 处；recovery key 解包 |
| `src/components/settings/NotificationSettings.tsx` | `notify` | 5 处→4 处（删权限）；3 处 update 去 `req` 包装 |
| `src/components/settings/SemesterPhaseSettings.tsx` | `semester` | 12 处；`termLabel`/`source`→query |
| `src/App.tsx` | `auth` | 新增门禁 + 删 `useAndroidBack` |
| `src/components/auth/LoginPage.tsx` | `auth` | **新增** |

## 8. 环境配置

- `vite.config.ts`：已有 `/api` → `VITE_API_PROXY_TARGET ?? http://localhost:8080`；确认保留
- 清理 `envPrefix` 的 `TAURI_`、`server.host` 的 `TAURI_DEV_HOST`（`build` 的 `TAURI_DEBUG` 一并清理）
- `VITE_API_BASE_URL`：dev 留空走代理；prod 同源留空（nginx 反代）

## 9. 兼容性与回滚

- 前端数据类型不新增（`src/types/*` 保持），仅在 `api/*.ts` 内做请求/响应形状适配
- 每替换一个组件即保持可编译；单文件可回退
- 不改后端；后端为唯一事实契约
- 风险点：入参包装键解包、SSE 解析、番茄钟本地时钟与后端状态一致性——优先小步验证

## 10. 验证

- 静态：`npx tsc --noEmit`、`npm run lint`、`grep -rn "@tauri-apps" src/`（零）、`grep -rn "invoke(" src/`（零）
- 运行时（需本地 Go + dev PG）：登录 → CRUD → 日历 → 番茄钟开始/结束 → 同步
- **可选**：AI SSE 流式验证需外部 AI Key（非阻塞；未提供时跳过，仅保证代码路径被 tsc 覆盖）
- 命令覆盖：对照 §3 的 51 条逐一确认有 wrapper
- 外部依赖提醒点：**AI Key**（验证 AI 时）、**远端服务器/TLS/SMTP**（W11 时）
