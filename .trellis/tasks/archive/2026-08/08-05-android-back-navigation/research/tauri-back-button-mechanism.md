# 研究：Tauri v2 Android 返回键处理机制

来源：tauri PR #14133（`feat(core): back button event on Android`，closes #8142）、tauri issue #8142/#14754、本地 tauri-2.11.2 crate 源码。

## 官方机制（tauri ≥ 2.11，本工程版本已含）

1. **Kotlin 侧 `AppPlugin.kt`**（tauri crate 自带，`crates/tauri/mobile/android/src/main/java/app/tauri/AppPlugin.kt`）注册 `OnBackPressedCallback`：
   - **无 `back-button` 监听者** → webview `canGoBack()` ? `goBack()` : `activity.finish()`（退出）。
   - **有监听者** → 触发 `back-button` 事件，payload `{ canGoBack: boolean }`，默认行为完全被接管（不会自动 goBack/退出）。
2. **JS API**：`import { onBackButtonPress } from "@tauri-apps/api/app"` → 内部 `addPluginListener('app', 'back-button', handler)`，返回 `PluginListener`（可 `unregister()`）。
3. **权限**：`core:app:default` 包含 `allow-register-listener` / `allow-remove-listener`；capabilities 引用 `core:default` 即放行，无需改动。
4. **官方 `plugin:app|exit` 命令不可用**：Kotlin 侧 `@Command fun exit(invoke)` 存在（`activity.finish()`），但 tauri `build.rs` 的权限清单未注册该命令 → JS invoke 会被 ACL 拒绝。
5. **Rust 侧退出**：`AppHandle::exit(code)` → `runtime_handle.request_exit`，失败回退 `std::process::exit(code)`（`crates/tauri/src/app.rs:574`）。

## 结论（对 Kairos 的适用性）

- Kairos 是 state 导航 SPA，webview 无 history → 默认行为恒为 `finish()`，即"按返回直接退出"的根因。
- 方案：始终注册 `onBackButtonPress`（接管默认行为）→ 导航栈非空则 pop；栈空则调用**自研 Rust 命令** `exit_app`（`app.exit(0)`，自定义命令不走 ACL）。
- 平台判断用 `navigator.userAgent.includes("Android")`（Tauri Android webview UA 恒含 Android，同步零依赖）。

## 备选方案（未采用）

- **MainActivity.kt 重写 `onBackPressed` + `evaluateJavascript` 同步回调**（issue #8142 社区方案）：可work但需 Kotlin 改动，且与官方 AppPlugin 回调重复/冲突。
- **根页面时 `unregister()` 恢复默认行为**：省 Rust 命令，但依赖 webview 无历史（dev HMR 可能有历史 → goBack 而非退出），行为不确定。
- **`plugin:app|exit`**：ACL 拒绝，需上游支持，不可行。
