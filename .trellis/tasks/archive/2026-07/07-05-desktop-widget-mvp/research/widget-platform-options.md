# Widget Platform Options

## Decision

Desktop Widget MVP will use a Tauri-managed floating window, not native Windows Widgets or macOS WidgetKit.

## Sources

- Tauri v2 WebviewWindow JavaScript API: https://v2.tauri.app/reference/javascript/api/namespacewebviewwindow/
- Tauri v2 Window JavaScript API: https://v2.tauri.app/reference/javascript/api/namespacewindow/
- Tauri v2 Window State plugin: https://v2.tauri.app/plugin/window-state/
- Tauri v2 Autostart plugin: https://v2.tauri.app/plugin/autostart/
- Tauri v2 security capabilities: https://v2.tauri.app/security/capabilities/
- Microsoft Windows Widgets provider docs: https://learn.microsoft.com/en-us/windows/apps/develop/widgets/widget-providers
- Apple WidgetKit docs: https://developer.apple.com/documentation/widgetkit

## Local Findings

- `src-tauri/tauri.conf.json` currently defines only the main Kairos window.
- `src-tauri/capabilities/default.json` grants permissions only to the `main` window and currently includes `core:default` and `notification:default`.
- `src/App.tsx` uses local React state for page navigation, not a router.
- `src-tauri/src/commands/briefing.rs` already returns `TodayBriefingResponse`, which is a good data source for widget small and large modes.
- `src-tauri/src/commands/pomodoro.rs` provides state and control commands required by the medium widget.
- `pomodoro-tick` and `sync-finished` already exist as backend events.

## Implications

- A widget window should load the same frontend bundle with a view discriminator, such as `index.html?view=widget`, to avoid introducing routing solely for this feature.
- The frontend entry can branch on `URLSearchParams` and render either `App` or `WidgetApp`.
- Tauri capability config must include the widget window label, or frontend window APIs / events may be denied.
- The widget should reuse backend IPC payload types. Do not create local component-only copies of `TodayBriefingResponse`, `PomodoroState`, or widget config payloads.
- Native platform widgets remain a separate research and implementation track because they require provider-specific packaging and lifecycle contracts.

## Risks

- Transparent always-on-top windows can behave differently across Linux, Windows, and macOS. MVP acceptance should focus on Windows desktop first, with non-Windows treated as best effort unless explicitly tested.
- Persisting window position on every move can create noisy writes. Use debounced updates or plugin-backed window state.
- Skip-taskbar and always-on-top behavior may require platform-specific testing even when APIs are cross-platform.
