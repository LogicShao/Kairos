# State Management

> How state is managed in this project.

---

## Overview

<!--
Document your project's state management conventions here.

Questions to answer:
- What state management solution do you use?
- How is local vs global state decided?
- How do you handle server state?
- What are the patterns for derived state?
-->

(To be filled by the team)

---

## State Categories

<!-- Local state, global state, server state, URL state -->

(To be filled by the team)

---

## When to Use Global State

<!-- Criteria for promoting state to global -->

(To be filled by the team)

---

## Server State

<!-- How server data is cached and synchronized -->

- **无服务端状态缓存库**：不使用 react-query 等（当前规模不需要）。
- 数据获取统一走 `src/lib/api/<domain>.ts`（见 [API Layer](./api-layer.md)）；组件内以 `useState` 持有结果，在 `useEffect` 中加载，操作成功后本地刷新（重新请求）。
- 不再有后端推送事件：番茄钟用**本地 `setInterval` 时钟** + 挂载/窗口聚焦时 `getState()` 校准；阶段到点调 `finishPhase()`。
- 加载/错误/空态：每个数据区块显式覆盖三态；错误统一经 `userErrorMessage(err, fallback)` 展示。

---

## Common Mistakes

<!-- State management mistakes your team has made -->

(To be filled by the team)
