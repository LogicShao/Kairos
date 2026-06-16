# Hook Guidelines

> How hooks are used in this project.

---

## Overview

Kairos 自定义 hooks 遵循 React 官方规范，专注于封装可复用的状态逻辑和副作用。所有 hooks 必须：
- 文件名使用 `use-<name>.ts` 格式（kebab-case）
- 导出的函数名以 `use` 开头（camelCase）
- 返回值使用对象形式（便于选择性解构）
- 避免在 hooks 内部直接操作 DOM（通过返回值让组件处理）

**参考示例**：
- 外部状态订阅：`src/hooks/use-theme.ts`（使用 `useSyncExternalStore`）
- 本地存储持久化：`src/hooks/use-theme.ts`（localStorage + 系统主题检测）

---

## Custom Hook Patterns

### 1. 标准结构模板

```typescript
// 1. 常量定义（hook 外部）
const STORAGE_KEY = "app-setting"

// 2. 辅助函数（hook 外部，纯函数）
function getSnapshot(): StateType {
  // 获取当前状态
}

function subscribe(callback: () => void) {
  // 订阅状态变化
  return () => { /* 清理订阅 */ }
}

// 3. Hook 函数（导出）
export function useCustomHook(config?: ConfigType) {
  // 3.1 状态订阅
  const value = useSyncExternalStore(subscribe, getSnapshot)
  
  // 3.2 派生状态
  const derived = useMemo(() => computeDerived(value), [value])
  
  // 3.3 操作函数（使用 useCallback）
  const update = useCallback((newValue: StateType) => {
    // 更新逻辑
  }, [])
  
  // 3.4 返回对象（便于选择性解构）
  return { value, derived, update }
}

// 4. 独立的操作函数（可选，供组件直接调用）
export function applyConfig(config: ConfigType) {
  // 同步操作，无需 hooks 上下文
}
```

### 2. 真实示例：useTheme

参考 `src/hooks/use-theme.ts:1-46`：

```typescript
import { useCallback, useSyncExternalStore } from "react"

const THEME_KEY = "kairos-theme"

// 辅助函数：获取当前主题
function getSnapshot(): "dark" | "light" {
  return document.documentElement.classList.contains("dark") ? "dark" : "light"
}

// 辅助函数：订阅 DOM 变化
function subscribe(callback: () => void) {
  const observer = new MutationObserver(callback)
  observer.observe(document.documentElement, { 
    attributes: true, 
    attributeFilter: ["class"] 
  })
  return () => observer.disconnect()
}

// 辅助函数：解析主题（localStorage → 系统偏好）
function resolveTheme(): "dark" | "light" {
  const stored = localStorage.getItem(THEME_KEY)
  if (stored === "dark" || stored === "light") return stored
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light"
}

// 独立操作函数（供外部调用）
export function applyTheme(theme: "dark" | "light") {
  const root = document.documentElement
  if (theme === "dark") {
    root.classList.add("dark")
  } else {
    root.classList.remove("dark")
  }
  localStorage.setItem(THEME_KEY, theme)
}

// Hook 函数
export function useTheme() {
  // 订阅 DOM 状态
  const theme = useSyncExternalStore(subscribe, getSnapshot)

  // 操作函数（memoized）
  const toggle = useCallback(() => {
    applyTheme(theme === "dark" ? "light" : "dark")
  }, [theme])

  const setTheme = useCallback((t: "dark" | "light") => {
    applyTheme(t)
  }, [])

  // 返回对象（便于选择性解构）
  return { theme, toggle, setTheme }
}

// 初始化逻辑（在模块加载时执行）
applyTheme(resolveTheme())
```

**关键设计点**：
1. **外部状态订阅**：使用 `useSyncExternalStore` 订阅 DOM class 变化
2. **持久化**：通过 `localStorage` 保存用户选择
3. **回退逻辑**：无存储值时使用系统主题偏好（`prefers-color-scheme`）
4. **独立操作函数**：`applyTheme()` 可脱离 hook 上下文调用
5. **返回对象**：`{ theme, toggle, setTheme }` 便于按需解构

