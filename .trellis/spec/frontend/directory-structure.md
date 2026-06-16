# Directory Structure

> How frontend code is organized in this project.

---

## Overview

Kairos 前端采用功能分层的目录结构：按功能模块（pomodoro/todo/courses/exams）组织组件，类型定义、工具函数、自定义 hooks 分别集中管理。这种结构在保持模块边界清晰的同时，避免了跨模块重复代码。

**关键原则**：
- **功能模块隔离**：`components/` 下按功能分目录（如 `pomodoro/`、`todo/`）
- **类型集中管理**：`types/` 目录按领域模型（task.ts、course.ts）组织类型定义
- **共享资源分离**：工具函数（`lib/`）、hooks（`hooks/`）、shadcn/ui 组件（`components/ui/`）独立存放
- **离线优先约束**：禁止引用外部 CDN，所有资源本地打包

---

## Directory Layout

```
src/
├── components/
│   ├── pomodoro/
│   │   └── PomodoroTimer.tsx        # 番茄钟计时器组件
│   ├── todo/
│   │   ├── TaskForm.tsx              # 任务表单
│   │   └── TaskList.tsx              # 任务列表
│   ├── courses/
│   │   └── CourseSchedule.tsx        # 课程表
│   ├── exams/
│   │   └── ExamList.tsx              # 考试列表
│   ├── sync/
│   │   └── SyncSettings.tsx          # WebDAV 同步设置
│   ├── shared/
│   │   └── acrylic-panel.tsx         # 跨模块共享组件（毛玻璃面板）
│   └── ui/                           # shadcn/ui 基础组件（自动生成，不手动编辑）
│       ├── button.tsx
│       └── card.tsx
├── hooks/
│   └── use-theme.ts                  # 自定义 hooks（主题切换）
├── lib/
│   └── utils.ts                      # 工具函数（cn() 样式合并）
├── types/
│   ├── task.ts                       # 任务领域类型
│   ├── pomodoro.ts                   # 番茄钟类型
│   ├── course.ts                     # 课程类型
│   ├── exam.ts                       # 考试类型
│   └── sync.ts                       # 同步配置类型
├── App.tsx                           # 根组件
└── main.tsx                          # 入口文件
```

---

## Module Organization

### 1. 功能组件（`components/<feature>/`）

**规则**：
- 每个功能模块一个子目录（如 `pomodoro/`、`todo/`）
- 文件名使用 PascalCase（`TaskForm.tsx`、`PomodoroTimer.tsx`）
- 导出的组件名必须与文件名一致

**示例**：
```tsx
// src/components/todo/TaskForm.tsx
export function TaskForm({ task, onSave, onCancel }: TaskFormProps) {
  // ...
}
```

### 2. 跨模块共享组件（`components/shared/`）

**规则**：
- 被 2+ 个功能模块使用的组件放入 `shared/`
- 文件名使用 kebab-case（`acrylic-panel.tsx`）
- 必须有明确的复用场景，避免过早抽象

**示例**：
```tsx
// src/components/shared/acrylic-panel.tsx
export function AcrylicPanel({ children, className }: AcrylicPanelProps) {
  // 毛玻璃效果面板，被多个功能模块使用
}
```

### 3. UI 基础组件（`components/ui/`）

**规则**：
- **只能通过 shadcn/ui CLI 生成，禁止手动创建**
- 文件名使用 kebab-case（`button.tsx`、`card.tsx`）
- 如需定制，修改生成后的代码（保留 Tailwind + CVA 模式）

**生成命令**：
```bash
npx shadcn@latest add button
```

### 4. 类型定义（`types/`）

**规则**：
- 按领域模型组织文件（`task.ts`、`pomodoro.ts`、`course.ts`）
- 使用字面量类型代替枚举（`type TaskStatus = "todo" | "in_progress" | "done"`）
- 导出所有类型，禁止内部私有类型

**示例**：
```typescript
// src/types/task.ts (见 L1-14)
export type TaskStatus = "todo" | "in_progress" | "done"
export type TaskPriority = "high" | "medium" | "low"

export interface Task {
  id: number
  title: string
  status: TaskStatus
  priority: TaskPriority
  // ...
}
```

### 5. 自定义 Hooks（`hooks/`）

**规则**：
- 文件名使用 kebab-case + `use-` 前缀（`use-theme.ts`）
- 导出的 hook 函数必须以 `use` 开头（`useTheme`）
- 每个文件导出一个主 hook，辅助函数可按需导出

**示例**：
```typescript
// src/hooks/use-theme.ts (见 L31-43)
export function useTheme() {
  const theme = useSyncExternalStore(subscribe, getSnapshot)
  const toggle = useCallback(() => { /* ... */ }, [theme])
  return { theme, toggle, setTheme }
}
```

### 6. 工具函数（`lib/`）

**规则**：
- 文件名使用 kebab-case（`utils.ts`）
- 每个函数独立导出（`export function`）
- 必须是纯函数（无副作用）

**示例**：
```typescript
// src/lib/utils.ts (见 L4-6)
export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}
```

---

## Naming Conventions

| 类型           | 规则                | 示例                          |
|----------------|---------------------|-------------------------------|
| 功能组件目录   | kebab-case          | `components/pomodoro/`        |
| 组件文件       | PascalCase          | `PomodoroTimer.tsx`           |
| 共享组件文件   | kebab-case          | `acrylic-panel.tsx`           |
| UI 组件文件    | kebab-case          | `button.tsx`, `card.tsx`      |
| 类型文件       | kebab-case          | `task.ts`, `pomodoro.ts`      |
| Hook 文件      | kebab-case + use-   | `use-theme.ts`                |
| 工具文件       | kebab-case          | `utils.ts`                    |

---

## Examples

### ✅ 正确示例

```
src/components/todo/TaskForm.tsx        # 功能组件
src/components/shared/acrylic-panel.tsx # 跨模块共享
src/components/ui/button.tsx            # shadcn/ui 组件
src/types/task.ts                       # 领域类型
src/hooks/use-theme.ts                  # 自定义 hook
src/lib/utils.ts                        # 工具函数
```

### ❌ 错误示例

```
src/components/taskForm.tsx             # ❌ 应放入 todo/ 子目录
src/components/ui/CustomButton.tsx      # ❌ ui/ 只能放 shadcn 生成的组件
src/types/TaskTypes.ts                  # ❌ 文件名应为 task.ts
src/hooks/theme.ts                      # ❌ 缺少 use- 前缀
src/utils/cn.ts                         # ❌ 应放入 lib/ 目录
```

---

## Common Mistakes

1. **过早抽象**：单个模块使用的组件就放入 `shared/`
   - **修复**：只有 2+ 模块实际使用时才移入 `shared/`

2. **手动创建 UI 组件**：在 `components/ui/` 手写组件
   - **修复**：使用 `npx shadcn@latest add <component>`

3. **类型文件过大**：把所有类型塞入一个 `types.ts`
   - **修复**：按领域模型拆分（task.ts、course.ts、pomodoro.ts）

4. **工具函数分散**：在组件内定义可复用的工具函数
   - **修复**：移入 `lib/utils.ts` 并导出
