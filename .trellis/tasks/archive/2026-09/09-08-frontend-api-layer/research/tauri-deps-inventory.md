# Research: 前端 Tauri 非 invoke 依赖清单（W10 侦察）

> `src/` 中所有 `@tauri-apps/*` 使用（除 `invoke()`）。

## 导入 `@tauri-apps/*` 的文件（13）

| 文件 | 导入 | 类别 |
|---|---|---|
| `src/pages/today/AiBriefCard.tsx` | `Channel, invoke`（core） | Channel 流式 |
| `src/pages/today/TodayPage.tsx` | `invoke`（core） | — |
| `src/hooks/use-android-back.ts` | `onBackButtonPress`（app）、`PluginListener, invoke`（core） | Android 返回键 |
| `src/lib/tauri-events.ts` | `listen, EventCallback, EventName, UnlistenFn`（event） | 事件封装 |
| `src/components/settings/NotificationSettings.tsx` | `invoke`（core） | — |
| `src/components/settings/AiSettings.tsx` | `invoke`（core） | — |
| `src/components/settings/SemesterPhaseSettings.tsx` | `invoke`（core） | — |
| `src/components/exams/ExamList.tsx` | `invoke`（core）、`readText`（plugin-clipboard-manager） | 剪贴板 |
| `src/components/todo/TaskList.tsx` | `invoke`（core） | — |
| `src/components/pomodoro/PomodoroTimer.tsx` | `invoke`（core）+ `@/lib/tauri-events` | 事件 |
| `src/components/calendar/CalendarView.tsx` | `invoke`（core） | — |
| `src/components/courses/CourseSchedule.tsx` | `invoke`（core）、`readText`（plugin-clipboard-manager） | 剪贴板 |
| `src/components/sync/SyncSettings.tsx` | `invoke`（core）+ `@/lib/tauri-events` | 事件 |

## 事件订阅（唯一底层 `listen` 在 `tauri-events.ts`）

| 事件名 | 订阅点 | 替换方案 |
|---|---|---|
| `ai-brief-generated` | `AiBriefCard.tsx:276` | 挂载时 `getMorningBrief()` 拉取（无事件） |
| `pomodoro-tick` | `PomodoroTimer.tsx:91` | 前端本地 `setInterval` + 挂载拉取 `getState()` |
| `sync-finished` | `PomodoroTimer.tsx:116`、`SyncSettings.tsx:117` | `syncNow()` 响应内结果 + 完成后刷新 |

`tauri-events.ts` 消费者：`AiBriefCard.tsx`、`PomodoroTimer.tsx`、`SyncSettings.tsx`。

## Channel（AI 流式，唯一）

- `AiBriefCard.tsx:13,297,304`；`StreamChunk { delta: string }`（本地定义于第 24 行）
- 替换：`POST /api/ai/morning-brief/generate` SSE + fetch ReadableStream

## 剪贴板

- `ExamList.tsx:3,219`、`CourseSchedule.tsx:3,272`：`readText()` 优先 Tauri，catch 回退 `navigator.clipboard.readText()`
- 替换：直接删 Tauri 分支，保留 `navigator.clipboard.readText()`
- `AiSettings.tsx:221` 已是原生 `navigator.clipboard.writeText`（无需改）

## 通知插件

- `package.json` 声明 `@tauri-apps/plugin-notification`，`src/` **零直接引用**；唯一相关为 `invoke("request_notification_permission")`（已列入删除）

## Android 返回键

- `use-android-back.ts`（`onBackButtonPress` + `invoke("exit_app")`）；唯一调用方 `App.tsx:16,50`
- 替换：整体删除 hook + App 调用

## package.json 待移除依赖

- `@tauri-apps/api`、`@tauri-apps/plugin-clipboard-manager`、`@tauri-apps/plugin-notification`
