---
name: kairos-frontend
description: "Kairos project frontend conventions — React + TypeScript + Tailwind CSS v4 + shadcn/ui. Covers component patterns, Tauri IPC, Zustand stores, offline-first constraints, and shadcn/ui usage. Use when writing any frontend code in the Kairos project."
---

# Kairos Frontend Skill

> React + TypeScript + Tailwind CSS v4 + shadcn/ui conventions for the Kairos project.

---

## Tech Stack

| Layer | Technology | Version |
|-------|-----------|---------|
| Framework | React | 19+ |
| Language | TypeScript | 5.8+ |
| Build | Vite | 6+ |
| Styling | Tailwind CSS | v4 |
| Components | shadcn/ui | latest (new-york style, neutral) |
| Icons | lucide-react | latest |
| State | Zustand | latest |
| IPC | @tauri-apps/api v2 | invoke(), event() |

---

## Critical Constraint: Offline-First (MUST READ)

**This application is strictly offline.** The ONLY network calls allowed are:

1. **WebDAV sync** (in `sync/` Rust module only) — explicit user-initiated sync
2. **Dev server** (`localhost:5173`) — Vite dev server, local only

**FORBIDDEN in frontend code (`src/`):**
- ❌ `<script src="https://...">` or `<link href="https://...">` in `index.html`
- ❌ External CDN resources (fonts, icons, images) — ALL must be bundled locally
- ❌ `fetch()` / `XMLHttpRequest` to any external URL
- ❌ Google Fonts, Adobe Fonts, or any external font service
- ❌ Analytics, telemetry, crash reporting, update checking

**Verification:** After every frontend change, run:
```bash
grep -rn 'https\?://' src/  # Must return zero matches
```

---

## Directory Structure

```
src/
├── main.tsx                 # React entry point
├── App.tsx                  # Root component / router
├── index.css                # Tailwind v4 import + CSS variables
├── components/
│   ├── ui/                  # shadcn/ui primitives (NEVER hand-edit)
│   ├── pomodoro/            # Pomodoro timer components
│   ├── todo/                # TODO list components
│   ├── courses/             # Course schedule components
│   ├── exams/               # Exam countdown components
│   └── shared/              # Cross-feature reusable components
├── hooks/                   # Custom hooks (usePomodoro, useTasks, etc.)
├── stores/                  # Zustand stores (pomodoroStore, taskStore, etc.)
├── lib/                     # Utilities (cn(), invoke wrappers)
└── types/                   # TypeScript interfaces (shared between stores/components)
```

---

## Component Conventions

### Naming
- **PascalCase** for component files: `PomodoroTimer.tsx`
- **PascalCase** for component functions: `export function PomodoroTimer()`
- **camelCase** for hooks: `usePomodoroTimer()`
- **camelCase** for stores: `usePomodoroStore()`

### Structure
```tsx
// ✅ Correct: imports → types → component → exports
import { useState } from "react"
import { Button } from "@/components/ui/button"
import { cn } from "@/lib/utils"
import type { Task } from "@/types"

interface PomodoroTimerProps {
  task?: Task
  className?: string
}

export function PomodoroTimer({ task, className }: PomodoroTimerProps) {
  // 1. Hooks (state, store, effects)
  // 2. Derived state
  // 3. Event handlers
  // 4. Render
  
  return (
    <div className={cn("flex flex-col gap-4", className)}>
      {/* ... */}
    </div>
  )
}
```

### Props
- Always define `interface XxxProps` — never inline `{}` types
- Always accept `className?: string` on reusable components
- Use `type` for unions, `interface` for objects

---

## Tailwind CSS v4

### Key differences from v3
- **No `tailwind.config.ts`** — configuration via CSS using `@theme`
- **Import**: `@import "tailwindcss"` in `src/index.css` (DONE)
- **Dark mode**: Uses `dark:` prefix with CSS variables, no `darkMode: "class"` config
- **Arbitrary values**: Same as v3: `w-[320px]`, `bg-[#ff0000]`

### Conventions
- Use CSS variables defined in `index.css` for theming: `bg-background`, `text-foreground`, `border-border`
- Use shadcn/ui semantic tokens: `bg-primary`, `text-primary-foreground`, `bg-destructive`
- Responsive breakpoints: `sm:` (640), `md:` (768), `lg:` (1024)
- **No inline `style={{}}`** — use Tailwind classes or `className`

