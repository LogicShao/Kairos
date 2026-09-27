# W10 前端 API 层替换 - 执行计划

> 复杂任务，需 `prd.md` + `design.md` + 本文件评审后 `task.py start` 再实现。
> 契约与逐命令映射见 `design.md` §3；侦察清单见 `research/`。

## 执行原则

1. **先地基后组件**：先建 `client.ts` + API 模块，再逐组件替换（避免边写边改地基）
2. **单一事实源**：后端路由/DTO 为契约，前端不改后端；类型保持 `src/types/*`
3. **小步可编译**：每完成一个工作包即跑 `tsc --noEmit`，保持可回退
4. **零残留**：结束时 `grep @tauri-apps` 与 `grep invoke(` 必须零命中
5. **AI 晨报可选**：不阻塞验收；外部 AI Key 在验证时再请用户提供

## 工作包（WP）

### WP1 地基：HTTP client + 鉴权 + 登录页 `[阻塞后续]`
1. `src/lib/api/client.ts`：`apiFetch`、token 读写（`kairos_token`）、`Authorization` 注入、错误归一化（`{error}` → `Error`）、401 → 派发 `kairos:unauthorized` 并抛错
2. `src/lib/api/auth.ts`：`login` / `logout` / `me`
3. `src/components/auth/LoginPage.tsx`：用户名+密码表单，调 `login()`，成功后 `setToken()` 并回调
4. `src/App.tsx`：`authed` 门禁（无 token → LoginPage；监听 `kairos:unauthorized`）
5. `src/components/kairos/KairosHub.tsx`：新增登出入口（调用 `logout` + `clearToken`）
- 验证：`npx tsc --noEmit`；浏览器手测（若本地后端就绪）：错误凭据报错、正确凭据进入主界面、登出回登录页

### WP2 API 模块（全部域）`[依赖 WP1]`
1. `tasks.ts` / `courses.ts` / `exams.ts` / `pomodoro.ts` / `calendar.ts` / `briefing.ts` / `sync.ts` / `ai.ts` / `notify.ts` / `semester.ts`
2. 严格按 `design.md` §3 映射：方法、路径、query 名、body 解包（`cmd`/`req`/`config` → 直接 body）、`setAiRecoveryKey(hexKey)` → `{recovery_key}`
3. `src/lib/api/sse.ts`：`generateMorningBriefStream`（fetch + ReadableStream，按 `\n\n` 切事件，处理 `delta`/`error`/`[DONE]`）
- 验证：`npx tsc --noEmit`；逐条核对 51 个唯一命令均有 wrapper（对照 `research/invoke-inventory.md`）

### WP3 组件替换（可并行，按域拆分）
每个文件：删除 `import { invoke }`，改调对应 API 函数，保持结构与样式不变。

- **WP3a tasks/courses/exams**：`TaskList`(8)、`CourseSchedule`(7，含剪贴板去 Tauri)、`ExamList`(6，含剪贴板去 Tauri)
- **WP3b calendar/briefing/semester**：`CalendarView`(2)、`TodayPage`(1)、`SemesterPhaseSettings`(12)
- **WP3c pomodoro**（复杂）：`PomodoroTimer`(8) 本地时钟 + `finishPhase` + 删 `pomodoro-tick`/`sync-finished` 监听
- **WP3d sync/ai/notify**：`SyncSettings`(8，删 `sync-finished` 监听改响应刷新)、`AiSettings`(7)、`AiBriefCard`(3，删 `Channel`/事件改 SSE)、`NotificationSettings`(删权限 UI，5→4)

- 验证（每包）：`npx tsc --noEmit`；该域 grep 无 `invoke(`
- Review gate：完成 WP3c/WP3d 后做一次浏览器运行时冒烟（pomodoro/sync/ai 属高风险）

### WP4 平台依赖清理 `[依赖 WP3]`
1. 删除 `src/lib/tauri-events.ts`
2. 删除 `src/hooks/use-android-back.ts`（App.tsx 引用已在 WP1 移除）
3. `package.json` 移除 `@tauri-apps/api`、`@tauri-apps/plugin-clipboard-manager`、`@tauri-apps/plugin-notification`
4. 清理 `src-tauri/capabilities` 引用（若 src 已无引用可留待 W11 删目录）
- 验证：`grep -rn "@tauri-apps" src/` 零命中；`npm install` 后 `npx tsc --noEmit` 通过

### WP5 环境配置 `[依赖 WP4]`
1. `vite.config.ts`：清理 `envPrefix` 的 `TAURI_`、`server.host` 的 `TAURI_DEV_HOST`、`build` 的 `TAURI_DEBUG`；保留 `/api` 代理
2. `src/vite-env.d.ts`：声明 `VITE_API_BASE_URL`（若缺失）
- 验证：`npm run dev` 能启动；`/api` 代理指向 Go 后端

### WP6 验证与收尾
1. `npx tsc --noEmit` → 0 错误
2. `npm run lint` → 通过
3. `grep -rn "@tauri-apps" src/` → 0；`grep -rn "invoke(" src/` → 0
4. 命令覆盖：51 条唯一命令逐条确认
5. 浏览器运行时冒烟（需本地 Go + dev PG，用后即关）：登录 → 任务/课程/考试 CRUD → 日历 → 番茄钟开始/结束落库 → 同步
6. （可选）AI SSE：用户提供 AI Key 时验证；否则跳过，仅保证 tsc 覆盖
7. Spec 更新（Phase 3.3）：把"前端 fetch client + SSE + 本地时钟"模式写入 `.trellis/spec/frontend/`
8. 提交（Phase 3.4）：按工作包分批 commit

## 依赖矩阵

| WP | 依赖 | 说明 |
|---|---|---|
| WP1 | — | 地基，阻塞全部 |
| WP2 | WP1 | 需 client |
| WP3a/b/c/d | WP2 | 需对应模块 |
| WP4 | WP3 | 消费者全替换后才能删 tauri-events |
| WP5 | WP4 | 无 TAURI_ 引用后清理 |
| WP6 | WP1–WP5 | 全量验证 |

## 验证命令汇总

```bash
npx tsc --noEmit
npm run lint
grep -rn "@tauri-apps" src/        # 期望 0
grep -rn "invoke(" src/            # 期望 0
grep -rn "request_notification_permission\|exit_app" src/  # 期望 0
```

## 回滚点

- 每个 WP 完成 = 可编译状态，可单独提交/回退
- 地基（WP1/WP2）为纯新增文件，回退无副作用
- 组件替换按文件粒度回退

## 完成条件（对应 prd AC1–AC8）

- AC1 `npx tsc --noEmit` 通过
- AC2 `npm run lint` 通过
- AC3 `@tauri-apps` / `invoke(` 零残留
- AC4 51 命令全覆盖
- AC5 登录门禁可用
- AC6 错误归一化生效（`userErrorMessage` 呈现后端 `{error}`）
- AC7 浏览器主流程冒烟（外部依赖就绪时；AI 为可选 AC7b）
- AC8 `package.json` 无 `@tauri-apps/*`
