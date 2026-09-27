# Frontend Development Guidelines

> Kairos 前端开发规范 — React 19 + TypeScript + Tailwind CSS v4 + Go 后端 REST API

---

## 预开发检查清单

在开始写前端代码前：

1. **确认数据来源** — 通过 `src/lib/api/<domain>.ts` 的 HTTP 封装从 Go 后端获取；组件内禁止直接 `fetch` 或拼接后端 URL
2. **确认模块归属** — 新组件放在 `src/components/<feature>/` 下，共享组件放 `shared/`；API 封装放 `src/lib/api/`
3. **确认类型定义** — 在 `src/types/` 中定义好 REST 端点的入参/返回值类型（snake_case）
4. **读相关 spec** — 下面列表中的具体 guide 文件

---

## Guidelines Index

| Guide | Description | Status |
|-------|-------------|--------|
| [Directory Structure](./directory-structure.md) | Module organization and file layout | To fill |
| [API Layer](./api-layer.md) | HTTP client 封装、JWT、错误归一化、SSE、事件→拉取/本地时钟 | ✅ Filled |
| [Component Guidelines](./component-guidelines.md) | 响应式组件模式、Props约定、双渲染/渐进增强 | ✅ Filled |
| [Hook Guidelines](./hook-guidelines.md) | Custom hooks, data fetching patterns | To fill |
| [State Management](./state-management.md) | 无全局状态库、本地 useState + `src/lib/api/` 请求 | To fill |
| [Quality Guidelines](./quality-guidelines.md) | 离线优先、响应式、触摸目标、三态覆盖、禁止模式 | ✅ Filled |
| [Type Safety](./type-safety.md) | Tauri IPC contract mirrors, field comments, optional/null rules | ✅ Filled |
| [Input Patterns](./input-patterns.md) | 数字输入组件选型指南：步进器/滑块/滚轮/自定义键盘 | ✅ Filled |

---

## 质量检查

完成后对照以下项检查：

1. 运行 `npm run lint` + `npx tsc --noEmit`
2. 对照 [Quality Guidelines](./quality-guidelines.md) 逐项核对
3. 对照 [Component Guidelines](./component-guidelines.md) 检查组件结构
4. 数据访问对照 [API Layer](./api-layer.md)：走 `src/lib/api/`、字段 snake_case、错误经 `userErrorMessage`
5. 验证三态覆盖：loading / error / empty
6. 缩小窗口到 <768px 验证移动端布局