---

## Data Fetching

### Tauri IPC 数据获取模式

**规则**：
- 在组件内使用 `useEffect` + `invoke()` 获取数据（而非在 hook 中封装）
- 监听 Tauri 事件时，返回清理函数取消订阅

```typescript
// ✅ 正确：在组件中获取数据
import { useEffect, useState } from "react"
import { invoke } from "@tauri-apps/api/core"
import { listen, type UnlistenFn } from "@tauri-apps/api/event"

export function MyComponent() {
  const [data, setData] = useState<DataType | null>(null)

  useEffect(() => {
    let unlisten: UnlistenFn | undefined

    async function init() {
      // 初始数据获取
      const initial = await invoke<DataType>("get_data")
      setData(initial)

      // 订阅实时更新
      unlisten = await listen<DataType>("data-update", (event) => {
        setData(event.payload)
      })
    }

    init()

    return () => {
      unlisten?.()  // 清理订阅
    }
  }, [])

  return <div>{/* 使用 data */}</div>
}
```

**参考**：`src/components/pomodoro/PomodoroTimer.tsx:26-77`

### 本地数据获取（无需 hook 封装）

Kairos 是 **Tauri 桌面应用**，数据获取通过后端 Rust API 完成，前端只需：
1. 使用 `invoke()` 调用 Tauri 命令
2. 使用 `listen()` 订阅 Tauri 事件

**不需要**封装 React Query / SWR 等第三方库。

---

## Naming Conventions

| 类型           | 规则                          | 示例                          |
|----------------|-------------------------------|-------------------------------|
| 文件名         | kebab-case + use- 前缀        | `use-theme.ts`                |
| Hook 函数名    | camelCase + use 前缀          | `useTheme()`                  |
| 辅助函数       | camelCase（非 use 前缀）      | `getSnapshot()`, `subscribe()`|
| 独立操作函数   | camelCase                     | `applyTheme()`                |

### 命名示例

```typescript
// ✅ 正确
// 文件：src/hooks/use-local-storage.ts
export function useLocalStorage(key: string) { }
function getStorageValue() { }
function setStorageValue() { }

// ❌ 错误
// 文件：src/hooks/localStorage.ts          // ❌ 缺少 use- 前缀
export function localStorageHook() { }      // ❌ 不符合 useXxx 命名
function useGetValue() { }                  // ❌ 辅助函数不应有 use 前缀
```

---

## Return Value Patterns

### 1. 对象返回（推荐）

**规则**：返回对象便于选择性解构，且添加新字段不破坏现有代码。

```typescript
// ✅ 正确：对象返回
export function useTheme() {
  const theme = useSyncExternalStore(subscribe, getSnapshot)
  const toggle = useCallback(() => { /* ... */ }, [theme])
  
  return { theme, toggle, setTheme }
}

// 使用时可选择性解构
const { theme } = useTheme()              // 只需要 theme
const { theme, toggle } = useTheme()      // 需要 theme 和 toggle
```

**参考**：`src/hooks/use-theme.ts:31-43`

### 2. 数组返回（特殊场景）

**规则**：仅当返回值明确成对出现（如 state + setter）时使用。

```typescript
// ✅ 适用场景：仿 useState 接口
export function useToggle(initial = false) {
  const [value, setValue] = useState(initial)
  const toggle = useCallback(() => setValue(v => !v), [])
  return [value, toggle] as const
}

// 使用
const [isOpen, toggleOpen] = useToggle()
```

### 3. 单值返回（简单场景）

**规则**：仅当 hook 只返回一个派生值时使用。

```typescript
// ✅ 适用场景：只读的派生状态
export function useIsMobile() {
  const [isMobile, setIsMobile] = useState(false)
  
  useEffect(() => {
    const query = window.matchMedia("(max-width: 768px)")
    setIsMobile(query.matches)
    
    const handler = () => setIsMobile(query.matches)
    query.addEventListener("change", handler)
    return () => query.removeEventListener("change", handler)
  }, [])
  
  return isMobile
}

// 使用
const isMobile = useIsMobile()
```

