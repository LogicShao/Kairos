# Research: Go 后端 API 契约（W10 前端对接用）

> 来源（已核对）：`server/internal/httpapi/router.go`、`dto/*.go`、`handlers/*.go`。
> 字段一律 snake_case；响应为**裸实体/数组**，无 `{data:...}` 包装。

## 全局约定

- **认证**：`Authorization: Bearer <JWT>`；`/healthz`、`/api/auth/login` 放行，其余 401
- **错误体**：`{ "error": "<message>" }` + 对应 HTTP 状态码（400/401/404/409/500/502）
- **成功体**：直接是实体/数组；删除等返回 `{ "status": "ok" }`
- **创建**：返回实体，状态 201
- **JSON null**：`GET /api/ai/morning-brief` 无记录时返回 `null`
- **SSE**：`Content-Type: text/event-stream`；事件 `data: {"delta":"..."}\n\n`；结束 `data: [DONE]\n\n`；错误 `data: {"error":"..."}\n\n`

## 端点清单

### auth
- `POST /api/auth/login` body `{username,password}` → `{token, user:{username}}`
- `POST /api/auth/logout` → `{status:"ok"}`
- `GET /api/auth/me` → `{username}`

### tasks
- `GET /api/tasks?status_filter=&priority_filter=&sort_by=&sort_order=` → `Task[]`
  - 默认 `sort_by=created_at`, `sort_order=DESC`
- `POST /api/tasks` body `CreateTaskRequest` → `Task`(201)
- `PATCH /api/tasks/{id}` body `UpdateTaskRequest` → `Task`
- `DELETE /api/tasks/{id}` → `{status}`
- `POST /api/tasks/{id}/complete` → `{status}`（仅每日任务）
- `POST /api/tasks/{id}/uncomplete` → `{status}`

### courses
- `GET /api/courses?semester=` → `Course[]`
- `POST /api/courses` body `CreateCourseRequest` → `Course`(201)
- `PATCH /api/courses/{id}` body `UpdateCourseRequest` → `Course`
- `DELETE /api/courses/{id}` → `{status}`
- `POST /api/courses/import-text` body `{text,semester,semester_start_date}` → `ImportTextResult`
- `POST /api/courses/reset-semester-dates` body `{date}` → `{updated:number}`

### exams
- `GET /api/exams` → `Exam[]`
- `POST /api/exams` body `CreateExamRequest` → `Exam`(201)
- `PATCH /api/exams/{id}` body `UpdateExamRequest` → `Exam`
- `DELETE /api/exams/{id}` → `{status}`
- `POST /api/exams/import-text` body `{text,semester}` → `ImportTextResult`

### semester / term-phases
- `GET /api/semesters` → `SemesterContext[]`
- `GET /api/term-phases?term_label=` → `TermPhase[]`
- `POST /api/term-phases` body `CreateTermPhaseRequest` → `TermPhase`(201)
- `PATCH /api/term-phases/{id}` body `UpdateTermPhaseRequest` → `TermPhase`
- `DELETE /api/term-phases/{id}` → `{status}`
- `GET /api/term-phases/current-status?source=` → `CurrentPhaseStatus`

### pomodoro
- `GET /api/pomodoro/state` → `PomodoroState`
- `POST /api/pomodoro/start|pause|reset` → `PomodoroState`
- `POST /api/pomodoro/interrupt` body `{action}` → `PomodoroState`
- `POST /api/pomodoro/finish-phase` body `{phase, completed_at?, task_id?}` → `PomodoroState`
- `GET /api/pomodoro/config` → `PomodoroConfig`
- `PATCH /api/pomodoro/config` body **扁平 `PomodoroConfig`** → `PomodoroState`
- `GET /api/pomodoro/profiles` → `PomodoroProfile[]`
- `POST /api/pomodoro/profiles` body `CreatePomodoroProfileRequest` → `PomodoroProfile`(201)
- `PATCH /api/pomodoro/profiles/{id}` body `UpdatePomodoroProfileRequest` → `PomodoroProfile`
- `DELETE /api/pomodoro/profiles/{id}` → `{status}`

> `PomodoroState` JSON（domain `service.go`）与前端 `PomodoroState` **完全一致**：
> `phase, remaining_seconds, total_seconds, is_running, completed_sessions, interrupted, interrupted_session_id, last_seen_at`

### calendar / briefing
- `GET /api/calendar/week?semester=&week_index=&semester_start_date=` → `WeekScheduleResponse`
- `GET /api/calendar/day?date=&semester=` → `CalendarWeekResponse`（前端未用）
- `GET /api/briefing/today` → `TodayBriefingResponse`

### sync / ai
- `GET /api/sync/config` → `SyncConfig`
- `PATCH /api/sync/config` body `{server_url,username,auto_sync,password?}` → `SyncConfig`
- `POST /api/sync/test` → `boolean`
- `POST /api/sync/now` → `SyncResult`
- `GET /api/ai/sync-recovery-key` → `{recovery_key: string|null}`
- `POST /api/ai/sync-recovery-key` body `{recovery_key}` → `{status}`
- `GET /api/ai/config` → `AiConfig`（`api_key_configured`，不回传明文 key）
- `PATCH /api/ai/config` body `UpdateAiConfigRequest`（`api_key` 省略=保留，空串=清空）→ `AiConfig`
- `GET /api/ai/morning-brief?date=` → `AiMorningBrief | null`
- `POST /api/ai/morning-brief/generate?force=&sync=` → SSE（默认） / JSON（`sync=true`）

### notify
- `GET /api/notify/config` → `NotificationConfig`
- `PATCH /api/notify/config` body `UpdateNotificationConfigRequest`（直接 body）→ `NotificationConfig`

## 关键坑（必须处理）

1. **body 解包**：前端 invoke 的 `{cmd}`/`{req}`/`{config}` 是 Tauri 参数名，HTTP 侧 body 是**解包后的直接结构**（见 `invoke-inventory.md`）
2. **`update_pomodoro_config`**：body 是**扁平** `PomodoroConfig`，不是 `{config}`
3. **`set_ai_sync_recovery_key`**：前端 `hexKey` → 后端字段 `recovery_key`
4. **`get_ai_sync_recovery_key`**：后端包成 `{recovery_key}`，wrapper 应解包为 `string|null`
5. **query 名**：tasks 用 `status_filter/priority_filter/sort_by/sort_order`；term-phases 用 `term_label`；current-status 用 `source`
6. **通知配置**：`exam_offsets_json` 必须是正整数 JSON 数组（后端校验）；`android_channel_created` 恒 false
7. **`request_notification_permission` 无端点** → 删除调用
8. **AI 恢复密钥双端点**：`/api/ai/sync-recovery-key` 与 `/api/sync/ai-recovery-key` 均存在；本任务用前者
