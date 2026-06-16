# Type Safety

> Type safety patterns in this project.

---

## Overview

Kairos 前端使用 **TypeScript 6.0** 实现完全类型安全，所有代码必须通过 `tsc --noEmit` 检查。类型系统策略：
- **零容忍 `any`**：禁止使用 `any` 类型（使用 `unknown` 代替）
- **字面量类型优于枚举**：使用联合类型（`"todo" | "done"`）而非 `enum`
- **导出所有类型**：类型定义集中在 `src/types/` 并导出
- **类型推断优先**：简单场景依赖 TypeScript 推断，复杂场景显式注解

**参考示例**：
- 领域类型定义：`src/types/task.ts`
- 组件 Props 类型：`src/components/todo/TaskForm.tsx`

---

## Type Organization

### 1. 领域类型（`src/types/`）

**规则**：
- 按领域模型划分文件（`task.ts`、`pomodoro.ts`、`course.ts`）
- 所有类型使用 `export` 导出
- 类型名使用 PascalCase
- 文件名使用 kebab-case

**示例**：`src/types/task.ts`（完整内容）

```typescript
// 字面量类型（优于 enum）
export type TaskStatus = "todo" | "in_progress" | "done"
export type TaskPriority = "high" | "medium" | "low"

// 实体接口
export interface Task {
  id: number
  title: string
  description: string
  status: TaskStatus
  priority: TaskPriority
  due_date: string | null
  tags: string
  created_at: string
  updated_at: string
}

// 请求/响应类型
export interface CreateTaskRequest {
  title: string
  description?: string
  status?: TaskStatus
  priority?: TaskPriority
  due_date?: string | null
  tags?: string
}

export interface UpdateTaskRequest {
  title?: string
  description?: string
  status?: TaskStatus
  priority?: TaskPriority
  due_date?: string | null
  tags?: string
}

// 查询参数类型
export interface TaskFilterParams {
  status_filter?: string | null
  priority_filter?: string | null
  sort_by?: string | null
  sort_order?: string | null
}
```

**关键点**：
- `TaskStatus` 和 `TaskPriority` 使用字面量类型（非 enum）
- `Task` 是完整实体（所有字段必填）
- `CreateTaskRequest` 和 `UpdateTaskRequest` 是部分可选（`?`）
- `null` 和 `undefined` 语义不同：`null` 表示"无值"，`undefined` 表示"未提供"

### 2. 组件 Props 类型

**规则**：
- Props 接口在组件文件中定义（紧邻组件函数）
- 接口名为 `<ComponentName>Props`
- 回调函数必须指定完整类型签名

**示例**：`src/components/todo/TaskForm.tsx:6-10`

```typescript
interface TaskFormProps {
  task?: Task | null                                     // 可选 + nullable
  onSave: (task: CreateTaskRequest | UpdateTaskRequest) => void  // 联合类型
  onCancel: () => void                                   // 无参数回调
}

export function TaskForm({ task, onSave, onCancel }: TaskFormProps) {
  // ...
}
```

**关键点**：
- `task?: Task | null` 表示"可选参数，且可能为 null"
- `onSave` 接受联合类型参数（创建或更新）
- 回调函数明确 `void` 返回值

### 3. 常量类型

**规则**：使用 `as const` 断言推断字面量类型。

```typescript
// src/components/todo/TaskForm.tsx:12-22
const PRIORITY_OPTIONS: { value: TaskPriority; label: string }[] = [
  { value: "high", label: "高" },
  { value: "medium", label: "中" },
  { value: "low", label: "低" },
]

const STATUS_OPTIONS: { value: TaskStatus; label: string }[] = [
  { value: "todo", label: "待办" },
  { value: "in_progress", label: "进行中" },
  { value: "done", label: "已完成" },
]
```

**或使用 `as const` 获得更精确的类型**：

```typescript
// ✅ 推荐：as const 获得字面量类型
const PHASE_LABELS = {
  work: "专注",
  short_break: "短休",
  long_break: "长休",
} as const

type Phase = keyof typeof PHASE_LABELS  // "work" | "short_break" | "long_break"
```

**参考**：`src/components/pomodoro/PomodoroTimer.tsx:10-14`

---

## Validation

