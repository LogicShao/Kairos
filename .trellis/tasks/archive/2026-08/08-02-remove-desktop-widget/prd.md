# 移除桌面小组件功能

## Goal

完全移除 Kairos 的桌面小组件（桌面悬浮窗口）功能，包括后端命令、数据库表、前端组件与入口。

## Background

- 桌面小组件依赖 Tauri 桌面端多窗口悬浮窗（`WebviewWindowBuilder`），Android 上无法实现同类功能（Android App Widget 需原生 `AppWidgetProvider` + `RemoteViews`，不支持 WebView）。
- Rust 侧已用 `#[cfg(not(target_os = "android"))]` 将全部小组件命令在 Android 编译剔除，但前端入口仍暴露，导致 Android 端进入「设置 → 桌面小组件」时报 `Command get_widget_config not found`。
- 用户确认该功能不再有意义，决定整体移除。

## Requirements

- 删除后端 `commands/widget.rs` 与 `db/widget.rs` 模块及全部 6 个 Tauri 命令。
- 删除前端 `components/widget/` 目录、`WidgetSettings.tsx`、`types/widget.ts`。
- 从 App 路由、KairosHub 入口、AppShell 中移除小组件引用。
- 数据库：删除 `widget_config` 表（含为已安装用户清理残留表的迁移）。
- Capabilities：移除 `widget` 窗口声明与 `core:window:allow-start-dragging` 权限（仅小组件使用）。

## Constraints

- 不得影响其他功能（番茄钟、任务、课程、考试、同步、LZU、AI 等）。
- 同步模块不涉及 widget_config，删除该表无副作用。
- 迁移编号：保留已有迁移历史，通过新增迁移清理旧库残留表。

## Acceptance Criteria

- [ ] `cargo test` 全部通过（含迁移测试断言更新）
- [ ] `npm run build`（tsc + vite）通过，无残留引用报错
- [ ] `rg -i "widget" src/ src-tauri/src/` 无任何残留（除 `_TEMP`、`dist`、`node_modules` 等忽略目录）
- [ ] Android 端不再出现 `Command get_widget_config not found`

## Out of Scope

- 不实现 Android 原生 App Widget（属独立功能，另行立项）。
