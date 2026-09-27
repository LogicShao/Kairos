# Research: 前端 invoke 调用清单（W10 侦察）

> 来源：对 `src/` 全量扫描（`invoke(` / `invoke<`）。**68 处调用，12 文件，51 个唯一命令**。
> 所有调用统一 `import { invoke } from "@tauri-apps/api/core"`；无别名、无动态命令名、无 `__TAURI_INTERNALS__`。

## 按文件（调用数）

| 文件 | 调用数 | 命令（按出现） |
|---|---|---|
| `src/pages/today/AiBriefCard.tsx` | 3 | get_ai_config, get_ai_morning_brief, generate_ai_morning_brief_streaming |
| `src/pages/today/TodayPage.tsx` | 1 | get_today_briefing |
| `src/hooks/use-android-back.ts` | 1 | exit_app |
| `src/components/settings/NotificationSettings.tsx` | 5 | get_notification_config, update_notification_config ×3, request_notification_permission |
| `src/components/settings/AiSettings.tsx` | 7 | get_ai_config ×2, get_sync_config, update_ai_config ×2, get_ai_sync_recovery_key, set_ai_sync_recovery_key |
| `src/components/settings/SemesterPhaseSettings.tsx` | 12 | get_term_phases, get_semester_contexts, get_pomodoro_profiles ×3, get_current_phase_status, create_term_phase, update_term_phase, delete_term_phase, create_pomodoro_profile, update_pomodoro_profile, delete_pomodoro_profile |
| `src/components/exams/ExamList.tsx` | 6 | get_all_exams ×2, update_exam, create_exam, delete_exam, import_exams_from_text |
| `src/components/todo/TaskList.tsx` | 8 | get_all_tasks ×2, create_task, update_task ×2, delete_task, complete_daily_task, uncomplete_daily_task |
| `src/components/pomodoro/PomodoroTimer.tsx` | 8 | get_pomodoro_state ×2, pause_pomodoro, start_pomodoro, reset_pomodoro, get_pomodoro_config, update_pomodoro_config, resolve_pomodoro_interruption |
| `src/components/calendar/CalendarView.tsx` | 2 | get_calendar_week ×2（周/月） |
| `src/components/courses/CourseSchedule.tsx` | 7 | get_all_courses, get_week_schedule, update_course, create_course, delete_course, import_courses_from_text, reset_all_semester_start_dates |
| `src/components/sync/SyncSettings.tsx` | 8 | get_sync_config ×4, update_sync_config ×2, test_sync_connection, sync_now |

**合计：3+1+1+5+7+12+6+8+8+2+7+8 = 68**

## 51 个唯一命令（去重）

```
complete_daily_task  create_course  create_exam  create_pomodoro_profile  create_task  create_term_phase
delete_course  delete_exam  delete_pomodoro_profile  delete_task  delete_term_phase
exit_app  generate_ai_morning_brief_streaming
get_ai_config  get_ai_morning_brief  get_ai_sync_recovery_key  get_all_courses  get_all_exams  get_all_tasks
get_calendar_week  get_current_phase_status  get_notification_config  get_pomodoro_config  get_pomodoro_profiles
get_pomodoro_state  get_semester_contexts  get_sync_config  get_term_phases  get_today_briefing  get_week_schedule
import_courses_from_text  import_exams_from_text
pause_pomodoro  request_notification_permission  reset_all_semester_start_dates  reset_pomodoro
resolve_pomodoro_interruption  set_ai_sync_recovery_key
start_pomodoro  sync_now  test_sync_connection  uncomplete_daily_task
update_ai_config  update_course  update_exam  update_notification_config  update_pomodoro_config
update_pomodoro_profile  update_sync_config  update_task  update_term_phase
```

## 入参包装键（HTTP 侧需解包）

| 包装键 | 命令 |
|---|---|
| `cmd` | create_task, update_task, create_course, update_course, create_exam, update_exam, import_courses_from_text, import_exams_from_text, get_week_schedule, get_calendar_week |
| `req` | update_notification_config, update_ai_config, create_term_phase, update_term_phase, create_pomodoro_profile, update_pomodoro_profile |
| `config` | update_sync_config, update_pomodoro_config（后者实际是扁平 PomodoroConfig） |
| `hexKey` | set_ai_sync_recovery_key（→ 后端字段 `recovery_key`） |
| 其他 | `filters`（get_all_tasks/get_all_courses）、`termLabel`（get_term_phases）、`source`（get_current_phase_status）、`date`（reset_all_semester_start_dates）、`id`、`request`（resolve_pomodoro_interruption）、`config`（update_sync_config） |

## 需删除的命令（后端无端点 / 功能移除）

- `request_notification_permission`：后端无端点，PRD 已砍系统通知 → 删调用与权限 UI
- `exit_app`：Android 返回键移除 → 删 hook 与 App 调用
