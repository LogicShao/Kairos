# 通知能力调研

## 结论摘要

结论：**当前技术栈可以完成跨平台本地通知能力，且实现风险可控。**

更具体地说：

- **应用运行中即时通知**：可以直接做，难度低。
- **未来时间调度通知**：Tauri 官方通知插件已经暴露 `Schedule`、`pending()`、`cancel()` 等 API，说明它支持本地通知调度，而不只是立即弹窗。
- **Android 后台/进程被杀后的提醒**：不应继续依赖当前番茄钟线程，应该改为在开始计时时直接向系统注册一个定时通知。
- **TODO 截止提醒**：当前模型不足，因为 `Task.due_date` 只有日期，没有提醒时刻，首期如果纳入 TODO，必须先补字段或定义全局默认提醒时段。

## 当前仓库约束

### 已具备的基础

- Tauri v2 / Rust 版本满足官方插件要求：
- `@tauri-apps/api` 版本是 `^2.11.0`，见 [package.json](/D:/proj/Kairos/package.json:1)
- `tauri` 版本是 `2`，`rust-version` 是 `1.77.2`，见 [src-tauri/Cargo.toml](/D:/proj/Kairos/src-tauri/Cargo.toml:1)
- Android 平台脚手架已经存在，说明项目不是纯桌面假设。
- Rust 入口已经有长期运行线程，能承载“应用运行中重排/取消通知”的逻辑。

### 当前缺口

- 尚未接入通知插件：
- 前端没有 `@tauri-apps/plugin-notification`
- Rust 没有 `tauri-plugin-notification`
- 当前 capability 只有 `core:default`，通知相关权限策略还未落地。
- 业务数据没有统一的“提醒配置”模型。

## 官方能力映射

### 1. Tauri 官方通知插件是跨平台的

Tauri 官方通知插件文档列出的支持平台包括 `windows / linux / macos / android / ios`，并说明插件需要 Rust `1.77.2` 及以上。  
来源：Tauri Notifications 文档  
https://v2.tauri.app/plugin/notification/

这与 Kairos 当前技术栈兼容。

### 2. 不只是立即通知，还支持调度和查询

官方 JavaScript reference 暴露了这些核心能力：

- `sendNotification()`：发送通知
- `Schedule.at(...)` / `Schedule.every(...)` / `Schedule.interval(...)`：定义通知调度
- `pending()`：读取待发送通知列表
- `cancel()` / `cancelAll()`：取消待发送通知
- `active()` / `removeActive()`：管理已展示通知

来源：
- Tauri Notification JS Reference  
  https://v2.tauri.app/reference/javascript/notification/

这意味着第一期不需要自建提醒轮询框架，也不需要自己封装一层系统 API。

### 3. Windows 有一个实际限制

官方文档写明：Windows 侧“只对已安装应用正常工作，开发环境会显示 PowerShell 名称和图标”。  
来源：Tauri Notifications 文档  
https://v2.tauri.app/plugin/notification/

这意味着：

- 开发阶段可以先调通逻辑，但最终验收不能只看 `tauri dev`。
- Windows 通知至少要补一次安装包验证。

### 4. 开机自启是可选增强，不是第一阶段硬依赖

Tauri 也提供官方 `autostart` 插件，支持 `windows / linux / macos / android / ios`，可用于系统启动后自动拉起应用。  
来源：Tauri Autostart 文档  
https://v2.tauri.app/plugin/autostart/

但对 Kairos 来说：

- 如果通知完全走系统已注册的定时通知，第一阶段不一定需要自启。
- 如果未来要做“开机后自动重建未来 30 天提醒计划”或“托盘常驻 + 同步重排通知”，自启会变成有价值的增强项。

## 对 Kairos 各模块的可行性判断

### A. 番茄钟提醒

**可行，而且应该优先做。**

当前番茄钟是应用内线程每秒 tick。这个方案在桌面前台可用，但对 Android 后台/被杀场景天然不可靠。历史任务里也已经把“后台线程被杀”记为风险。  
见 [06-20-android-multiplatform/prd.md](/D:/proj/Kairos/.trellis/tasks/archive/2026-06/06-20-android-multiplatform/prd.md:1)

