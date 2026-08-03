# Desktop Widget MVP Implementation Plan

## Pre-Implementation Gate

- Do not start implementation until this task is reviewed and `task.py start 07-05-desktop-widget-mvp` succeeds.
- Keep the existing uncommitted pomodoro sync fix separate from this feature. Do not include unrelated dirty files in this task's commits.

## Step 1: Backend Config Persistence

- Add `WidgetConfig` model and request type in `src-tauri/src/db/models.rs`.
- Add migration for `widget_config`.
- Add `src-tauri/src/db/widget.rs` with:
  - `get_or_create_widget_config`
  - `update_widget_config`
  - validation helper if using full replacement.
- Add DB tests for defaults, update, invalid values, and migration idempotency.

## Step 2: Backend Window Commands

- Add `src-tauri/src/commands/widget.rs`.
- Register module in `commands/mod.rs`.
- Register commands in `lib.rs` invoke handler.
- Implement:
  - `get_widget_config`
  - `update_widget_config`
  - `show_widget`
  - `hide_widget`
  - `open_main_window`
- Add window creation / update helper in a small backend module if command file gets too large.
- Ensure `widget` window creation uses config size, position, opacity, always-on-top, skip-taskbar, transparent, undecorated.
- Update `src-tauri/capabilities/default.json` to include `"widget"` and required window permissions.

## Step 3: Frontend Types And Entry Split

- Add `src/types/widget.ts`.
- Extract navigation target type if useful.
- Update `src/App.tsx` or the app entry to render `WidgetApp` when `view=widget`.
- Keep main app behavior unchanged when query param is absent.

## Step 4: Widget UI

- Add `src/components/widget/`.
- Implement `WidgetApp` state loading:
  - config
  - briefing
  - pomodoro state as needed.
- Implement modes:
  - `SmallWidget`
  - `MediumWidget`
  - `LargeWidget`
- Implement compact frame controls:
  - close / hide.
  - drag region when unlocked.
  - optional lock indicator.
- Use Lucide icons, no emoji.
- Use stable dimensions for each size; avoid text overlap.

## Step 5: Settings UI

- Add `WidgetSettings` and link it from existing settings / Kairos hub flow.
- Add controls:
  - enabled toggle.
  - mode segmented control.
  - always-on-top toggle.
  - locked toggle.
  - opacity slider.
- Persist settings through widget IPC commands.
- Apply settings immediately to an existing widget window.

## Step 6: Event Refresh And Navigation

- Listen to `pomodoro-tick` in widget medium mode.
- Listen to `sync-finished` and refresh briefing.
- Add main-window navigation event:
  - backend emits `main-navigate`.
  - main app listens and sets `active`.
- Add fallback interval refresh for small / large mode if next-course timing would otherwise stale.

## Step 7: Verification

- Run formatting:
  - `cargo fmt --manifest-path "src-tauri/Cargo.toml"` / `cargo fmt -- --check` (passed)
- Run backend checks:
  - `cargo clippy --manifest-path "src-tauri/Cargo.toml" --all-targets -- -D warnings` (passed)
  - `cargo test --manifest-path "src-tauri/Cargo.toml"` (passed, 145 tests)
- Run frontend checks:
  - `npm run lint` (passed)
  - `npx tsc -b` (passed)
  - `npm run build` (passed)
- Run duplication check:
  - `jscpd . --threshold 10 --reporters console --format rust,typescript` (passed, 8.64% total duplicated lines)
- Manual desktop verification:
  - enable / disable widget.
  - restart app and confirm restore.
  - switch small / medium / large.
  - start / pause pomodoro from widget.
  - trigger sync and confirm refresh.
  - click widget sections and confirm main window navigation.

Manual desktop verification has not been run in this environment.

## Review Notes

- If implementing in smaller commits, split into:
  - `feat(widget): persist desktop widget config`
  - `feat(widget): add floating widget window`
  - `feat(widget): render widget modes and settings`
- Do not push.
