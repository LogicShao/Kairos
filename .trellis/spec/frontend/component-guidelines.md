# Component Guidelines

> How components are built in Kairos.

---

## 组件结构

### 函数组件 + TypeScript

```tsx
import type { ReactNode } from "react"
import { cn } from "@/lib/utils"

interface MyComponentProps {
  title: string
  children?: ReactNode
  className?: string
}

export function MyComponent({ title, children, className }: MyComponentProps) {
  return (
    <div className={cn("base-class", className)}>
      <h2>{title}</h2>
      {children}
    </div>
  )
}
```

### 文件组织

```
src/components/
├── shared/           # 跨模块共享（AppShell, AppBackground, AcrylicPanel, Modal）
├── pomodoro/         # 番茄钟模块
├── todo/             # 待办模块
├── courses/          # 课程表模块
├── exams/            # 考试模块
├── sync/             # 同步模块
└── ui/               # shadcn/ui 基础组件（button, card, etc.）
```

每个模块目录自包含，不跨模块导入内部组件。

---

## Props 约定

- 用 `interface` 定义 props（不用 `type`）
- `className?: string` 始终透传以支持外部样式覆盖
- 子元素用 `ReactNode` 类型
- 事件回调用 `() => void` 签名，调用方提供具体实现

```tsx
interface ComponentProps {
  /** 组件唯一标识 */
  id: string
  /** 点击回调 */
  onClick?: () => void
  /** 额外 CSS 类名 */
  className?: string
}
```

---

## 响应式组件模式

### 双渲染模式（桌面/移动各自一套 DOM）

```tsx
// ✅ 适用于桌面/移动布局差异大的场景
<div className="hidden md:block">{/* 桌面布局 */}</div>
<div className="md:hidden">{/* 移动布局 */}</div>
```

### 渐进增强模式（同一 DOM，class 响应式变化）

```tsx
// ✅ 适用于布局相似仅尺寸/方向不同的场景
<div className="flex flex-col sm:flex-row gap-2 md:gap-4">
<button className="min-h-11 md:min-h-0 px-3 md:px-2">
```

### 按钮文字响应式隐藏

```tsx
<button className="p-2 md:px-3">
  <Icon className="h-5 w-5 md:h-4 md:w-4" />
  <span className="hidden sm:inline">按钮文字</span>
</button>
```

---

## 状态管理模式

- **无全局状态库** — 不使用 Zustand / Redux / Context 做业务状态
- 本地 `useState` + Tauri `invoke()` 做数据获取
- 实时更新用 `listen()` 监听后端事件

```tsx
// 番茄钟状态：listen 后端 push，本地渲染
useEffect(() => {
  const unlisten = listen<PomodoroState>("pomodoro-tick", (e) => {
    setState(e.payload)
  })
  return () => { unlisten.then(fn => fn()) }
}, [])
```

---

## 无障碍 (A11y)

- 交互元素必须有 `aria-label`（图标按钮）或可见文字
- 导航按钮用 `aria-current="page"` 标记当前项
- 触摸目标 ≥ 44dp 保证可用性

---

## 常见错误

| 错误 | 修正 |
|------|------|
| 忘记 `pb-16 md:pb-0` 给固定底部栏留空间 | 固定 bar + 内容区 bottom padding |
| 移动端按钮太小无法点击 | `min-h-11 min-w-11` |
| 用 `platform()` 判断布局 | 用 Tailwind `md:` 前缀 |
| `useEffect` 缺少清理函数 | 返回 unlisten cleanup |
