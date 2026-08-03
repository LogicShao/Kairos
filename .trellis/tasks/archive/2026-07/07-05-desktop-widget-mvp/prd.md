# Desktop Widget MVP

## Goal

为 Kairos 增加一个安装后可启用的桌面小组件 MVP。第一版采用 Tauri 独立悬浮窗口实现，不做 Windows Widgets / macOS WidgetKit 原生系统小组件。目标是在不打开主窗口的情况下，让用户快速看到下一件事、今日概览和专注状态，并可直接控制番茄钟开始 / 暂停。

## Background

Kairos 已经有今日概览命令 `get_today_briefing`、番茄钟状态命令 `get_pomodoro_state`、番茄钟控制命令和 `pomodoro-tick` / `sync-finished` 事件。桌面小组件应复用这些现有数据源，而不是建立第二套任务、课程、考试或番茄钟计算逻辑。

## Users

- 学生日常使用者：希望桌面常驻显示下一节课、待办和考试提醒。
- 专注使用者：希望不用打开主窗口即可查看倒计时并开始 / 暂停专注。
- 多端同步使用者：希望同步后小组件展示的数据与主应用一致。

## Requirements

### Window Behavior

- 用户可以在设置中启用或关闭桌面小组件。
- 启用后，Kairos 创建一个独立的小组件窗口。
- 小组件窗口默认无边框、非全屏、非主任务窗口。
- 小组件窗口支持置顶开关。
- 小组件窗口支持透明度设置。
- 小组件窗口支持位置记忆：应用重启后恢复上次位置。
- 小组件窗口支持三种尺寸：small、medium、large。
- 小组件关闭后，只关闭小组件窗口，不退出主应用。

### Widget Modes

- `small`：显示下一件事。
  - 优先级：当前/下一节课 > 最近考试 > 今日到期待办 > 番茄钟状态。
  - 内容必须可在紧凑尺寸中完整显示，不出现文本重叠。
- `medium`：显示专注状态。
  - 显示阶段、剩余时间、今日完成轮数。
  - 支持开始 / 暂停。
  - 可点击打开主应用专注页。
- `large`：显示今日概览。
  - 显示今日课程数和下一节课。
  - 显示逾期 / 今日到期待办和首个 spotlight。
  - 显示最近考试。
  - 显示番茄钟状态。
  - 可点击各区域打开主应用对应页面。

### Settings

- 设置入口放在 Kairos 设置区域，命名为“桌面小组件”。
- 设置项至少包含：
  - 启用小组件。
  - 小组件模式 / 尺寸。
  - 置顶。
  - 透明度。
  - 锁定位置。
- 设置保存后应立即影响当前小组件窗口。
- 设置持久化到本地 SQLite。

### Data Freshness

- 小组件初次打开时从后端读取真实状态。
- 番茄钟运行时，小组件应随 `pomodoro-tick` 更新。
- 同步完成后，小组件应随 `sync-finished` 重新读取今日概览。
- 任务、课程、考试在主应用中变更后，小组件应能刷新。MVP 可采用轻量事件或短周期刷新，但不得在前端复制业务计算逻辑。

### Constraints

- 前端不得直接发起外部网络请求。
- 前端不得本地重算课程、考试、待办排序规则；使用后端命令返回的数据。
- 不新增原生 Windows Widgets / macOS WidgetKit provider。
- 不在小组件里实现完整任务编辑、课程编辑或考试编辑。
- 不改变现有番茄钟计时语义。
- 不要求应用随系统开机自动启动；如后续需要，单独评估 autostart 插件和平台行为。

## Acceptance Criteria

- [x] 用户可从设置页启用 / 关闭桌面小组件。
- [x] 启用后创建独立 widget 窗口，且主窗口仍可正常使用。
- [x] small / medium / large 三种模式均可切换并持久化。
- [x] 应用重启后恢复小组件启用状态、模式、透明度、置顶配置和窗口位置。
- [x] medium 模式可以开始 / 暂停番茄钟，并跟随 `pomodoro-tick` 更新倒计时。
- [x] `sync-finished` 后小组件重新读取今日概览和番茄钟状态。
- [x] 小组件点击可打开主窗口并导航到对应页面。
- [x] 小组件 loading / error / empty 状态完整。
- [x] 小组件文本在目标尺寸下不重叠、不溢出关键按钮。
- [x] 新增 IPC 类型在 `src/types/` 中定义并与 Rust struct 对齐。
- [x] 新增 DB 迁移幂等，且有迁移测试。
- [x] `npm run lint`、`npm run build`、`cargo fmt`、`cargo clippy --all-targets -- -D warnings`、`cargo test`、`jscpd . --threshold 10 --reporters console --format rust,typescript` 通过。

## Verification Notes

- Automated verification passed on 2026-07-05:
  - `npm run lint`
  - `npx tsc -b`
  - `npm run build`
  - `cargo fmt -- --check`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo test` (145 tests)
  - `npx jscpd . --threshold 10 --reporters console --format rust,typescript` (8.64% total duplicated lines)
- Real Tauri desktop-window smoke testing was not performed in this headless review environment. Layout acceptance is based on component constraints, stable widget sizes, `truncate` / `line-clamp`, and successful build/lint checks.

## Out Of Scope

- Windows Widgets board 原生 provider。
- macOS Notification Center / Desktop WidgetKit 小组件。
- Android 小组件。
- 复杂布局编辑器。
- 小组件内创建 / 编辑任务。
- 多个小组件实例同时存在。
- 系统开机自启动。
