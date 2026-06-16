# Quality Guidelines

> Code quality standards for frontend development.

---

## Overview

Kairos 前端代码质量要求：
- **离线优先硬约束**：禁止任何外部网络资源引用（CDN、字体、图片、API）
- **图标规范**：使用 Lucide React，禁止 emoji 字符
- **样式规范**：Tailwind CSS v4 + `cn()` 工具函数，禁止内联 `style`
- **TypeScript 严格模式**：禁止 `any`，启用 `strict`、`noImplicitAny`、`strictNullChecks`
- **无障碍要求**：交互元素必须有 `aria-label`，表单字段必须关联 `<label>`

**项目特有约定**（见 commit `4b378bc`）：
- 禁止 emoji 字符（用 Lucide 图标代替）
- 所有资源本地打包（离线可用）

---

## Forbidden Patterns

### 1. 禁止外部 CDN 引用

**硬约束**：代码中不得出现 `https://` 或 `http://` 开头的资源引用。

```typescript
// ❌ 错误：外部 CDN
<link href="https://cdn.tailwindcss.com/3.0.0/tailwind.min.css" rel="stylesheet" />
<img src="https://example.com/avatar.png" />
<script src="https://unpkg.com/react@18/umd/react.production.min.js" />

// ✅ 正确：本地资源
import "./styles.css"
import avatarUrl from "@/assets/avatar.png"
<img src={avatarUrl} />
```

**检查方式**：

```bash
# 搜索外部引用
grep -rE 'https?://' src/
```

**原因**：离线优先设计，应用必须在无网络环境下完整运行。

### 2. 禁止 emoji 字符

**规则**：UI 中不使用 emoji 字符，必须用 Lucide React 图标。

```typescript
// ❌ 错误：emoji 字符
<button>▶️ 开始</button>
<span>✅ 已完成</span>
<div>📅 {dueDate}</div>

// ✅ 正确：Lucide React 图标
import { Play, Check, Calendar } from "lucide-react"

<button><Play className="h-4 w-4" /> 开始</button>
<span><Check className="h-4 w-4" /> 已完成</span>
<div><Calendar className="h-4 w-4" /> {dueDate}</div>
```

**参考**：
- `src/components/pomodoro/PomodoroTimer.tsx:6` — 导入 Lucide 图标
- `src/components/pomodoro/PomodoroTimer.tsx:172-175` — 使用图标

**原因**：emoji 在不同操作系统/字体下显示不一致，且无法精确控制尺寸和颜色。

### 3. 禁止内联 style 属性

**规则**：样式必须使用 Tailwind CSS 类名，禁止内联 `style` 属性。

```typescript
// ❌ 错误：内联 style
<div style={{ backgroundColor: 'red', padding: '16px' }}>
  内容
</div>

// ✅ 正确：Tailwind 类名
<div className="bg-red-500 p-4">
  内容
</div>

// ✅ 正确：动态样式使用 cn()
<div className={cn(
  "p-4",
  isActive ? "bg-primary" : "bg-muted"
)}>
  内容
</div>
```

**例外**：需要动态计算的值（如 SVG 路径、动画进度）

```typescript
// ✅ 可接受：动态计算的 SVG 属性
<circle
  strokeDashoffset={offset}  // 运行时计算的进度值
  className="text-primary transition-[stroke-dashoffset]"
/>
```

**参考**：`src/components/pomodoro/PomodoroTimer.tsx:128-129`

### 4. 禁止 TypeScript `any` 类型

**规则**：所有类型必须显式定义或可推断，禁止 `any`。

```typescript
// ❌ 错误：any 类型
function handleSave(data: any) {
  invoke("save_task", { task: data })
}

interface Props {
  onChange: (value: any) => void
}

// ✅ 正确：具体类型
function handleSave(data: CreateTaskRequest) {
  invoke("save_task", { task: data })
}

interface Props {
  onChange: (value: Task) => void
}
```

**例外**：使用 `unknown` 代替需要运行时检查的场景

```typescript
// ✅ 正确：unknown + 运行时检查
function parseResponse(response: unknown) {
  if (typeof response === "object" && response !== null && "data" in response) {
    return response.data
  }
  throw new Error("Invalid response")
}
```

### 5. 禁止手动创建 shadcn/ui 组件

**规则**：`components/ui/` 目录下的组件只能通过 shadcn CLI 生成。

```bash
# ✅ 正确：使用 CLI 生成
npx shadcn@latest add button
npx shadcn@latest add card

# ❌ 错误：手动创建
touch src/components/ui/my-button.tsx
```

