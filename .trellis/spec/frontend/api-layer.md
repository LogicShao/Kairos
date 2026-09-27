# Frontend API Layer

> 前端数据访问规范。Kairos 前端为纯 Web 应用，数据来自 Go 后端的 REST API（`server/`），
> 经 `src/lib/api/` 统一封装；**组件内不得直接 `fetch`，不得直接拼接后端 URL**。

---

## 目录结构

```
src/lib/api/
  client.ts     # fetch 封装：base URL、JWT 注入、错误归一化、401 处理
  sse.ts        # SSE 流式读取助手（AI 晨报）
  <domain>.ts   # 按域拆分：auth/tasks/courses/exams/pomodoro/calendar/briefing/sync/ai/notify/semester
```

## 约定

1. **字段一律 snake_case**，与 `src/types/*` 及后端 DTO 同名；wrapper **不做 camelCase 转换**。
2. **每个函数显式返回类型**，类型从 `@/types/*` 引入（`import type`）。
3. **请求体解包**：Tauri 时代的 `{cmd}`/`{req}`/`{config}` 是 command 参数名；HTTP body 是解包后的直接结构。
   例：`updatePomodoroConfig(config)` 发送**扁平** `PomodoroConfig`；`setAiRecoveryKey(hexKey)` 发送 `{recovery_key}`。
4. **响应形状适配在 wrapper 内完成**，不泄漏给组件。例：后端 `{recovery_key}` 由 `getAiRecoveryKey()` 解包为 `string|null`。
5. **query 参数名以后端为准**（如 tasks 用 `status_filter`/`priority_filter`/`sort_by`/`sort_order`）。
6. **错误处理**：`client.ts` 读取后端 `{ "error": string }` 并抛出 `Error(message)`，使组件侧 `userErrorMessage()` 直接可用。**不要抛字符串**。

## 认证

- 单账号 JWT；token 存 `localStorage`（键 `kairos_token`），由 `client.ts` 统一注入 `Authorization: Bearer`。
- `401` → 清除 token 并派发 `window` 事件 `kairos:unauthorized`；`App.tsx` 据此回到登录页。
- 登录页 `src/components/auth/LoginPage.tsx`；登出入口在 `KairosHub`。

## 事件 → 拉取 / 本地时钟（替代 Tauri event）

后端无常驻事件总线。原 Tauri 事件改为：

| 原事件 | 替代 |
|---|---|
| `pomodoro-tick` | 前端本地 `setInterval` 时钟 + 挂载/聚焦时 `getState()` 校准；阶段到点 `finishPhase()` 上报 |
| `sync-finished` | `syncNow()` 请求-响应，完成后本地刷新配置 |
| `ai-brief-generated` | 挂载时 `getMorningBrief()` 拉取 |

## SSE 流式

AI 晨报通过 `src/lib/api/sse.ts` 的 `generateMorningBriefStream({ force, onDelta, signal })`：
`fetch` + `ReadableStream` 读取 `text/event-stream`；解析 `data: {"delta":...}` / `data: {"error":...}` / `data: [DONE]`。
因需携带 JWT header，**不使用原生 `EventSource`**。

## 环境

- `VITE_API_BASE_URL`：dev 留空走 Vite `/api` 代理；prod 同源留空（nginx 反代 `/api`）。
- `vite.config.ts` 的 `/api` 代理 target 可经 `VITE_API_PROXY_TARGET` 覆盖（默认 `http://localhost:8080`）。
