# SMTP 邮件通知服务（W9）

## Goal

实现 `internal/domain/notify`：**SMTP 邮件发送器**（异步、失败有限重试）+ **集中调度器**（考试偏移 / 每日 `reminder_time` / 一次性 `remind_at` / AI 7:00 +08:00 / 番茄钟 finish-phase），并新增 `GET/PATCH /api/notify/config` 配置端点与既有 handler 的重算钩子。

**架构差异（app→web）**：Rust `notifications/*` 是进程内 `thread::sleep` + `tauri_plugin_notification` 系统通知；Go 无系统通知、无常驻 WebSocket，改为**单进程调度器 goroutine + `net/smtp` 邮件**。番茄钟不再后台 tick（由前端 `finish-phase` 请求触发）；考试无设备常驻（改为服务端定时器 + 启动/CRUD 重算）。

## Requirements

- **G1 Mailer** `internal/domain/notify/mailer.go`：SMTP 发送（host/port/user/pass/from 经环境变量注入；支持 plain / STARTTLS / 465 隐式 TLS），中文模板，发送失败有限重试（单次），异步 goroutine 发送不阻塞 HTTP；未配置 SMTP 时降级为日志告警。
- **G2 Scheduler** `internal/domain/notify/scheduler.go`：集中调度四类触发 + 一个钩子：
  - **考试**：按 `notification_config.exam_offsets`（分钟数组）计算 `exam_datetime - offset`；受 `config.enabled` **AND** term phase `ExamNotificationsEnabled` 双重门控；启动全量重算 + 单场增删改重算。
  - **每日任务**：`is_daily=true && reminder_time`（+08:00）每天循环；无提醒时每小时重试。
  - **一次性任务**：`is_daily=false && remind_at` 一次；触发后清空 `remind_at`（去重核心）；过期（>1 分钟）不补发。
  - **AI 晨报**：每日 7:00（+08:00）调用 `ai.Generator.GenerateBrief`（缓存命中即复用）并发邮件；仅当 AI `enabled` 且有 key。
  - **番茄钟**：不注册定时器；实现 `pomodoro.Notifier`，`finish-phase` 时发送阶段结束邮件。
- **G3 Config** `internal/config`：新增 `SMTP_HOST/SMTP_PORT/SMTP_USER/SMTP_PASS/SMTP_FROM/SMTP_TLS` 环境变量加载（可选；未配置则调度器跳过邮件发送）。
- **G4 Handler** `internal/httpapi/handlers/notifications.go`：`GET/PATCH /api/notify/config`，DTO 对齐 `src/types/notification.ts`（`enabled`、`exam_offsets_json` string、`android_channel_created` 兼容恒 false）。
- **G5 重算钩子**：考试 CRUD（含文本导入）/ 学期阶段 CRUD 后重算考试定时器；任务 CRUD 后重算任务提醒；AI 配置变更后重算 7:00；通知配置 `enabled`/`exam_offsets` 变更后重算考试（`enabled=false` 取消已注册考试定时器）。
- **G6 装配** `cmd/api/main.go`：构造 mailer + scheduler → 注入 `httpapi.Options`（`PomodoroNotifier`）→ 启动调度器 → 随进程优雅关闭。

## Acceptance Criteria

- [ ] `go test ./internal/domain/notify/...` 绿（时间计算对齐 Rust 测试：offset 描述 / 下次出现 / `plan_next` / 过期；stable_id 确定性；模板渲染；fake SMTP 收信 roundtrip）
- [ ] `go test ./internal/...` 全绿
- [ ] curl 冒烟：登录 → `GET/PATCH /api/notify/config` → 触发一次 SMTP（fake SMTP 或 MailHog）→ 收件断言；番茄钟 finish-phase 钩子发信
- [ ] dev PG 容器仅验证期间临时启动，验证后已关闭

## Notes

- **移植锚点**：Rust `src-tauri/src/notifications/{ids,exam_scheduler,daily_reminder,pomodoro_scheduler}.rs` + `src-tauri/src/ai/scheduler.rs`（7:00）+ `src-tauri/src/db/notifications.rs` + `src-tauri/src/commands/notifications.rs`；提取其 `#[cfg(test)]` 场景作为 Go 单测行为规格。
- **明确不移植**：`notifications/system.rs`（系统通知/权限/Windows toast）、`android_channel_created`、进程内 thread 模型。
- **发送方**：Go 标准库 `net/smtp`（避免新增依赖；465 隐式 TLS 用 `tls.Dial`）。
- **通知 ID**：移植 FNV-1a 64bit `stable_id`（保持确定性，用于日志/去重）；一次性任务去重靠清空 `remind_at`。
- **无 schema 变更**；**不新增 sqlc 查询**（本机无 sqlc）——复用 `ListExams/ListTasks/GetTask/UpdateTask/GetNotificationConfig/UpdateNotificationConfig`。
- Rust `import_exams_from_text` 未触发重算的缺口，Go 侧在导入后一并重算（修正）。
- 配置契约分叉见 `design.md §2`（保留前端 `exam_offsets_json` 字段名）。
