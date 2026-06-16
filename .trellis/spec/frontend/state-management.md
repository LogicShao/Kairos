# State Management

> How state is managed in this project.

---

## Overview

Kairos 采用 **本地状态优先 + Tauri 后端同步** 的状态管理策略。由于是 Tauri 桌面应用，前端不直接操作数据库，所有持久化数据通过 Rust 后端管理。状态分类：
- **本地 UI 状态**：`useState`（表单输入、模态框开关、loading 状态）
- **外部订阅状态**：`useSyncExternalStore`（DOM 状态、浏览器 API）
- **后端数据状态**：Tauri IPC `invoke()` + `listen()` 事件（任务列表、番茄钟状态）

**无全局状态管理库**：项目未使用 Redux/Zustand/MobX，组件通过 props 传递共享状态。

---

## State Categories

### 1. 本地 UI 状态（useState）

**定义**：仅影响组件自身渲染的临时状态。

**使用场景**：
- 表单输入值
- 模态框/抽屉打开状态
- 加载/错误状态
- 组件内部的临时计算值

**示例**：表单输入管理（`src/components/todo/TaskForm.tsx:25-39`）

```typescript
export function TaskForm({ task, onSave, onCancel }: TaskFormProps) {
  // 表单字段状态
  const [title, setTitle] = useState(task?.title ?? "")
  const [description, setDescription] = useState(task?.description ?? "")
  const [status, setStatus] = useState<TaskStatus>(task?.status ?? "todo")
  const [priority, setPriority] = useState<TaskPriority>(task?.priority ?? "medium")
  const [dueDate, setDueDate] = useState(task?.due_date ?? "")
  const [tags, setTags] = useState(() => {
    if (!task?.tags) return ""
    try {
      const parsed = JSON.parse(task.tags) as string[]
      return Array.isArray(parsed) ? parsed.join(", ") : ""
    } catch {
      return task.tags
    }
  })
  
  // 提交状态
  const [saving, setSaving] = useState(false)
  
  // ...
}
```

**关键点**：
- 初始值使用 `??` 提供默认值
- 复杂初始化逻辑使用函数形式 `useState(() => { /* ... */ })`
- 类型注解确保类型安全（`useState<TaskStatus>`）

### 2. 外部订阅状态（useSyncExternalStore）

**定义**：订阅 React 之外的数据源（DOM、localStorage、浏览器 API）。

**使用场景**：
- 主题切换（订阅 `<html>` class 变化）
- 响应式布局（订阅 `matchMedia` 变化）
- localStorage 同步
- WebSocket 连接状态

**示例**：主题订阅（`src/hooks/use-theme.ts:1-46`）

```typescript
// 获取当前状态
function getSnapshot(): "dark" | "light" {
  return document.documentElement.classList.contains("dark") ? "dark" : "light"
}

// 订阅变化
function subscribe(callback: () => void) {
  const observer = new MutationObserver(callback)
  observer.observe(document.documentElement, { 
    attributes: true, 
    attributeFilter: ["class"] 
  })
  return () => observer.disconnect()  // 返回清理函数
}

export function useTheme() {
  // 订阅 DOM 状态
  const theme = useSyncExternalStore(subscribe, getSnapshot)
  
  const toggle = useCallback(() => {
    applyTheme(theme === "dark" ? "light" : "dark")
  }, [theme])
  
  return { theme, toggle }
}
```

**关键点**：
- `getSnapshot()` 返回当前状态
- `subscribe(callback)` 注册监听器，返回清理函数
- React 自动处理组件卸载时的清理

### 3. 后端数据状态（Tauri IPC）

**定义**：从 Rust 后端获取的持久化数据。

**使用场景**：
- 任务列表（SQLite 数据库）
- 番茄钟状态（后端定时器）
- 课程表/考试列表
- 同步配置

**模式 A：一次性数据获取**

```typescript
const [data, setData] = useState<DataType | null>(null)

useEffect(() => {
  async function fetchData() {
    try {
      const result = await invoke<DataType>("get_data")
      setData(result)
    } catch (error) {
      console.error("获取数据失败:", error)
    }
  }
  fetchData()
}, [])
```

**模式 B：实时数据订阅**（`src/components/pomodoro/PomodoroTimer.tsx:26-77`）