更合适的实现是：

- 开始一个番茄时，同时注册一个“工作结束”定时通知。
- 暂停/重置时取消对应通知。
- 阶段切换时重新注册下一条休息/工作通知。

这符合 KISS，也最贴近系统能力。

### B. 考试提醒

**可行，且现有模型基本够用。**

原因：

- `Exam.exam_datetime` 已经是 RFC3339 精确时间。
- 可以在创建/更新考试时，按“提前 1 天 / 1 小时 / 30 分钟”等策略生成一条或多条通知。

首期只要补一个配置来源即可：

- 全局默认提醒偏移量；或
- 每个考试独立提醒偏移量。

从复杂度看，建议先做“全局默认规则 + 单考试可关闭”。

### C. TODO 截止提醒

**当前不能无歧义落地，需要先补模型。**

原因：

- `Task.due_date` 只有 `YYYY-MM-DD`，没有具体时刻。
- 没有“提醒时间”“提前多久提醒”“是否提醒”字段。

如果强行做，会落入含糊设计：

- 到期当天 09:00 提醒？
- 前一晚 20:00 提醒？
- 截止当天 00:00 提醒？

这些都不是技术问题，而是产品语义未定义。

所以 TODO 有两个合理路径：

1. **MVP 排除 TODO 提醒**
- 先做番茄钟 + 考试。

2. **MVP 纳入 TODO，但先补模型**
- 至少新增 `remind_at` 或 `reminder_policy`。

在 YAGNI 角度下，更建议路径 1。

### D. 课程上课前提醒

**技术上能做，但不建议首期纳入。**

课程提醒需要从这些字段推导未来实例：

- `day_of_week`
- `start_time`
- `week_pattern`
- `semester_start_date`

这会牵涉：

- 周次规则解释
- 学期切换
- 批量重排未来课程实例
- 课程修改后的批量撤销与重建

这不是不能做，而是复杂度明显高于番茄钟和考试提醒。

## 对数据模型的建议

### 最小可落地方案

新增一个独立的通知计划表，而不是一开始把所有提醒字段塞进业务表。

建议方向：

- `notification_jobs`
- `id`
- `source_type`，例如 `pomodoro` / `exam` / `task`
- `source_sync_id`
- `platform_notification_id`
- `title`
- `body`
- `scheduled_at`
- `status`
- `created_at`
- `updated_at`

好处：

- 便于做取消/重建
- 便于和系统 `pending()` 结果对账
- 不污染现有业务实体

但如果只做最小 MVP，也可以不马上建完整 job 表，先采用：

- 番茄钟：运行态只维护当前 1 条通知 ID
- 考试：按考试 `sync_id + offset` 生成稳定通知 ID

这会更轻，但后续扩展性一般。

## 推荐的 MVP 范围

### 推荐方案 A（建议）

第一期只做：

- 番茄钟阶段结束通知
- 考试考前通知

第一期不做：

- TODO 截止提醒
- 课程上课前提醒
- 通知中心/历史通知箱
- 托盘常驻和开机自启

原因：

- 复用现有数据最充分
- 模型变更最少
- Android 价值最大
- 风险最可控

### 方案 B（更激进）

第一期同时做：

- 番茄钟
- 考试
- TODO

前提：

- 先补任务提醒时刻字段
- 明确任务提醒产品语义

这会显著放大设计面，不符合首轮 MVP 控制原则。

## 仍需验证的点

以下判断带有**推断**成分，需要后续实机验证：

- 虽然官方 API 暴露了调度能力，但 Windows / Linux 桌面端“应用完全退出后”的行为细节，仍应在真实安装包中验证。
- Linux 桌面环境差异较大，通知展示样式与行为可能受发行版/通知守护进程影响。
- Android 上通知权限、渠道（channel）与厂商系统省电策略，也需要真机验证。

这些不影响“技术栈可做”的结论，但会影响最终验收标准。

## 参考来源

- Tauri Notifications 文档  
  https://v2.tauri.app/plugin/notification/
- Tauri Notification JavaScript Reference  
  https://v2.tauri.app/reference/javascript/notification/
- Tauri Autostart 文档  
  https://v2.tauri.app/plugin/autostart/