**原因**：shadcn/ui 组件有标准的 Tailwind + CVA 模式，手动创建容易偏离规范。

---

## Required Patterns

### 1. 样式合并必须使用 cn() 工具函数

**规则**：合并 Tailwind 类名时使用 `cn()` 工具函数（而非字符串拼接）。

```typescript
// ❌ 错误：字符串拼接
<div className={`p-4 ${isActive ? 'bg-primary' : 'bg-muted'}`}>

// ❌ 错误：数组 join
<div className={['p-4', isActive ? 'bg-primary' : 'bg-muted'].join(' ')}>

// ✅ 正确：使用 cn()
import { cn } from "@/lib/utils"

<div className={cn(
  "p-4",
  isActive ? "bg-primary" : "bg-muted"
)}>
```

**cn() 定义**（`src/lib/utils.ts:4-6`）：

```typescript
import { clsx, type ClassValue } from "clsx"
import { twMerge } from "tailwind-merge"

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}
```

**优势**：
- 自动去重冲突的 Tailwind 类（如 `text-red-500` 和 `text-blue-500`）
- 支持条件类名（对象语法）
- 支持数组嵌套

**参考**：`src/components/pomodoro/PomodoroTimer.tsx:130-134`

### 2. 图标必须来自 Lucide React

**规则**：所有图标从 `lucide-react` 导入。

```typescript
// ✅ 正确
import { Play, Pause, RotateCcw, Save, X } from "lucide-react"

<button>
  <Play className="h-4 w-4" />
  开始
</button>
```

**常用图标规格**：
- 小图标（列表项、表单）：`h-3.5 w-3.5` 或 `h-4 w-4`
- 按钮图标：`h-4 w-4` 或 `h-5 w-5`
- 大图标（空状态、引导）：`h-8 w-8` 或更大

**参考**：
- `src/components/pomodoro/PomodoroTimer.tsx:6`
- `src/components/todo/TaskForm.tsx:4`

### 3. 交互元素必须有无障碍属性

**规则**：
- 图标按钮必须有 `aria-label`
- 表单字段必须关联 `<label>`
- 必填字段使用视觉标识（`<span className="text-destructive">*</span>`）

```typescript
// ✅ 正确：图标按钮 + aria-label
<button
  onClick={handleStartPause}
  aria-label={state.is_running ? "暂停" : "开始"}
>
  {state.is_running ? <Pause /> : <Play />}
</button>

// ✅ 正确：表单字段 + label
<label className="block text-xs font-medium text-muted-foreground mb-1">
  标题 <span className="text-destructive">*</span>
</label>
<input
  type="text"
  value={title}
  onChange={(e) => setTitle(e.target.value)}
  required
  placeholder="任务标题"
/>
```

**参考**：
- `src/components/pomodoro/PomodoroTimer.tsx:170`
- `src/components/todo/TaskForm.tsx:86-96`

### 4. 事件处理函数必须使用 useCallback

**规则**：传递给子组件或作为依赖项的事件处理函数必须用 `useCallback` 包裹。

```typescript
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

### 5. 异步操作必须有 loading 状态

**规则**：调用 Tauri 命令时，必须显示 loading 状态并处理错误。

```typescript
// ✅ 正确：完整的 loading + error 处理
const [saving, setSaving] = useState(false)
const [error, setError] = useState<string | null>(null)

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

return (
  <>
    <Button disabled={saving}>
      {saving ? "保存中..." : "保存"}
    </Button>
    {error && <p className="text-destructive text-sm">{error}</p>}
  </>
)
```

**参考**：`src/components/todo/TaskForm.tsx:39-69`

---

## Testing Requirements

### 当前状态

项目目前 **未配置测试框架**，以下为推荐的测试策略（待实施）：

### 推荐测试框架

- **单元测试**：Vitest（与 Vite 原生集成）
- **组件测试**：@testing-library/react
- **E2E 测试**：Playwright（适合 Tauri 应用）

### 测试覆盖率目标

| 类型           | 覆盖率要求 | 说明                          |
|----------------|------------|-------------------------------|
| 工具函数       | 100%       | `lib/utils.ts` 等纯函数       |
| 自定义 hooks   | 80%+       | `hooks/use-theme.ts` 等       |
| UI 组件        | 60%+       | 关键交互逻辑（按钮、表单）    |
| 类型定义       | N/A        | 由 TypeScript 编译器保证      |

### 测试示例（待实施）

```typescript
// lib/utils.test.ts
import { describe, it, expect } from "vitest"
import { cn } from "./utils"

