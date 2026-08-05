# Android 返回键返回上一页面 — 技术设计

## 1. 背景机制（研究结论）

Tauri v2.11（本工程 `tauri 2.11.2` + `@tauri-apps/api 2.11.0`）已内置 Android 返回键支持：

- Kotlin 侧 `AppPlugin.kt`（tauri crate 自带，无需修改）注册了 `OnBackPressedCallback`：
  - **无 `back-button` 监听者**：webview `canGoBack()` → `goBack()`；否则 `finish()`（退出）——这是当前 bug 的直接原因。
  - **有监听者**：触发 `back-button` 事件给 JS，payload `{ canGoBack }`，默认行为被接管。
- JS API：`import { onBackButtonPress } from "@tauri-apps/api/app"`，返回 `PluginListener`（`unregister()` 可注销）。
- 权限：`core:app:default` 已含 `allow-register-listener` / `allow-remove-listener`，本项目 capabilities 已引用 `core:default`，**无需改权限**。
- 官方 `plugin:app|exit` 命令**不可用**（未注册进 ACL，`build.rs` 权限清单无此命令）。
- 退出方案：Rust 侧新增自研 `tauri::command`（自定义命令不走 ACL），调用 `AppHandle::exit(0)`（tauri 已处理 Android 退出流程：request_exit → ExitRequested → 失败回退 `std::process::exit`）。

## 2. 总体方案

纯前端导航栈 + 一个 Rust 退出命令，共两层改动：

```
Android 返回键
  → AppPlugin: 有监听者 → 触发 back-button 事件
  → 前端 handler:
      导航栈非空 → pop 上一页（setActive）
      导航栈为空 → invoke("exit_app") → AppHandle::exit(0) 退出
```

## 3. 前端设计

### 3.1 导航栈（`src/App.tsx`）

- `const [navStack, setNavStack] = useState<string[]>([])`。
- 新增 `navigate(key)` 替代直接 `setActive`（AppShell onNavigate 与各页面 onNavigate 均改传 `navigate`）：
  - `key === active` → 忽略。
  - `MAIN_PAGES.includes(key)`（today/pomodoro/todo/calendar/kairos）→ `setNavStack([])` 清栈（Tab 平级切换重置导航）。
  - 否则（子页面）→ `setNavStack(s => [...s, active])` 压栈。
  - 最后 `setActive(key)`。
- `goBack()`：栈非空 → `setActive(栈顶)` + `setNavStack(去掉栈顶)`。
- 注意：**不得在 setState updater 内调用其它 setState**（StrictMode dev 下 updater 双调用会重复压栈）；`navigate` 直接用事件闭包里的 `active`。

### 3.2 返回键监听（新文件 `src/hooks/use-android-back.ts`）

```ts
export function useAndroidBack(stackRef: RefObject<string[]>, goBack: () => void)
```

- 仅 Android 注册：`navigator.userAgent.includes("Android")`（同步判断，Tauri Android webview UA 恒含 Android；避免引入 `@tauri-apps/plugin-os` 依赖与权限）。
- `useEffect` 内 `onBackButtonPress(({}) => { stackRef.current.length > 0 ? goBack() : void invoke("exit_app") })`。
- StrictMode 双 mount 防护：async 注册 + `mounted` 标志 + cleanup `unregister()`，避免泄漏与重复注册。
- 栈用 ref 同步（`navStackRef.current = navStack` 随渲染更新），handler 里读最新值，避免闭包过期。

### 3.3 平台判断

`navigator.userAgent.includes("Android")` —— Tauri Android webview 的 UA 含 Android 标识；桌面端恒 false，注册逻辑直接跳过，桌面行为零改动。

## 4. Rust 设计

### 4.1 退出命令（`src-tauri/src/commands/mod.rs` 或新 `commands/app.rs`）

```rust
#[tauri::command]
pub fn exit_app(app: tauri::AppHandle) {
    app.exit(0);
}
```

- 自定义命令不走 ACL，无需 capabilities 改动。
- 在 `lib.rs` `invoke_handler` 注册（所有平台注册无副作用；仅 Android 前端调用）。

## 5. 边界与兼容

- **桌面端**：不注册监听 → AppPlugin 逻辑不涉及 → 行为与现状完全一致。
- **webview 内部 history**：不依赖默认 goBack；退出一律走 `exit_app`，行为确定。
- **StrictMode dev 双注册**：cleanup 正确 unregister，生产构建无双执行。
- **连续快速按返回**：handler 内同步读 ref 栈 + 同步 setState，React 批处理保证顺序正确。
- **从子页面直接切主 Tab**：navigate 清栈 → 返回键退出（Android 惯例，验收 A4）。

## 6. 验证方式

- 单元测试：导航栈的 push/pop 逻辑可提取为纯函数 `pushNav(stack, active, key)` / `popNav(stack)` 并加 Rust 侧？——前端无测试框架（package.json 无 vitest），改为：逻辑放在 App.tsx 内部 + `tsc` 类型检查 + Android 真机/模拟器手动验收（A1–A5）。
- `cargo test`（确认 Rust 改动无回归）+ `npm run build`。
- Android 手动验收：`npm run android:dev` 或安装 debug APK。
