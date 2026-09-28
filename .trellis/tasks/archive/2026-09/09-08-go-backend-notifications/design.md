# W9 SMTP 邮件通知服务 - 技术设计

> 契约来源：parent `design.md §7`（通知/调度）+ Rust 通知子系统行为规格（本文件 §7 摘录）。
> 本 child 不得违反 parent 边界；Go 侧按 web 范式重建，**严禁机械搬运** Tauri 进程内模型。

## 1. 组件与目录

```
server/internal/domain/notify/
  ids.go          # FNV-1a 64bit stable_id（对齐 Rust notifications/ids.rs）
  mailer.go       # SMTP 发送器（异步 + 重试 + 中文模板）
  templates.go    # 各类型邮件文案（中文）
  scheduler.go    # 集中调度器（考试/每日/一次性/AI 7:00）
  notify_test.go  # 时间计算 + 模板 + fake SMTP 单测
server/internal/httpapi/dto/notification.go      # DTO
server/internal/httpapi/handlers/notifications.go # GET/PATCH /api/notify/config
```

## 2. API 契约（GET/PATCH /api/notify/config）

前端 `src/types/notification.ts` 为字段基准（parent design §5「命名沿用前端 snake_case」）。Go DB 列已为 `exam_offsets jsonb`，**handler DTO 保留前端字段名**：

```jsonc
// GET 响应 / PATCH 请求（三字段可选，merge 语义）
{
  "id": 1,
  "enabled": true,
  "exam_offsets_json": "[1440,60]",   // string：由 DB jsonb []byte 直接作为字符串输出/解析
  "android_channel_created": false,    // web 无此概念，恒 false（兼容 TS 类型，不入库）
  "created_at": "RFC3339",
  "updated_at": "RFC3339"
}
```

- **PATCH merge**：仅更新请求中出现的字段；`updated_at=now()`，`created_at` 不变（对齐 Rust `db/notifications.rs:56-82`）。
- **无校验**：`exam_offsets_json` 原样写入；非法 JSON 在调度时回退默认 `[1440,60]` 并告警（不报 500）。
- PATCH 后若 `enabled` 或 `exam_offsets_json` 存在 → **重算考试定时器**。

## 3. 配置（env）

| 变量 | 默认 | 说明 |
|---|---|---|
| `SMTP_HOST` | `""` | 空则禁用邮件发送（仅日志） |
| `SMTP_PORT` | `587` | |
| `SMTP_USER` / `SMTP_PASS` | `""` | 空则匿名（MailHog） |
| `SMTP_FROM` | `Kairos <no-reply@kairos.local>` | |
| `SMTP_TLS` | `starttls` | `starttls` / `implicit`(465) / `none` |

`internal/config.Config` 增加字段；`Load()` 读取；未配置不报错（warning）。收件人：单用户，使用 `SMTP_USER` 或新增 `NOTIFY_TO`（默认等于 `SMTP_FROM` 地址）。

## 4. Scheduler 设计（可测试）

- **纯函数 + 注入时钟**：所有时间计算抽为纯函数（`nextOccurrence`、`planNext`、`examFireTimes`、`offsetDescription`、`nextSevenAM`），`now func() time.Time` 注入，便于对齐 Rust 单测断言。
- **定时器抽象**：`type timer interface{ Stop() bool }`，生产用 `time.AfterFunc`，测试用可手动触发的 fake（或直接测纯函数，不测真实等待）。
- **状态与并发**：`Scheduler` 持有 `*store.Queries`、`Mailer`、`*ai.Generator`、互斥锁保护的注册表（考试 id 集合、任务定时器、AI 定时器）；`RecomputeExams/RecomputeTasks/RecomputeAI` 幂等、goroutine 安全；每次重算先取消旧的再注册。
- **启动**：`scheduler.Start(ctx)` 注册考试 + 任务 + AI；`ctx` 取消时停止所有定时器。
- **AI 7:00**：计算到下一个 +08:00 07:00 的时长 → `AfterFunc`；到点 `GenerateBrief(ctx, today, false, nil)` → 发邮件 → 重新注册次日。

