# 专注功能配置完善

## Goal

为番茄钟添加可自定义工作时长/休息时长的设置入口，后端已支持 `update_pomodoro_config`，只需前端 UI。

## Requirements

- 番茄钟页面增加设置入口（齿轮图标按钮）
- 点击弹出配置 Modal/面板，可调整：
  - 工作时长（分钟，默认 25，范围 1-120）
  - 短休时长（分钟，默认 5，范围 1-30）
  - 长休时长（分钟，默认 15，范围 1-60）
  - 长休前番茄数（次，默认 4，范围 1-10）
- 保存时调用 `update_pomodoro_config` 后端命令
- 保存后立即生效（后端更新引擎配置）
- 配置持久化到 SQLite

## Technical Design

- 前端类型：`src/types/pomodoro.ts` 新增 `PomodoroConfig` 接口
- Tauri command：`update_pomodoro_config` 已注册在 `lib.rs`
- 后端配置：`src-tauri/src/db/pomodoro.rs` 已有 `update_config()` + `get_config()`
- 引擎更新：`commands/pomodoro.rs` 已处理 `eng.update_config(config)`

## Acceptance Criteria

- [ ] 番茄钟页面可见设置入口（齿轮图标）
- [ ] 设置面板可修改工作时长/短休/长休/长休间隔
- [ ] 保存后计时器使用新配置
- [ ] 配置在应用重启后保持
- [ ] 桌面端和移动端均可正常使用

## Out of Scope

- 提示音/震动自定义
- 每日目标番茄数
- 统计数据面板