---

## Dependency Management

### 1. useCallback 依赖项

**规则**：
- 回调函数内使用的外部变量必须加入依赖数组
- 使用 ESLint `react-hooks/exhaustive-deps` 规则检查

```typescript
// src/hooks/use-theme.ts:34-40
const toggle = useCallback(() => {
  applyTheme(theme === "dark" ? "light" : "dark")
}, [theme])  // ✅ theme 在函数内使用，必须列入依赖

const setTheme = useCallback((t: "dark" | "light") => {
  applyTheme(t)
}, [])  // ✅ 无外部依赖，空数组
```

### 2. useEffect 依赖项

**规则**：
- 订阅外部数据源时，依赖数组通常为空 `[]`（只在挂载时执行）
- 需要响应 props/state 变化时，必须列入依赖

```typescript
// ✅ 正确：订阅外部事件（只在挂载时执行）
useEffect(() => {
  const unlisten = listen("event", handler)
  return () => unlisten()
}, [])

// ✅ 正确：响应 userId 变化
useEffect(() => {
  fetchUserData(userId)
}, [userId])

// ❌ 错误：遗漏依赖
useEffect(() => {
  if (isEnabled) {
    doSomething(userId)  // ❌ userId 和 isEnabled 未列入依赖
  }
}, [])
```

### 3. useMemo 使用场景

**规则**：仅用于昂贵的计算，简单派生值直接在 render 中计算。

```typescript
// ✅ 正确：昂贵的计算
const sortedList = useMemo(() => {
  return items.slice().sort((a, b) => /* 复杂排序逻辑 */)
}, [items])

// ❌ 错误：简单派生值不需要 memo
const isDisabled = useMemo(() => !isEnabled, [isEnabled])  // ❌ 过度优化
const isDisabled = !isEnabled  // ✅ 直接计算
```

---

## Common Mistakes

### 1. Hook 内部直接操作 DOM

```typescript
// ❌ 错误：在 hook 内部操作 DOM
export function useModal() {
  const open = () => {
    document.getElementById("modal")!.style.display = "block"
  }
  return { open }
}

// ✅ 正确：返回状态，让组件处理 DOM
export function useModal() {
  const [isOpen, setIsOpen] = useState(false)
  return { isOpen, open: () => setIsOpen(true) }
}
```

### 2. 忘记清理订阅

```typescript
// ❌ 错误：未返回清理函数
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

**参考**：`src/components/pomodoro/PomodoroTimer.tsx:73-76`

### 3. 在循环/条件中调用 hooks

```typescript
// ❌ 错误：条件调用 hook
if (isEnabled) {
  const value = useCustomHook()  // ❌ Hook 调用顺序不稳定
}

// ✅ 正确：始终调用，条件放在内部
const value = useCustomHook()
if (isEnabled) {
  // 使用 value
}
```

### 4. 过度封装简单逻辑

```typescript
// ❌ 错误：为简单状态创建 hook
export function useCounter() {
  const [count, setCount] = useState(0)
  const increment = () => setCount(c => c + 1)
  return { count, increment }
}

// ✅ 正确：直接在组件中使用 useState
function Component() {
  const [count, setCount] = useState(0)
  // ...
}
```

**原则**：只有在逻辑被 2+ 个组件复用时才封装 hook。

---

## Checklist

在创建自定义 hook 前，确认：

- [ ] 文件名为 `use-<name>.ts`（kebab-case）
- [ ] 导出的函数名为 `useXxx`（camelCase）
- [ ] 返回值使用对象形式（便于扩展）
- [ ] 辅助函数放在 hook 外部（纯函数）
- [ ] 使用 `useCallback` 优化操作函数
- [ ] 依赖数组正确列出所有外部变量
- [ ] Effect 有清理函数（订阅/定时器）
- [ ] 不在 hook 内部直接操作 DOM
- [ ] 逻辑确实被 2+ 组件复用（避免过度封装）