### 前端验证（TypeScript 静态检查）

**规则**：依赖 TypeScript 编译器进行类型检查，不使用运行时验证库（Zod/Yup）。

```typescript
// ✅ 正确：编译时类型安全
const payload: CreateTaskRequest = {
  title: title.trim(),
  description: description.trim() || undefined,
  status: status,           // 类型检查确保是 TaskStatus
  priority: priority,       // 类型检查确保是 TaskPriority
  due_date: dueDate || null,
  tags: JSON.stringify(tagList),
}

// ❌ 错误：会导致编译错误
const payload: CreateTaskRequest = {
  title: title.trim(),
  status: "invalid_status",  // ❌ 类型错误
  priority: 123,             // ❌ 类型错误
}
```

**参考**：`src/components/todo/TaskForm.tsx:51-58`

### 后端验证（Rust 类型系统）

前端只做基本的 UI 验证（非空检查、格式检查），业务逻辑验证由 Rust 后端完成：

```rust
// Rust 后端
#[derive(Deserialize)]
pub struct CreateTaskRequest {
    pub title: String,
    pub description: Option<String>,
    pub status: Option<TaskStatus>,
    // ...
}

#[tauri::command]
fn create_task(task: CreateTaskRequest) -> Result<Task, String> {
    if task.title.trim().is_empty() {
        return Err("标题不能为空".into());
    }
    // 其他验证逻辑
}
```

---

## Common Patterns

### 1. 字面量类型 vs 枚举

**规则**：始终使用字面量联合类型，禁止使用 `enum`。

```typescript
// ✅ 正确：字面量联合类型
export type TaskStatus = "todo" | "in_progress" | "done"

const status: TaskStatus = "todo"
if (status === "done") { /* ... */ }  // 字符串比较

// ❌ 错误：枚举（避免使用）
enum TaskStatus {
  Todo = "todo",
  InProgress = "in_progress",
  Done = "done",
}

const status = TaskStatus.Todo
if (status === TaskStatus.Done) { /* ... */ }  // 需要枚举成员访问
```

**原因**：
- 字面量类型更轻量（无运行时代码）
- 与后端 JSON 数据直接兼容
- 更符合 JavaScript/TypeScript 习惯

**参考**：`src/types/task.ts:1-2`

### 2. 可选属性 vs 联合 undefined

```typescript
// ✅ 推荐：可选属性（更简洁）
interface CreateTaskRequest {
  title: string
  description?: string      // 可选属性
  due_date?: string | null  // 可选 + nullable
}

// ❌ 避免：显式联合 undefined（冗余）
interface CreateTaskRequest {
  title: string
  description: string | undefined
  due_date: string | null | undefined
}
```

**语义区别**：
- `description?: string` — 参数可以不提供（省略）
- `due_date?: string | null` — 参数可以不提供，提供时可以是 null

**参考**：`src/types/task.ts:16-23`

### 3. 类型守卫（Type Guards）

**规则**：使用 `typeof`、`in` 运算符或自定义类型守卫缩窄类型。

```typescript
// 示例 1：typeof 守卫
function formatValue(value: string | number): string {
  if (typeof value === "string") {
    return value.toUpperCase()  // value 被缩窄为 string
  }
  return value.toFixed(2)       // value 被缩窄为 number
}

// 示例 2：in 运算符
interface Task { id: number; title: string }
interface CreateTaskRequest { title: string }

function process(data: Task | CreateTaskRequest) {
  if ("id" in data) {
    console.log(data.id)  // data 被缩窄为 Task
  } else {
    // data 被缩窄为 CreateTaskRequest
  }
}

// 示例 3：自定义类型守卫
function isTask(data: Task | CreateTaskRequest): data is Task {
  return "id" in data
}

if (isTask(data)) {
  console.log(data.id)  // data 被缩窄为 Task
}
```

### 4. 泛型工具类型

**规则**：使用 TypeScript 内置工具类型简化类型定义。

