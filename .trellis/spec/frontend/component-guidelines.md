# Component Guidelines

> How components are built in this project.

---

## Overview

Kairos 组件采用 **函数组件 + TypeScript + Tailwind CSS** 模式，严格遵循类型安全和离线优先原则。所有组件必须：
- 使用函数组件（禁止 class 组件）
- 显式定义 props 类型（禁止隐式 `any`）
- 使用 Lucide React 图标（禁止 emoji 字符）
- 通过 `cn()` 工具函数合并 Tailwind 类名
- 使用 shadcn/ui 组件（Button、Card 等）而非自己实现

**参考示例**：
- 复杂交互组件：`src/components/pomodoro/PomodoroTimer.tsx`
- 表单组件：`src/components/todo/TaskForm.tsx`

---

## Component Structure

### 标准结构模板

```tsx
// 1. 依赖导入（React hooks → Tauri API → 类型 → UI 组件 → 工具函数 → 图标）
import { useState, useEffect, useCallback } from "react"
import { invoke } from "@tauri-apps/api/core"
import type { PomodoroState } from "@/types/pomodoro"
import { Button } from "@/components/ui/button"
import { cn } from "@/lib/utils"
import { Play, Pause } from "lucide-react"

// 2. 常量定义（组件外部，避免重复创建）
const PHASE_LABELS: Record<string, string> = {
  work: "专注",
  short_break: "短休",
}

// 3. 辅助函数（纯函数，组件外部）
function formatTime(seconds: number): string {
  const m = Math.floor(seconds / 60)
  const s = seconds % 60
  return `${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`
}

// 4. Props 类型定义
interface ComponentProps {
  onSave: (data: SomeType) => void
  initialValue?: string
}

// 5. 组件函数（导出）
export function ComponentName({ onSave, initialValue }: ComponentProps) {
  // 5.1 状态声明
  const [value, setValue] = useState(initialValue ?? "")
  
  // 5.2 Effects（副作用）
  useEffect(() => {
    // 订阅/初始化逻辑
    return () => {
      // 清理逻辑
    }
  }, [])
  
  // 5.3 事件处理函数（使用 useCallback 优化）
  const handleClick = useCallback(() => {
    // 处理逻辑
  }, [])
  
  // 5.4 提前返回（loading/error 状态）
  if (!value) {
    return <div>Loading...</div>
  }
  
  // 5.6 主渲染
  return (
    <div className="flex flex-col gap-4">
      {/* JSX 内容 */}
    </div>
  )
}
```

### 真实示例：PomodoroTimer

参考 `src/components/pomodoro/PomodoroTimer.tsx:22-188`：

```tsx
export function PomodoroTimer() {
  // 状态
  const [state, setState] = useState<PomodoroState | null>(null)
  const [error, setError] = useState<string | null>(null)

  // Effect: 初始化 Tauri IPC 监听
  useEffect(() => {
    let unlistenTick: UnlistenFn | undefined
    
    async function init() {
      const initial = await invoke<PomodoroState>("get_pomodoro_state")
      setState(initial)
      
      unlistenTick = await listen<PomodoroState>("pomodoro-tick", (event) => {
        setState(event.payload)
      })
    }
    
    init()
    return () => { unlistenTick?.() }
  }, [])

  // 事件处理（memoized）
  const handleStartPause = useCallback(() => {
    if (!state) return
    if (state.is_running) {
      invoke("pause_pomodoro").catch(console.error)
    } else {
      invoke("start_pomodoro").catch(console.error)
    }
  }, [state])

  // 提前返回 loading 状态
  if (!state) {
    return <div className="flex items-center justify-center h-96">
      <div className="animate-spin h-8 w-8 border-2 border-primary" />
    </div>
  }

  // 主渲染
  return (
    <div className="flex flex-col items-center gap-6">
      {/* SVG 进度环 + 按钮 */}
    </div>
  )
}
```

---

## Props Conventions

### 1. 显式类型定义

**规则**：所有 props 必须定义接口，禁止内联类型或隐式 `any`。

```tsx
// ✅ 正确：独立接口
interface TaskFormProps {
  task?: Task | null
  onSave: (task: CreateTaskRequest | UpdateTaskRequest) => void
  onCancel: () => void
}

export function TaskForm({ task, onSave, onCancel }: TaskFormProps) {
  // ...
}

// ❌ 错误：内联类型
export function TaskForm({ task, onSave }: { 
  task?: Task
  onSave: (task: any) => void  // ❌ 使用了 any
}) { }
```

**参考**：`src/components/todo/TaskForm.tsx:6-10`

### 2. 可选 Props 默认值

**规则**：使用空值合并运算符（`??`）提供默认值。

```tsx
// src/components/todo/TaskForm.tsx:25-28
const [title, setTitle] = useState(task?.title ?? "")
const [status, setStatus] = useState<TaskStatus>(task?.status ?? "todo")
const [priority, setPriority] = useState<TaskPriority>(task?.priority ?? "medium")
```

### 3. 回调函数类型

**规则**：回调函数必须指定参数和返回值类型。

```tsx
// ✅ 正确：完整类型
interface Props {
  onSave: (data: CreateTaskRequest) => void
  onDelete: (id: number) => Promise<void>
}

// ❌ 错误：缺少类型
interface Props {
  onSave: (data) => void           // ❌ 参数无类型
  onDelete: () => any              // ❌ 返回 any
}
```

---

## Styling Patterns

### 1. Tailwind + cn() 工具函数

**规则**：
- 所有样式使用 Tailwind CSS 类名
- 禁止内联 `style` 属性
- 动态类名通过 `cn()` 合并

```tsx
// src/components/pomodoro/PomodoroTimer.tsx:130-134
<circle
  className={cn(
    isWork ? "text-primary" : "text-emerald-400",
    "transition-[stroke-dashoffset] duration-1000 ease-linear",
  )}
/>
```

**cn() 定义**（`src/lib/utils.ts:4-6`）：
```tsx
import { clsx, type ClassValue } from "clsx"
import { twMerge } from "tailwind-merge"

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}
```

### 2. 条件样式

```tsx
// ✅ 正确：使用 cn() 和三元表达式
<button
  className={cn(
    "inline-flex items-center justify-center rounded-full h-12 w-12",
    isWork
      ? "bg-primary text-primary-foreground hover:bg-primary/90"
      : "bg-emerald-500 text-white hover:bg-emerald-600",
  )}
>
  {/* ... */}
</button>

// ❌ 错误：内联 style
<button style={{ backgroundColor: isWork ? 'blue' : 'green' }}>
```

**参考**：`src/components/pomodoro/PomodoroTimer.tsx:164-169`

### 3. shadcn/ui 组件样式覆盖

**规则**：通过 `className` prop 传递额外的 Tailwind 类。

```tsx
// src/components/todo/TaskForm.tsx:79
<Button 
  type="button" 
  variant="ghost" 
  size="icon" 
  onClick={onCancel} 
  className="h-7 w-7"  // 覆盖默认尺寸
>
  <X className="h-4 w-4" />
</Button>
```

---

## Accessibility

### 1. 交互元素的 aria-label

**规则**：所有图标按钮必须提供 `aria-label`。

```tsx
// src/components/pomodoro/PomodoroTimer.tsx:170-171
<button
  onClick={handleStartPause}
  aria-label={state.is_running ? "暂停" : "开始"}
>
  {state.is_running ? <Pause /> : <Play />}
</button>
```

### 2. 表单字段标签

**规则**：
- 所有 `<input>` / `<textarea>` / `<select>` 必须关联 `<label>`
- 必填字段使用 `<span className="text-destructive">*</span>` 标识

```tsx
// src/components/todo/TaskForm.tsx:86-96
<label className="block text-xs font-medium text-muted-foreground mb-1">
  标题 <span className="text-destructive">*</span>
</label>
<input
  type="text"
  value={title}
  onChange={(e) => setTitle(e.target.value)}
  required
  placeholder="任务标题"
  className="w-full h-9 rounded-md border border-input ..."
/>
```

### 3. 语义化 HTML

**规则**：使用正确的 HTML 元素（`<button>`、`<form>`、`<label>`）。

```tsx
// ✅ 正确
<form onSubmit={handleSubmit}>
  <button type="submit">保存</button>
</form>

// ❌ 错误
<div onClick={handleSubmit}>
  <div onClick={handleSave}>保存</div>
</div>
```

---

## Common Mistakes

### 1. 使用 emoji 字符代替图标

```tsx
// ❌ 错误
<button>▶️ 开始</button>

// ✅ 正确：使用 Lucide React
import { Play } from "lucide-react"
<button><Play className="h-4 w-4" /> 开始</button>
```

**原因**：项目禁止 emoji（见 commit `4b378bc`）。

### 2. 引用外部 CDN 资源

```tsx
// ❌ 错误
<img src="https://cdn.example.com/icon.png" />

// ✅ 正确：本地资源
import iconUrl from "@/assets/icon.png"
<img src={iconUrl} />
```

**原因**：离线优先硬约束，禁止 `https?://` 外部引用。

### 3. 未 memo 事件处理函数

```tsx
// ❌ 错误：每次渲染都创建新函数
export function Component() {
  const handleClick = () => {
    invoke("some_command")
  }
  return <button onClick={handleClick}>Click</button>
}

// ✅ 正确：使用 useCallback
export function Component() {
  const handleClick = useCallback(() => {
    invoke("some_command")
  }, [])
  return <button onClick={handleClick}>Click</button>
}
```

**参考**：`src/components/pomodoro/PomodoroTimer.tsx:79-90`

### 4. Props 类型使用 any

```tsx
// ❌ 错误
interface Props {
  data: any
  onChange: (val: any) => void
}

// ✅ 正确
interface Props {
  data: Task
  onChange: (val: Task) => void
}
```

---

## Checklist

在提交组件代码前，确认：

- [ ] 使用函数组件（非 class 组件）
- [ ] Props 有显式的 TypeScript 接口定义
- [ ] 所有状态变量有明确的类型注解
- [ ] 事件处理函数使用 `useCallback` 优化
- [ ] 样式使用 Tailwind + `cn()` 工具函数
- [ ] 图标使用 Lucide React（非 emoji）
- [ ] 交互元素有 `aria-label`（图标按钮）
- [ ] 表单字段有关联的 `<label>`
- [ ] 无外部 CDN 引用（图片/字体/样式）
- [ ] 使用 shadcn/ui 组件（Button、Card 等）而非自己实现
