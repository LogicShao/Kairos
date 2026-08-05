# Android 返回键返回上一页面 — 执行计划

## 前置

- 任务基于当前 `main`（HEAD `923abed`）继续，工作区干净。
- 依赖：`@tauri-apps/api` 2.11.0（已含 `onBackButtonPress`）、`tauri` 2.11.2（已含 AppPlugin）。

## 执行顺序（按依赖关系）

### 第 1 层：Rust 退出命令

- [ ] **1.1** `src-tauri/src/commands/app.rs`（新建）— `#[tauri::command] pub fn exit_app(app: tauri::AppHandle)`，内部 `app.exit(0)`
- [ ] **1.2** `src-tauri/src/commands/mod.rs` — 追加 `pub mod app;`
- [ ] **1.3** `src-tauri/src/lib.rs` — `invoke_handler` 注册 `commands::app::exit_app`

### 第 2 层：前端导航栈

- [ ] **2.1** `src/App.tsx` — 提取 `MAIN_PAGES` 常量；新增 `navStack` state + `navigate()` + `goBack()`；所有 `onNavigate={setActive}` 改为 `onNavigate={navigate}`（AppShell、TodayPage、CalendarView、KairosHub、CourseSchedule、ExamList、LzuServicesPage、NotificationSettings、SemesterPhaseSettings、AiSettings、SyncSettings）
- [ ] **2.2** `src/hooks/use-android-back.ts`（新建）— `useAndroidBack(stackRef, goBack)`：UA 判断 Android → `onBackButtonPress` 注册；栈空时 `invoke("exit_app")`；StrictMode 双 mount 防护（mounted 标志 + cleanup unregister）
- [ ] **2.3** `src/App.tsx` — 接入 hook：`navStackRef` 同步 + 调用 `useAndroidBack`

### 第 3 层：验证

- [ ] **3.1** `npx tsc --noEmit` 通过（前端类型）
- [ ] **3.2** `cargo test` 通过（Rust 无回归）
- [ ] **3.3** `npm run build` 通过
- [ ] **3.4** Android 手动验收（真机/模拟器，`npm run android:dev`）：
  - A1 子页面返回 → 上一页
  - A2 主页面返回 → 退出到桌面
  - A3 子页面连续返回最终回主页面
  - A4 Tab 切换后返回 → 退出
  - A5 桌面端无副作用（Windows 启动抽查）

### 第 4 层：收尾

- [ ] **4.1** 按需更新 `.trellis/spec/frontend/` 或 `guides/`（若发现新模式：如 Tauri Android 返回键处理、StrictMode 双注册防护）
- [ ] **4.2** 提交：`fix(android): 返回键返回上一页面，无上级页面时退出应用`
- [ ] **4.3** `task.py archive` 归档

## 提交分组

1. `fix(android): 返回键返回上一页面，无上级页面时退出应用` — Rust 命令 + 前端导航栈 + hook

## 回滚

- 还原 `App.tsx` / `use-android-back.ts` / `commands/app.rs` / `mod.rs` / `lib.rs` 五处改动即可；无数据库/迁移影响。
- 退出命令失败不影响主流程（返回键退化为系统默认行为——退出）。
