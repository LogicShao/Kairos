# Implementation: Todo UX Overhaul

## Order

1. Extend Modal (Bottom Sheet variant) → foundational for other steps
2. Build FAB component
3. Build FilterChip + Popover
4. Refactor EmptyState
5. Refactor TaskList (wire everything together)
6. Visual polish (title, priority dots)
7. Verify on MuMu emulator

## Checklist

### Step 1: Extend Modal with Bottom Sheet variant

- [ ] 1.1 Add `variant` prop to `ModalProps` in `modal.tsx`
- [ ] 1.2 Add bottom-sheet CSS classes conditional on `variant === "bottom"`
- [ ] 1.3 Slide-up animation: `data-[state=open]:slide-in-from-bottom`
- [ ] 1.4 Verify existing center-mode Modal unchanged (KairosHub etc.)
- [ ] **Validate**: `tsc -b` compiles

### Step 2: FAB component

- [ ] 2.1 Create `src/components/shared/fab.tsx`
- [ ] 2.2 Fixed bottom-right with z-40
- [ ] 2.3 `rounded-full h-14 w-14` with Plus icon
- [ ] 2.4 hover/active scale transitions
- [ ] **Validate**: visually check position on MuMu

### Step 3: FilterChip + Popover

- [ ] 3.1 Create Chip button (pill shape, active/inactive states)
- [ ] 3.2 Build Popover dropdown (absolute positioning, click outside close)
- [ ] 3.3 Wire `statusFilter` / `priorityFilter` state
- [ ] 3.4 Remove old `<select>` elements and ChevronDown overrides
- [ ] **Validate**: filter state changes, popover opens/closes correctly

### Step 4: EmptyState refactor

- [ ] 4.1 Remove `<AcrylicPanel>` wrapper from empty state block
- [ ] 4.2 Add `ListTodo` icon (48px, muted)
- [ ] 4.3 Update copy text
- [ ] 4.4 Add "添加任务" button wired to `setShowForm(true)`
- [ ] **Validate**: empty state renders without card, button works

### Step 5: Wire everything in TaskList

- [ ] 5.1 Replace header `<Button>` with nothing (FAB takes over)
- [ ] 5.2 Replace `<Modal>` with `variant="bottom"` Modal
- [ ] 5.3 Add `<Fab>` component to JSX
- [ ] 5.4 Replace `<select>` section with `<FilterChip>` components
- [ ] 5.5 Wire empty-state CTA button
- [ ] **Validate**: full create/edit/delete/filter flow works

### Step 6: Visual polish

- [ ] 6.1 Enlarge title to `text-xl font-heading font-semibold`
- [ ] 6.2 Add color dot to `PRIORITY_CONFIG`
- [ ] 6.3 Render priority dot before label in task cards
- [ ] **Validate**: visual consistency check

### Step 7: Cross-browser verification

- [ ] 7.1 Verify on MuMu emulator (Chromium 110)
- [ ] 7.2 Verify on phone (modern WebView)
- [ ] 7.3 Toggle dark/light mode
- [ ] 7.4 Switch accent colors — FAB and active states follow
- [ ] **Validate**: all ACs from prd.md pass

## Risky Files

- `modal.tsx` — existing component used by multiple pages, must not break center variant
- `TaskList.tsx` — core page, large refactor
- `index.css` — avoid adding new oklch()/color-mix() patterns