```typescript
// Partial<T> — 所有属性变为可选
type UpdateTaskRequest = Partial<Omit<Task, "id" | "created_at" | "updated_at">>

// Pick<T, K> — 选择部分属性
type TaskSummary = Pick<Task, "id" | "title" | "status">

// Omit<T, K> — 排除部分属性
type TaskWithoutTimestamps = Omit<Task, "created_at" | "updated_at">

// Required<T> — 所有属性变为必填
type CompleteTask = Required<Task>

// Record<K, V> — 键值对类型
type TaskStatusLabels = Record<TaskStatus, string>
const labels: TaskStatusLabels = {
  todo: "待办",
  in_progress: "进行中",
  done: "已完成",
}
```

### 5. 类型导入

**规则**：使用 `type` 关键字导入类型（便于区分值和类型）。

```typescript
// ✅ 推荐：显式 type 导入
import { useState } from "react"
import type { Task, CreateTaskRequest } from "@/types/task"
import { Button } from "@/components/ui/button"

// ✅ 也可以：混合导入
import { useState, type FormEvent } from "react"

// ❌ 避免：类型和值混在一起（不清晰）
import { Task, CreateTaskRequest } from "@/types/task"
```

**参考**：
- `src/components/todo/TaskForm.tsx:1-2`
- `src/components/pomodoro/PomodoroTimer.tsx:4`

---

## Forbidden Patterns

### 1. 禁止 `any` 类型

```typescript
// ❌ 错误：使用 any
function process(data: any) {
  return data.title  // 无类型检查
}

// ✅ 正确：使用具体类型
function process(data: Task) {
  return data.title
}

// ✅ 正确：使用 unknown（需要运行时检查）
function process(data: unknown) {
  if (typeof data === "object" && data !== null && "title" in data) {
    return (data as Task).title
  }
  throw new Error("Invalid data")
}
```

### 2. 禁止类型断言（非必要）

```typescript
// ❌ 错误：滥用类型断言
const title = (data as Task).title

// ✅ 正确：使用类型守卫
if (isTask(data)) {
  const title = data.title
}

// ✅ 例外：JSON 解析等场景（运行时无法验证的情况）
const parsed = JSON.parse(jsonString) as Task[]
```

**必要使用场景**：
- JSON 解析（`JSON.parse()` 返回 `any`）
- DOM API（`document.getElementById()` 返回 `HTMLElement | null`）
- Tauri API（`invoke<T>()` 需要显式类型参数）

**参考**：`src/components/pomodoro/PomodoroTimer.tsx:36`

```typescript
const initial = await invoke<PomodoroState>("get_pomodoro_state")
```

### 3. 禁止非空断言（`!`）

```typescript
// ❌ 错误：假设值一定存在
const element = document.getElementById("root")!
element.appendChild(child)  // 如果 element 为 null 会运行时报错

// ✅ 正确：显式检查
const element = document.getElementById("root")
if (element) {
  element.appendChild(child)
}

// ✅ 正确：提前返回
const element = document.getElementById("root")
if (!element) throw new Error("Root element not found")
element.appendChild(child)
```

**例外**：可选链 + 空值合并已确保安全的情况

```typescript
// ✅ 可接受：可选链确保安全
const title = task?.title ?? "未命名任务"
```

### 4. 禁止隐式 any

**规则**：所有函数参数、返回值必须有类型注解（或可推断）。

```typescript
// ❌ 错误：隐式 any 参数
function formatTask(task) {  // task 隐式为 any
  return task.title
}

// ✅ 正确：显式类型
function formatTask(task: Task): string {
  return task.title
}

// ✅ 正确：可推断的返回值（可省略注解）
function formatTask(task: Task) {  // 返回值推断为 string
  return task.title
}
```

**tsconfig.json 配置**：

```json
{
  "compilerOptions": {
    "strict": true,
    "noImplicitAny": true,
    "strictNullChecks": true
  }
}
```

---

## Checklist

在编写 TypeScript 代码前，确认：

- [ ] 所有类型定义在 `src/types/` 并导出
- [ ] 使用字面量类型（非 enum）
- [ ] 组件 Props 有显式接口定义
- [ ] 函数参数和返回值有类型注解
- [ ] 无 `any` 类型（使用 `unknown` 代替）
- [ ] 使用 `type` 关键字导入类型
- [ ] 可选属性使用 `?`（非 `| undefined`）
- [ ] 常量使用 `as const` 断言
- [ ] 避免非必要的类型断言
- [ ] 代码通过 `tsc --noEmit` 检查
