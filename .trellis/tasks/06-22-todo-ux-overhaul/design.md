# Design: Todo UX Overhaul

## Architecture Overview

```
TaskList.tsx
├── FAB (new)              — fixed bottom-right, opens Bottom Sheet
├── FilterChips (new)      — pill-shaped buttons with Popover
│   ├── Chip (new)         — single pill button
│   └── Popover (new)      — floating option list
├── EmptyState (refactored)— icon + text + CTA button, no card wrapper
├── TaskCard[] (refactored)— existing logic, minor styling
│   └── PriorityBadge       — adds color dot before label
├── BottomSheet (extended modal.tsx)
│   └── TaskForm            — existing form
└── Title (refactored)      — Large Title style
```

## Component Contracts

### 1. Modal → variant prop extension

```tsx
// modal.tsx
interface ModalProps {
  // ...existing props
  variant?: "center" | "bottom"  // default: "center"
}
```

**Bottom Sheet variant CSS contract:**
- `fixed inset-x-0 bottom-0 top-auto translate-x-0 translate-y-0`
- `rounded-t-2xl rounded-b-none`
- `max-h-[75vh] overflow-y-auto`
- Slide-up animation: `slide-in-from-bottom duration-300`
- Overlay stays the same (Radix Dialog.Overlay)

### 2. FAB component

```tsx
// src/components/shared/fab.tsx
interface FabProps {
  onClick: () => void
  "aria-label": string  // required for a11y
}
```

- Fixed position: `fixed right-4 bottom-20 z-40` (above Tab Bar's pb-16)
- Size: `h-14 w-14 rounded-full`
- Colors: `bg-primary text-primary-foreground`
- Shadow: `shadow-lg shadow-primary/25`
- Icon: `<Plus className="h-6 w-6" />`
- Transition: `hover:scale-105 active:scale-95 transition-transform`
- Chromium 110 compat: uses `var(--primary)`/`var(--primary-foreground)` — already has sRGB fallback

### 3. FilterChip + Popover

```tsx
// inline in TaskList.tsx or src/components/shared/filter-chip.tsx
interface ChipPopoverProps {
  label: string
  options: { value: string; label: string }[]
  value: string
  onChange: (value: string) => void
}
```

**Chip button:**
- `rounded-full px-3 py-1.5 text-xs font-medium`
- Active: `bg-primary/10 text-primary border-primary/30`
- Inactive: `bg-muted/60 text-muted-foreground border-border`
- Icon: `<ChevronDown className="h-3 w-3 ml-1" />`

**Popover:**
- `absolute top-full left-0 mt-1 z-30`
- `rounded-lg border border-border/60 bg-popover shadow-lg`
- `min-w-[120px] py-1`
- Each option: `px-3 py-2 text-sm hover:bg-muted rounded-md cursor-pointer`
- Selected option: `text-primary font-medium`
- Close on click outside (useEffect + ref)
- Close on option select

**Chromium 110 compat:**
- Chip/Popover use CSS variables (`--primary`, `--muted`, etc.) which have sRGB fallbacks
- No oklch() or color-mix() directly in these elements

### 4. EmptyState refactor

Remove `<AcrylicPanel>` wrapper. Replace with:

```tsx
<div className="flex flex-col items-center justify-center py-16 gap-4">
  <ListTodo className="h-12 w-12 text-muted-foreground/30" strokeWidth={1.5} />
  <div className="text-center space-y-1">
    <p className="text-sm font-medium text-muted-foreground">还没有待办任务</p>
    <p className="text-xs text-muted-foreground/60">点击下方按钮开始规划</p>
  </div>
  <Button size="sm" onClick={...}>
    <Plus className="h-4 w-4 mr-1" />添加任务
  </Button>
</div>
```

### 5. Bottom Sheet for TaskForm

In `TaskList.tsx`, replace `<Modal>` with new `<BottomSheet>` variant:

```tsx
<Modal
  open={showForm || editingTask !== null}
  variant="bottom"
  title={editingTask ? "编辑任务" : "新建任务"}
  // ...
>
  <TaskForm ... />
</Modal>
```

### 6. Priority color dots

In `PRIORITY_CONFIG`, add color dot:

```diff
- high: { label: "高", className: "bg-red-500/15 text-red-400 border-red-500/30" },
+ high: { label: "高", className: "bg-red-500/15 text-red-400 border-red-500/30", dotColor: "#f87171" },
```

Render priority badge with a small circle before the label:
```tsx
<span className="inline-block h-1.5 w-1.5 rounded-full mr-1" style={{ backgroundColor: dotColor }} />
```

## Data Flow

```
User taps FilterChip → Popover opens → select option → setStatusFilter/setPriorityFilter
  → useEffect triggers fetchTasks() → tasks state updates → re-render

User taps FAB → setShowForm(true) → Bottom Sheet opens with TaskForm
  → submit → handleCreate() → invoke create_task → close → fetchTasks()

User taps empty-state button → identical to FAB tap

User taps task card → setEditingTask(task) → Bottom Sheet opens with TaskForm pre-filled
  → submit → handleUpdate() → invoke update_task → close → fetchTasks()
```

## CSS / Compatibility Notes

- FAB uses `--primary`/`--primary-foreground` → already has sRGB fallback
- Chip uses `--primary`, `--muted`, `--border` → all covered
- Popover uses `--popover`, `--muted`, `--border` → all covered
- Bottom Sheet uses same `bg-card/95` scheme as modal → covered by existing `@supports not`
- EmptyState uses `text-muted-foreground/60` → covered
- Priority dots use inline `style={{ backgroundColor }}` → no CSS compat issue
- No new `oklch()` or `color-mix()` dependencies introduced

## Rollback Points

- FAB can be reverted by restoring `<Button>` in header
- Chips can be reverted to `<select>` elements
- Modal variant defaults to "center" — no breaking change
- Bottom Sheet is additive; existing Modal usage unaffected