### 4.1 触发器 → 重算钩子映射

| 触发点 | 钩子 |
|---|---|
| 启动 | `scheduler.Start`（考试/任务/AI 全量） |
| `PATCH /api/notify/config`（enabled/offsets 变） | `RecomputeExams` |
| 考试 create/update/delete/import-text | `RecomputeExams` |
| 学期阶段 create/update/delete | `RecomputeExams`（阶段门控变化） |
| 任务 create/update/delete/complete/uncomplete | `RecomputeTasks` |
| `PATCH /api/ai/config` | `RecomputeAI` |
| `POST /api/pomodoro/finish-phase` | `pomodoro.Notifier.PhaseEnded` → 邮件 |

## 5. 邮件文案（中文模板，对齐 Rust）

- 考试：标题=课程名；正文 `考试「{course_name}」将在 {desc} 后开始`；`desc`：`>=1440 且整除 1440 → N天`；`>=60 且整除 60 → N小时`；否则 `N分钟`。
- 每日任务：标题 `每日任务提醒`；正文 `「{title}」—— 别忘了今天完成`。
- 一次性任务：标题 `任务提醒`；正文 `「{title}」—— 到时间了，别忘了完成`。
- 番茄钟阶段结束（finish-phase）：标题 `番茄钟`；正文 `番茄钟「{专注|短休息|长休息}」阶段已结束`。
- AI 晨报：标题 `今日 AI 摘要已生成`；正文 markdown 前 60 字符（其余作为邮件正文整体附上）。

## 6. 门控与边界（对齐 Rust）

- 考试：`notifications config.enabled` AND `termphase` 当前 `ExamNotificationsEnabled`。**每日任务、番茄钟、AI 晨报不读 `config.enabled`**。
- 一次性任务过期阈值 **60 秒**（`now >= remind_at + 60s` 视为过期，不补发）；启动清理已过期 `remind_at`（对应 Rust `clear_expired_remind_at`）。
- 无提醒待处理时任务循环 **3600 秒** 后重试。
- 每日任务 `today_target == now` 视为今天（`>=`）；同刻多条全部触发。
- 时区：比赛/提醒按 +08:00 墙钟；考试 `exam_datetime` TIMESTAMPTZ（UTC 存储）。

## 7. 测试计划（行为对齐 Rust）

| 单测 | 对齐 Rust |
|---|---|
| `stable_id` 确定性/不同键/非负 | ids.rs 3 个测试 |
| `offsetDescription`：1440→1天, 2880→2天, 60→1小时, 120→2小时, 30→30分钟, 5→5分钟 | exam_scheduler.rs |
| `nextOccurrence` 同日/次日 | daily_reminder.rs |
| `planNext`：单条 11h、到点、同刻多条、空→3600、一次性未来 190800s、到点、过期跳过、混合 | daily_reminder.rs |
| 每日/一次性/番茄/AI 模板渲染 | 模板断言 |
| fake SMTP 服务器收信 roundtrip + 重试 | mailer |

**验证边界**：本地仅 `go build` + `gofumpt` + `go test -count=1`（store 测试用 `kairos-pg`）；curl 冒烟用 fake SMTP 或临时 MailHog 容器（**启动前请示用户，用后关闭**）。

## 8. 约束

- **不新增 Go 依赖**（用 stdlib `net/smtp`/`crypto/tls`）。
- **不改 schema、不新增 sqlc 查询**（本机无 sqlc）：复用 `ListExams`、`ListTasks`、`GetTask`、`UpdateTask`（清 `remind_at`）、`GetNotificationConfig`、`UpdateNotificationConfig`。
- 不移植系统通知/权限/Windows toast/Android channel/进程内 thread。
- 每波独立 commit；main 保持可运行。