```typescript
const [state, setState] = useState<PomodoroState | null>(null)

useEffect(() => {
  let unlistenTick: UnlistenFn | undefined
  let unlistenPhase: UnlistenFn | undefined

  async function init() {
    // 1. 获取初始状态
    const initial = await invoke<PomodoroState>("get_pomodoro_state")
    setState(initial)

    // 2. 订阅实时更新
    unlistenTick = await listen<PomodoroState>("pomodoro-tick", (event) => {
      setState(event.payload)
    })

    unlistenPhase = await listen<string>("pomodoro-phase-change", (event) => {
      const phase = event.payload
      // 处理阶段切换逻辑
    })
  }

  init()

  // 3. 清理订阅
  return () => {
    unlistenTick?.()
    unlistenPhase?.()
  }
}, [])
```

**关键点**：
- 初始数据通过 `invoke()` 获取
- 实时更新通过 `listen()` 订阅 Tauri 事件
- 必须返回清理函数取消订阅

---

## When to Use Global State

**当前项目不使用全局状态管理**。数据共享通过以下方式实现：

### 1. Props 传递（短距离共享）

```tsx
// 父组件
function TodoPage() {
  const [tasks, setTasks] = useState<Task[]>([])
  
  return (
    <>
      <TaskForm onSave={handleSave} />
      <TaskList tasks={tasks} onUpdate={handleUpdate} />
    </>
  )
}
```

### 2. 组件组合（Context）

如果未来需要全局状态（如用户设置），应使用 React Context：

```tsx
// ✅ 正确：使用 Context
const SettingsContext = createContext<Settings | null>(null)

export function SettingsProvider({ children }: { children: ReactNode }) {
  const [settings, setSettings] = useState<Settings | null>(null)
  
  useEffect(() => {
    invoke<Settings>("get_settings").then(setSettings)
  }, [])
  
  return (
    <SettingsContext.Provider value={settings}>
      {children}
    </SettingsContext.Provider>
  )
}

export function useSettings() {
  const context = useContext(SettingsContext)
  if (!context) throw new Error("useSettings 必须在 SettingsProvider 内使用")
  return context
}
```

### 3. Tauri 后端统一管理

**推荐方式**：需要跨组件共享的数据放在后端管理，前端通过事件订阅同步。

```rust
// Rust 后端：广播状态变化
#[tauri::command]
async fn update_task(app: tauri::AppHandle, task: Task) -> Result<()> {
    // 更新数据库
    db::update_task(&task)?;
    
    // 广播事件通知所有监听者
    app.emit_all("task-updated", task)?;
    
    Ok(())
}
```

```typescript
// 前端：多个组件订阅同一事件
useEffect(() => {
  const unlisten = listen<Task>("task-updated", (event) => {
    setTasks(prev => prev.map(t => 
      t.id === event.payload.id ? event.payload : t
    ))
  })
  return () => { unlisten() }
}, [])
```

---

## Server State

### Tauri 命令调用模式

| 场景           | Tauri API                  | 示例                                   |
|----------------|----------------------------|----------------------------------------|
| 读取数据       | `invoke("get_xxx")`        | `invoke<Task[]>("get_tasks")`          |
| 写入数据       | `invoke("create_xxx")`     | `invoke("create_task", { task })`      |
| 更新数据       | `invoke("update_xxx")`     | `invoke("update_task", { id, task })`  |
| 删除数据       | `invoke("delete_xxx")`     | `invoke("delete_task", { id })`        |
| 订阅实时更新   | `listen("xxx-updated")`    | `listen("task-updated", handler)`      |

### 错误处理模式

```typescript
// ✅ 正确：完整的错误处理
async function handleSave(task: CreateTaskRequest) {
  setSaving(true)
  setError(null)
  
  try {
    await invoke("create_task", { task })
    onSuccess()
  } catch (err) {
    setError(err instanceof Error ? err.message : "保存失败")
    console.error("创建任务失败:", err)
  } finally {
    setSaving(false)
  }
}
```

**参考**：`src/components/todo/TaskForm.tsx:41-69`

### 离线降级模式

当 Tauri API 不可用（如开发时在浏览器中预览），应提供降级 UI：