describe("cn()", () => {
  it("合并类名", () => {
    expect(cn("p-4", "text-red-500")).toBe("p-4 text-red-500")
  })

  it("去重冲突的 Tailwind 类", () => {
    expect(cn("text-red-500", "text-blue-500")).toBe("text-blue-500")
  })

  it("支持条件类名", () => {
    expect(cn("p-4", { "bg-primary": true, "bg-muted": false }))
      .toBe("p-4 bg-primary")
  })
})
```

---

## Code Review Checklist

在提交 PR 前，确认以下项目：

### 离线优先

- [ ] 无外部 CDN 引用（`grep -rE 'https?://' src/` 无结果）
- [ ] 图片/字体本地存储在 `src/assets/`
- [ ] 无第三方 API 调用（数据通过 Tauri 后端获取）

### 图标和样式

- [ ] 无 emoji 字符（用 Lucide React 图标代替）
- [ ] 样式使用 Tailwind 类名（无内联 `style`）
- [ ] 动态类名使用 `cn()` 工具函数
- [ ] UI 组件来自 shadcn/ui（非手动创建）

### TypeScript

- [ ] 无 `any` 类型
- [ ] 所有函数参数/返回值有类型注解
- [ ] 使用字面量类型（非 enum）
- [ ] 代码通过 `tsc --noEmit` 检查

### 组件质量

- [ ] 图标按钮有 `aria-label`
- [ ] 表单字段有关联的 `<label>`
- [ ] 事件处理函数使用 `useCallback`
- [ ] 异步操作有 loading 状态和错误处理
- [ ] Tauri 事件监听有清理函数

### 代码组织

- [ ] 组件文件在正确的目录（`components/<feature>/`）
- [ ] 类型定义在 `types/` 并导出
- [ ] 工具函数在 `lib/` 并导出
- [ ] 文件命名符合规范（PascalCase 或 kebab-case）

---

## Linting and Formatting

### ESLint 配置（推荐）

```json
// .eslintrc.json
{
  "extends": [
    "eslint:recommended",
    "plugin:@typescript-eslint/recommended",
    "plugin:react-hooks/recommended"
  ],
  "rules": {
    "@typescript-eslint/no-explicit-any": "error",
    "react-hooks/exhaustive-deps": "warn",
    "no-console": ["warn", { "allow": ["error", "warn"] }]
  }
}
```

### Prettier 配置（推荐）

```json
// .prettierrc
{
  "semi": false,
  "singleQuote": false,
  "tabWidth": 2,
  "trailingComma": "all",
  "printWidth": 100
}
```

### 提交前检查脚本

```json
// package.json
{
  "scripts": {
    "lint": "eslint src --ext .ts,.tsx",
    "type-check": "tsc --noEmit",
    "format": "prettier --write src",
    "check-offline": "! grep -rE 'https?://' src/"
  }
}
```

---

## Common Mistakes

### 1. 引入外部字体

```css
/* ❌ 错误：Google Fonts CDN */
@import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600&display=swap');

/* ✅ 正确：本地字体 */
@font-face {
  font-family: 'Inter';
  src: url('/fonts/Inter-Regular.woff2') format('woff2');
  font-weight: 400;
}
```

### 2. 未清理 Tauri 事件监听

```typescript
// ❌ 错误：内存泄漏
useEffect(() => {
  listen("event", handler)
}, [])

// ✅ 正确：清理订阅
useEffect(() => {
  let unlisten: UnlistenFn | undefined
  async function init() {
    unlisten = await listen("event", handler)
  }
  init()
  return () => { unlisten?.() }
}, [])
```

### 3. 过度使用 useMemo

```typescript
// ❌ 错误：简单计算不需要 memo
const isDisabled = useMemo(() => !isEnabled, [isEnabled])

// ✅ 正确：直接计算
const isDisabled = !isEnabled
```

### 4. 未处理异步初始化的 null 状态

```typescript
// ❌ 错误：可能访问 null
const [data, setData] = useState<Data | null>(null)
return <div>{data.title}</div>

// ✅ 正确：提前返回 loading UI
if (!data) return <div>加载中...</div>
return <div>{data.title}</div>
```

---

## Summary

Kairos 前端质量标准的核心原则：

1. **离线优先** — 无外部依赖，本地打包
2. **类型安全** — 禁止 `any`，全类型覆盖
3. **一致性** — Tailwind + cn()，Lucide 图标，shadcn/ui 组件
4. **无障碍** — aria-label，语义化 HTML，label 关联
5. **可维护** — 清晰的目录结构，统一的命名规范

在编写代码时，始终优先考虑：**这段代码能否在离线环境下运行？是否有完整的类型覆盖？是否符合项目约定？**
