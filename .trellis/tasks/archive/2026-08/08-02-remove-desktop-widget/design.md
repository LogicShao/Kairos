# 移除桌面小组件功能 — 技术设计

## 背景与目标

Kairos 桌面小组件依赖 Tauri 桌面上多窗口悬浮窗。Android 端无同类能力（原生 App Widget 需 Kotlin `RemoteViews`，不支持 WebView），Rust 侧已用 `#[cfg(not(target_os = "android"))]` 剔除命令，但前端入口未同步，导致 Android 报 `Command get_widget_config not found`。用户决定整体移除该功能。

## 删除范围（已核对全部引用）

### 后端 Rust

| 文件 | 处理 |
|---|---|
| `src-tauri/src/commands/widget.rs` | 整文件删除 |
| `src-tauri/src/commands/mod.rs` | 删除 `pub mod widget;`（含 `#[cfg]` 属性行） |
| `src-tauri/src/db/widget.rs` | 整文件删除（含 5 个测试） |
| `src-tauri/src/db/mod.rs` | 删除 `pub mod widget;` |
| `src-tauri/src/db/models.rs` | 删除 `WidgetConfig`、`UpdateWidgetConfigRequest` 结构体（行 25-60 附近） |
| `src-tauri/src/lib.rs` | 删除 `restore_widget_on_startup` 调用（行 213-216）、6 个 `#[cfg(not(android))]` widget 命令（行 331-342） |

### 数据库迁移

| 文件 | 处理 |
|---|---|
| `src-tauri/src/db/migrations.rs` | 删除 migration 7（`widget_config` 建表 + seed）。**为已升级用户**追加 migration 16 `DROP TABLE IF EXISTS widget_config`。 |

关键约束：删除旧迁移定义（v7）会导致已升级用户库中 `_migrations` 记录 v7 但新代码解析列表不从 v7 开始 → 会把现在 v16 的记录当新迁移执行。真实行为：
- `current_version` 取 `MAX(version)`。老用户库 MAX=15，新增 v16 执行 `DROP TABLE IF EXISTS`（幂等，表不存在时无副作用）。
- 新安装库：从头执行 v1-v16，v7 已删则不会创建 `widget_config` 表，v16 的 DROP 也自然无操作。
- 结论：**删除 v7 定义 + 追加 v16 DROP 是安全的**，需同步更新测试断言：
  - `test_migration_creates_tables`：表数 15 → 14
  - `test_migration_idempotent`：迁移记录数仍为 15 不变（15 条：1-6 + 8-16 = 15）
  - `test_migration_v3_tolerates_preexisting_columns`：计数仍为 15 不变（预置 2 条 + 执行 13 条 = 15）
  - `test_migration_v4_backfills_sync_metadata`：无表数断言，不改
  - `test_migration_v10_*`：无表数断言，不改

### 前端

| 文件 | 处理 |
|---|---|
| `src/components/widget/`（App/Large/Medium/Small/WidgetFrame/widget-format） | 整目录删除 |
| `src/components/settings/WidgetSettings.tsx` | 删除 |
| `src/types/widget.ts` | 删除 |
| `src/App.tsx` | 移除 `WidgetSettings`/`WidgetApp` import 与引用、`main-navigate` 事件监听、`view=widget` 分流、`MainNavigateEvent` type import |
| `src/components/kairos/KairosHub.tsx` | 移除「桌面小组件」HUB_ENTRIES 条目、`MonitorCog` import |
| `src/components/shared/AppShell.tsx` | `isKairosArea` 数组移除 `"widget"` |

### Capabilities

| 文件 | 处理 |
|---|---|
| `src-tauri/capabilities/default.json` | `windows: ["main", "widget"]` → `["main"]`；删除 `core:window:allow-start-dragging`（仅小组件拖动使用，已全量 grep 确认无其他引用） |

### Spec 同步（Phase 3.3）

| 文件 | 处理 |
|---|---|
| `.trellis/spec/frontend/quality-guidelines.md` | 删除「桌面小组件窗口」章节（功能已移除，避免过时规范误导） |

## 验证策略

1. Rust：`cargo test`（迁移测试 + 其余模块测试确认无引用）
2. 前端：`npm run build`（tsc 严格捕获类型残留）
3. 全局：`rg -i widget` 仅允许命中 `src-tauri/gen/android`（Tauri 生成物，含 `__wryActivityId` 等无关匹配）、`dist`、`node_modules`；源码目录必须清空。

## 风险与回滚

- 风险极低：纯删除，无行为迁移；`widget_config` 非同步表，无跨设备一致性影响。
- 回滚：`git checkout` 恢复删除文件即可；数据库 v16 已执行的 DROP 不可逆，但无数据价值（小组件配置纯本地、无业务数据）。

## 兼容性

- 桌面版升级：`widget_config` 表被 DROP，小组件入口消失，无残留 UI。
- Android 版升级：命令不再存在，前端入口消失，报错消除。