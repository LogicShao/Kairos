# 移除桌面小组件功能 — 执行计划

## 执行顺序（按依赖关系排列）

### 第 1 层：Rust 后端（无前端依赖）

- [ ] **1.1** 删除 `src-tauri/src/db/widget.rs`（含测试）
- [ ] **1.2** 修改 `src-tauri/src/db/mod.rs` — 删除 `#[cfg(not(target_os = "android"))] pub mod widget;`
- [ ] **1.3** 修改 `src-tauri/src/db/models.rs` — 删除 `WidgetConfig` 和 `UpdateWidgetConfigRequest` 结构体
- [ ] **1.4** 修改 `src-tauri/src/db/migrations.rs` — 删除 migration 7（`widget_config` 建表）；追加 migration 16 `DROP TABLE IF EXISTS widget_config`；更新测试断言 `test_migration_creates_tables` 表数 15→14
- [ ] **1.5** 删除 `src-tauri/src/commands/widget.rs`
- [ ] **1.6** 修改 `src-tauri/src/commands/mod.rs` — 删除 `#[cfg(not(target_os = "android"))] pub mod widget;`
- [ ] **1.7** 修改 `src-tauri/src/lib.rs` — 删除 `restore_widget_on_startup` 调用（行 213-216）+ invoke_handler 中 6 个 widget 命令注册（行 331-342）
- [ ] **验证** `cargo test` 通过

### 第 2 层：Capabilities

- [ ] **2.1** 修改 `src-tauri/capabilities/default.json` — `windows` 删除 `"widget"`；删除 `core:window:allow-start-dragging`

### 第 3 层：前端

- [ ] **3.1** 删除 `src/types/widget.ts`
- [ ] **3.2** 删除 `src/components/widget/` 目录（6 文件）
- [ ] **3.3** 删除 `src/components/settings/WidgetSettings.tsx`
- [ ] **3.4** 修改 `src/App.tsx` — 删除 WidgetSettings / WidgetApp import + 路由 + view=widget 分流 + main-navigate 监听
- [ ] **3.5** 修改 `src/components/kairos/KairosHub.tsx` — 删除 widget HUB 条目 + MonitorCog import
- [ ] **3.6** 修改 `src/components/shared/AppShell.tsx` — `isKairosArea` 数组删除 `"widget"`
- [ ] **验证** `npm run build` 通过

### 第 4 层：Spec 清理（Phase 3.3）

- [ ] **4.1** 修改 `.trellis/spec/frontend/quality-guidelines.md` — 删除「桌面小组件窗口」章节

### 最终验证

- [ ] **5.1** `cargo test` 全部通过
- [ ] **5.2** `npm run build`（tsc + vite）通过
- [ ] **5.3** `rg -i widget` 在 `src/` 和 `src-tauri/src/` 无残留
