# Desktop Widget MVP Design

## Summary

Implement a single Tauri-managed floating widget window. The widget reuses existing Kairos data commands and adds one local widget configuration contract. It is not a native OS widget provider.

## Architecture

```
SQLite widget_config
    -> db::widget_config CRUD
    -> commands::widget_config IPC
    -> WidgetSettings UI
    -> Tauri window lifecycle
    -> WidgetApp render branch
    -> get_today_briefing / get_pomodoro_state / pomodoro controls
```

## Data Model

Add a singleton table `widget_config` through a migration.

Proposed columns:

- `id INTEGER PRIMARY KEY DEFAULT 1`
- `enabled INTEGER NOT NULL DEFAULT 0`
- `mode TEXT NOT NULL DEFAULT 'medium' CHECK(mode IN ('small', 'medium', 'large'))`
- `opacity REAL NOT NULL DEFAULT 0.92`
- `always_on_top INTEGER NOT NULL DEFAULT 1`
- `locked INTEGER NOT NULL DEFAULT 0`
- `x INTEGER`
- `y INTEGER`
- `width INTEGER NOT NULL DEFAULT 320`
- `height INTEGER NOT NULL DEFAULT 220`
- `created_at TEXT NOT NULL`
- `updated_at TEXT NOT NULL`

Rust model:

- `WidgetMode`: serialized as `"small" | "medium" | "large"` or stored as `String` with validation.
- `WidgetConfig`: mirrors the table.
- `UpdateWidgetConfigRequest`: partial update payload if following existing update command style, or full replacement if simpler.

Frontend type:

- `src/types/widget.ts`
- Must mirror Rust command payloads exactly.
- Comments required for persisted window coordinates, opacity range, and mode semantics.

## Backend Commands

Add `commands::widget`:

- `get_widget_config() -> Result<WidgetConfig, String>`
- `update_widget_config(config: WidgetConfig) -> Result<WidgetConfig, String>`
- `show_widget() -> Result<(), String>`
- `hide_widget() -> Result<(), String>`
- `open_main_window(target: Option<String>) -> Result<(), String>`

Command behavior:

- All database errors convert at command boundary using `.map_err(|e| e.to_string())`.
- For non-window-affecting fields, `update_widget_config` may persist first and then apply runtime changes.
- For `enabled = true`, validate config and create/apply the widget window before persisting the enabled state, so a failed window creation does not leave the database claiming that the widget is active.
- If window creation fails, return a clear string error and keep the previous persisted config.

## Window Lifecycle

Label: `widget`.

Creation strategy:

- On app setup, load `widget_config`.
- If `enabled`, create widget window after DB and managed state are ready.
- On settings enable, create the window if missing.
- On settings disable, close/hide the widget window.
- On widget close button, mark `enabled = false` only if user intentionally closes from widget UI; programmatic close during app shutdown should not mutate config.
- When the widget position changes, debounce persistence through a backend command or a shared `update_widget_config` path. Avoid writing on every mouse-move event.

Window properties:

- URL: `index.html?view=widget`
- decorations: false
- transparent: true
- resizable: false for MVP, unless mode switching needs resizing.
- skip taskbar: true.
- always on top: from config.
- inner size: from config / mode defaults.
- position: from config if present.

Capability update:

- Add `"widget"` to `src-tauri/capabilities/default.json` `windows`.
- Add any required core window permissions if Tauri denies frontend window calls beyond `core:default`.

## Frontend App Split

Current `App.tsx` renders the main application. Add a minimal entry switch:

```ts
const params = new URLSearchParams(window.location.search)
const isWidget = params.get("view") === "widget"
```

If `isWidget`, render `WidgetApp`; otherwise render current `App`.

Widget components:

- `src/components/widget/WidgetApp.tsx`
- `src/components/widget/WidgetFrame.tsx`
- `src/components/widget/SmallWidget.tsx`
- `src/components/widget/MediumWidget.tsx`
- `src/components/widget/LargeWidget.tsx`
- `src/components/settings/WidgetSettings.tsx`

Do not import internal components from unrelated feature modules unless they are moved to `shared/`. Reuse types and small formatting helpers where safe.

## Refresh Model

Initial load:

- `get_widget_config`
- `get_today_briefing`
- `get_pomodoro_state` for medium mode if full timer state is needed.

Events:

- `pomodoro-tick`: update timer display and completed sessions.
- `sync-finished`: refresh `get_today_briefing`.
- After widget settings update: re-read widget config and apply UI.

MVP fallback:

- Add a low-frequency refresh interval, for example 60 seconds, only for small / large mode time-sensitive "next" data.
- Keep interval cleanup in `useEffect`.

Avoid:

- Frontend recomputing course recurrence, exam sorting, overdue logic, or completed session counts.
- Multiple components parsing the same untyped event payload locally.

## Navigation From Widget

When a widget region is clicked:

- Call `open_main_window({ target })`.
- Backend shows/focuses the main window.
- Backend emits an event such as `main-navigate` with `{ target }`.
- Main App listens and updates its existing `active` state.

Targets:

- `today`
- `pomodoro`
- `todo`
- `courses`
- `exams`

Define shared frontend type for `MainNavigationTarget` or reuse an existing nav key type if one is extracted.

## Settings UX

Location: Kairos settings area / hub.

Controls:

- Toggle: enabled.
- Segmented control: mode small / medium / large.
- Toggle: always on top.
- Toggle: lock position.
- Slider or stepper: opacity.

Persist strategy:

- Explicit save is acceptable.
- If using immediate-save controls, each handler must persist before applying dependent window actions.

## Error States

Widget window:

- Loading state while reading config and briefing.
- Error state with a compact retry button.
- Empty state for no course / no task / no exam.

Settings:

- Display save errors without silently reverting persisted state.
- If widget window creation fails, show the backend error.

## Testing Strategy

Backend:

- Migration creates `widget_config` table and is idempotent.
- `get_or_create_widget_config` returns defaults.
- `update_widget_config` persists each field.
- Validation rejects invalid mode and out-of-range opacity if validation is implemented in Rust.
- Window helper pure pieces are tested where possible without creating real OS windows.

Frontend:

- TypeScript build verifies IPC types.
- Component smoke coverage through build/lint.
- Manual screenshot verification on Windows:
  - small / medium / large.
  - locked / unlocked.
  - opacity 0.75 / 1.0.
  - main navigation from widget.

Full validation:

- `cargo fmt --manifest-path "src-tauri/Cargo.toml"`
- `cargo clippy --manifest-path "src-tauri/Cargo.toml" --all-targets -- -D warnings`
- `cargo test --manifest-path "src-tauri/Cargo.toml"`
- `npm run lint`
- `npm run build`
- `jscpd . --threshold 10 --reporters console --format rust,typescript`

## Rollback

- Disabling widget removes all runtime impact; existing main application should continue to work.
- DB migration can stay in place as dormant config.
- If Tauri widget window creation is unstable on a platform, keep commands but hide settings behind platform / build guard in a follow-up.