```typescript
// src/components/pomodoro/PomodoroTimer.tsx:34-47
async function init() {
  try {
    const initial = await invoke<PomodoroState>("get_pomodoro_state")
    setState(initial)
  } catch {
    // 降级：使用模拟数据
    setState({
      phase: "work",
      remaining_seconds: 1500,
      total_seconds: 1500,
      is_running: false,
      completed_sessions: 0,
    })
    setError("Tauri 不可用 — 展示离线 UI")
    return
  }
}
```

---

## Derived State

### 直接计算（无需 useMemo）

**规则**：简单派生值直接在 render 中计算，无需缓存。

```typescript
// ✅ 正确：直接计算
export function PomodoroTimer() {
  const [state, setState] = useState<PomodoroState | null>(null)
  
  // 派生值：直接计算
  const progress = state 
    ? 1 - state.remaining_seconds / state.total_seconds
    : 0
  const isWork = state?.phase === "work"
  
  return (
    <div className={cn(isWork ? "text-primary" : "text-emerald-400")}>
      {/* ... */}
    </div>
  )
}
```

**参考**：`src/components/pomodoro/PomodoroTimer.tsx:100-105`

### 使用 useMemo（昂贵计算）

**规则**：仅对昂贵计算（排序、过滤大列表、复杂数学运算）使用 `useMemo`。

```typescript
// ✅ 正确：昂贵计算
const filteredTasks = useMemo(() => {
  return tasks
    .filter(t => t.status === filter)
    .sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime())
}, [tasks, filter])

// ❌ 错误：简单计算不需要 memo
const isDisabled = useMemo(() => !isEnabled, [isEnabled])  // 过度优化
```

---

## Common Mistakes

### 1. 忘记清理 Tauri 事件监听

```typescript
// ❌ 错误：未清理订阅导致内存泄漏
useEffect(() => {
  listen("event", handler)
}, [])

// ✅ 正确：返回清理函数
useEffect(() => {
  let unlisten: UnlistenFn | undefined
  
  async function init() {
    unlisten = await listen("event", handler)
  }
  init()
  
  return () => { unlisten?.() }
}, [])
```

### 2. 过度使用 useState

```typescript
// ❌ 错误：派生值存储为状态
const [total, setTotal] = useState(0)

useEffect(() => {
  setTotal(price * quantity)
}, [price, quantity])

// ✅ 正确：直接计算
const total = price * quantity
```

### 3. 在渲染中调用 invoke

```typescript
// ❌ 错误：每次渲染都调用后端
function Component() {
  const data = invoke("get_data")  // ❌ 会重复调用
  return <div>{data}</div>
}

// ✅ 正确：在 useEffect 中调用
function Component() {
  const [data, setData] = useState(null)
  
  useEffect(() => {
    invoke("get_data").then(setData)
  }, [])
  
  return <div>{data}</div>
}
```

### 4. 未处理异步初始化的 null 状态

```typescript
// ❌ 错误：未处理 loading 状态
function Component() {
  const [data, setData] = useState<Data | null>(null)
  
  useEffect(() => {
    invoke<Data>("get_data").then(setData)
  }, [])
  
  return <div>{data.title}</div>  // ❌ data 可能为 null
}

// ✅ 正确：提前返回 loading UI
function Component() {
  const [data, setData] = useState<Data | null>(null)
  
  useEffect(() => {
    invoke<Data>("get_data").then(setData)
  }, [])
  
  if (!data) {
    return <div>加载中...</div>
  }
  
  return <div>{data.title}</div>
}
```

**参考**：`src/components/pomodoro/PomodoroTimer.tsx:92-98`

---

## Checklist

在管理状态前，确认：

- [ ] 选择正确的状态类型（本地/外部订阅/后端数据）
- [ ] 使用 `useState` 初始化本地 UI 状态
- [ ] 使用 `useSyncExternalStore` 订阅外部数据源
- [ ] 使用 `invoke()` 获取后端数据
- [ ] 使用 `listen()` 订阅实时更新
- [ ] Effect 返回清理函数取消订阅
- [ ] 处理异步初始化的 loading/null 状态
- [ ] 简单派生值直接计算（不用 useMemo）
- [ ] 错误处理完整（try-catch + error 状态）
- [ ] 不在渲染中直接调用 invoke