---

## shadcn/ui

### Adding components
```bash
npx shadcn@latest add <component-name>
```
Installed components go to `src/components/ui/`. **NEVER hand-edit these files** — they are owned by shadcn.

### Currently installed
- `button` — `src/components/ui/button.tsx`
- `card` — `src/components/ui/card.tsx`

### Pattern: Composing shadcn components
```tsx
import { Card, CardHeader, CardTitle, CardContent } from "@/components/ui/card"
import { Button } from "@/components/ui/button"

<Card>
  <CardHeader>
    <CardTitle>Feature Title</CardTitle>
  </CardHeader>
  <CardContent>
    {/* content */}
    <Button variant="default">Action</Button>
  </CardContent>
</Card>
```

### Theme
- Style: `new-york` (radix-nova)
- Base color: `neutral`
- CSS variables: `yes`
- Config file: `components.json`

---

## Lucide React Icons

```tsx
import { Timer, CheckSquare, BookOpen, Clock } from "lucide-react"

// Usage: always include size and optionally className
<Timer className="h-6 w-6 text-primary" />
<BookOpen size={20} className="text-muted-foreground" />
```

Prefer `h-N w-N` Tailwind classes for sizing over the `size` prop, to maintain consistency with the design system.

---

## Tauri IPC (Rust ↔ Frontend)

```tsx
import { invoke } from "@tauri-apps/api/core"

// Calling Rust commands from frontend
const tasks = await invoke<Task[]>("get_all_tasks", { 
  statusFilter: "todo",
  sortBy: "priority" 
})

// Event listening (for timer ticks, notifications)
import { listen } from "@tauri-apps/api/event"
const unlisten = await listen<PomodoroTick>("pomodoro-tick", (event) => {
  setRemaining(event.payload.remaining_seconds)
})
```

**Rules:**
- Always type the `invoke<T>` generic with the expected return type
- Always `try/catch` around `invoke()` — Rust commands return `Result`
- Use `@tauri-apps/api/core` for `invoke`, never `window.__TAURI__`

---

## Zustand Stores

```tsx
import { create } from "zustand"

interface TaskStore {
  tasks: Task[]
  filter: TaskFilter
  setTasks: (tasks: Task[]) => void
  setFilter: (filter: TaskFilter) => void
  fetchTasks: () => Promise<void>
}

export const useTaskStore = create<TaskStore>((set, get) => ({
  tasks: [],
  filter: { status: null, priority: null },
  setTasks: (tasks) => set({ tasks }),
  setFilter: (filter) => set({ filter }),
  fetchTasks: async () => {
    const { filter } = get()
    const tasks = await invoke<Task[]>("get_all_tasks", { ...filter })
    set({ tasks })
  },
}))
```

**Rules:**
- One store per feature domain (pomodoro, tasks, courses, exams, sync)
- Never use `any` — type everything
- Async actions call Tauri `invoke()` inside the store

---

## Prohibited Patterns

| ❌ Forbidden | ✅ Use Instead |
|-------------|---------------|
| `as any` / `@ts-expect-error` | Proper type definitions |
| `any` type | `unknown` + type guard, or define interface |
| `style={{ color: "red" }}` | Tailwind: `className="text-red-500"` |
| `<img src="https://...">` | Local bundled images only |
| `fetch("https://...")` | Tauri `invoke()` → Rust handler |
| `document.querySelector()` | React state + conditional rendering |
| `console.log()` in production code | Remove before commit |
| `setTimeout/setInterval` for timers | Rust-side timer via Tauri events |
| Inline `{}` prop types | `interface XxxProps { ... }` |

---

## Adding New Dependencies

All npm packages must be installed locally — no CDN, no external scripts:
```bash
npm install <package>         # Regular dependency
npm install -D <package>      # Dev dependency
```

**Never** add `<script>` tags to `index.html` for third-party code.

---

## Font Stack

Use system native fonts (no external font service):
```css
--font-sans: 'Inter', ui-sans-serif, system-ui, sans-serif;
--font-mono: 'JetBrains Mono', ui-monospace, monospace;
```

This is already configured in `src/index.css` via shadcn's theme variables.
