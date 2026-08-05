# Quality Guidelines

> Code quality standards for Kairos frontend development.

---

## Overview

Kairos 前端是 React 19 + TypeScript + Tailwind CSS v4，通过 Tauri IPC 与 Rust 后端通信。所有业务逻辑在 Rust 侧，前端只是渲染层。

---

## 环境约束

### 离线优先

- **禁止对外部 URL 发起网络请求** — 所有数据通过 Tauri IPC 从本地 SQLite 获取
- WebDAV 同步是唯一例外，通过 Rust 后端代理
- 组件中 import 的外部资源必须在本地可用（无 CDN）

### 无 emoji

- **代码中禁止使用 emoji**（包括注释、变量名、字符串字面量）
- 图标统一使用 Lucide React

---

## 样式规范

### Tailwind + cn()

```tsx
// ✅ 正确：使用 cn() 合并类名
import { cn } from "@/lib/utils"
<div className={cn("base-class", isActive && "active-class", className)} />

// ❌ 禁止：字符串拼接
<div className={"base " + (isActive ? "active" : "")} />
// ❌ 禁止：行内 style 属性（除动态计算值如 transform）
<div style={{color: "red"}}>...</div>
```

### 响应式设计

- **CSS 响应式优先** — 使用 Tailwind 断点前缀，禁止 JS 平台检测做布局
- 断点：`sm` (640px)、`md` (768px)、`lg` (1024px)
- 768px 是桌面/移动分界线

```tsx
// ✅ 正确：CSS 响应式
<nav className="hidden md:flex">           {/* 桌面侧边栏 */}
<nav className="flex md:hidden fixed bottom-0">  {/* 移动底部Tab */}

// ❌ 禁止：JS 平台检测做布局
import { platform } from '@tauri-apps/plugin-os'
if (platform() === 'android') { ... }  // 仅在调用原生API时使用
```

### 移动端触摸目标

- 所有交互元素在移动端最小 **44x44dp**（`min-h-11 min-w-11`）
- 桌面端可以更紧凑

```tsx
// ✅ 正确：移动端大按钮，桌面端正常
<button className="min-h-11 min-w-11 md:min-h-0 md:min-w-0 p-2 md:p-1.5">
```

### 移动端布局模式

| 桌面（≥768px） | 移动（<768px） |
|----------------|----------------|
| 侧边栏 + 表格 | 底部 Tab + 卡片列表 |
| 并排按钮 | 垂直堆叠 |
| 文字按钮 | 图标按钮（文字用 `hidden sm:inline`） |
| 多列网格 | 单列 + 导航切换 |

---

## 状态覆盖

每个数据展示组件必须覆盖三种状态：

```tsx
// ✅ 三态覆盖
{loading && <LoadingSkeleton />}
{error && <ErrorBanner message={error} onRetry={refetch} />}
{!loading && !error && data.length === 0 && <EmptyState />}
{!loading && !error && data.length > 0 && <DataView data={data} />}
```

---

## 性能

- 列表项 callback 用 `useCallback` 包裹
- 纯展示子组件用 `React.memo`
- 避免在 render 中创建新对象/数组作为依赖

---

## 禁止模式

| 禁止 | 原因 |
|------|------|
| `any` 类型 | 破坏类型安全 |
| 行内 `style={{}}` | 无法 tree-shake，破坏 Tailwind 一致性 |
| JS 平台检测做布局 | CSS 响应式足够，JS 检测引入分支复杂度 |
| `console.log` 提交 | 开发调试完就删 |
| 字符串拼接 className | 用 `cn()` 保证去重和优先级 |
| Emoji | 用 Lucide 图标 |
| 直接操作 DOM | 用 React 状态驱动 |

---

## 代码审查清单

- [ ] 离线兼容：无外部网络请求？
- [ ] 无 `any`，类型推导正确？
- [ ] 无行内 `style`，全部 Tailwind？
- [ ] `cn()` 正确使用，无字符串拼接？
- [ ] 三态覆盖：loading / error / empty？
- [ ] 移动端触摸目标 ≥ 44dp？
- [ ] CSS 响应式而非 JS 平台检测？
- [ ] 无 emoji？
- [ ] 无 `console.log` / `debugger`？
