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

## Android 系统返回键

- 返回键由 Tauri 内置 AppPlugin 接管：JS 注册 `onBackButtonPress` 后默认行为被禁用（不再自动 goBack / 退出）；未注册时 webview 有历史则 `goBack()`，否则 `finish()` 直接退出。
- Kairos 是 state 导航 SPA（webview 无 history），**必须注册监听**，否则返回键 = 退出应用。
- 处理模式：维护前端导航栈（`src/App.tsx`）——子页面进入压栈、返回出栈；栈空（主页面）时调用 `exit_app` 命令（`commands/app.rs`）退出。
- **官方 `plugin:app|exit` 不可用**：tauri `build.rs` 未将 exit 注册进 ACL，JS invoke 会被拒绝；自研 `tauri::command`（`AppHandle::exit(0)`）不走 ACL，是正确退出方式。
- 平台判断用 `navigator.userAgent.includes("Android")`（Tauri Android webview UA 恒含 Android），避免引入 `@tauri-apps/plugin-os` 依赖与权限。
- StrictMode 双 mount：async 注册必须用 mounted 标志 + cleanup `unregister()`，防止首轮 mount 的 listener 泄漏。
- 导航栈更新禁止在 setState updater 内嵌套 setState（StrictMode updater 双调用会重复压栈）；直接用事件闭包里的最新 state 构造新数组。

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
