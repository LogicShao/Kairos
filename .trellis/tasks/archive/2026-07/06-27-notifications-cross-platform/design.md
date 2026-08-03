# 跨平台通知能力设计

## 1. 目标

在不引入远程推送、后台守护进程或过度新模型的前提下，为 Kairos 增加一套可覆盖 `Windows / Linux / Android` 的本地系统通知能力，第一期只支持：

- 番茄钟阶段结束通知
- 考试考前提醒通知

设计目标遵循：

- **KISS**：优先接入 Tauri 官方通知插件，不自建系统通知抽象层
- **YAGNI**：第一期不处理 TODO / 课程 / 通知中心 / 单考试自定义提醒
- **DRY**：统一通知调度入口，避免番茄钟与考试各自实现一套通知逻辑
- **SOLID**：通知职责集中在 Rust 后端新模块，前端只做设置与展示

## 2. 当前状态与问题

### 2.1 当前番茄钟通知路径不正确

当前 [PomodoroTimer.tsx](/D:/proj/Kairos/src/components/pomodoro/PomodoroTimer.tsx:1) 在前端监听 `pomodoro-phase-change` 事件后直接调用 Web Notification API：

- 只对 WebView 前台场景有效
- 无法保证 Android 后台可靠性
- 无法统一三端行为
- 通知生命周期不与 Rust 命令绑定

这条路径必须下沉到 Rust。

### 2.2 当前考试数据足够做全局提醒

`Exam.exam_datetime` 已经是 RFC3339 时间，足以用于本地调度。  
问题不在数据精度，而在于：

- 还没有“默认提醒偏移量”的持久化配置
- 创建/更新/删除考试时没有同步重排通知
- 启动时没有全量重建未来考试通知

### 2.3 当前任务模型不适合做提醒

`Task.due_date` 只有 `YYYY-MM-DD`，没有提醒时刻或提醒策略。  
因此第一期明确排除 TODO 提醒。

## 3. 方案总览

### 3.1 核心思路

引入 `tauri-plugin-notification`，由 Rust 后端负责：

1. 统一请求通知权限
2. 发送即时通知
3. 调度未来通知
4. 取消已调度通知
5. 在应用启动时按数据库重建考试通知
6. 在番茄钟状态变化时调度或取消当前阶段结束通知

前端只负责：

1. 展示通知设置
2. 调用 Tauri command 更新配置
3. 不再直接创建浏览器通知

### 3.2 模块边界

新增后端模块，建议目录：

```text
src-tauri/src/
├── notifications/
│   ├── mod.rs
│   ├── ids.rs
│   ├── exam_scheduler.rs
│   └── pomodoro_scheduler.rs
```

职责拆分：

- `notifications/mod.rs`
  - Tauri 插件能力薄封装
  - 通用发送 / 调度 / 取消方法
  - 启动时重建入口
- `notifications/ids.rs`
  - 统一计算稳定通知 ID
- `notifications/exam_scheduler.rs`
  - 从考试数据计算需要的通知计划
  - 创建 / 更新 / 删除考试时重排
- `notifications/pomodoro_scheduler.rs`
  - 根据当前番茄阶段和剩余秒数注册或取消通知

不在前端封装系统通知逻辑，不在 `commands/exams.rs` 或 `commands/pomodoro.rs` 中堆大量细节。

## 4. 数据模型设计

### 4.1 第一阶段不新增 `notification_jobs` 表

不建议第一期就引入持久化通知作业表，原因：

- MVP 范围仅覆盖番茄钟与考试
- 考试通知可以从 `exams + 全局设置` 重新计算
- 番茄钟同一时刻只需要维护当前阶段的 1 条通知
- 额外表会带来迁移、同步语义、对账和清理逻辑

因此第一期采用：

- **考试通知**：启动时按数据库重新构建
- **番茄钟通知**：运行态动态维护

### 4.2 需要新增的最小配置表/字段

建议新增一个全局通知配置表，例如：

```sql
CREATE TABLE IF NOT EXISTS notification_config (
    id INTEGER PRIMARY KEY DEFAULT 1,
    enabled INTEGER NOT NULL DEFAULT 1,
    exam_offsets_json TEXT NOT NULL DEFAULT '[1440,60]',
    android_channel_created INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
```

说明：

- `enabled`
  - 全局总开关
- `exam_offsets_json`
  - 单位为分钟
  - 例如 `[1440,60]` 表示提前 1 天、提前 1 小时
- `android_channel_created`
  - 可选
  - 若插件/API 需要显式创建 Android channel，可用于幂等保护

第一期**不**改 `exams` 表，不为每个考试追加单独提醒字段。

### 4.3 为什么用 JSON 存 offset

因为第一期只要支持全局统一规则，且偏移列表天然是数组。  
相比新建一张 `notification_offsets` 子表：

- 迁移更轻
- 读取更简单
- 配置 UI 更直接

这符合 YAGNI。

## 5. 通知 ID 设计

### 5.1 需求

通知 ID 需要满足：

- 同一业务对象 + 同一偏移量能稳定重算
- 创建 / 更新 / 删除时可以准确 cancel
- 不依赖系统返回的临时 ID

### 5.2 方案

统一通过稳定字符串生成整数通知 ID：

```text
exam:{sync_id}:{offset_minutes}
pomodoro:{phase}:{target_epoch_seconds}
```

再通过一个简单稳定哈希映射到 `u32/i32`。

建议用 Rust 标准哈希或轻量哈希实现一个内部 helper；不要把这个逻辑散落在命令层。

### 5.3 取舍

优点：

- 无需持久化通知 ID
- 启动后可按同样规则重建 / 取消
- 跨平台一致

风险：

- 理论上存在哈希碰撞

缓解：

- ID 输入空间很小且可控
- 首期业务量低
- 若后续扩展到课程 / TODO / 更多交互，再升级到持久化 job 表

## 6. 数据流设计

### 6.1 番茄钟

#### 启动

1. 前端调用 `start_pomodoro`
2. Rust `PomodoroEngine` 进入 running
3. Rust 根据当前 `phase + remaining_seconds` 计算目标结束时间
4. Rust 调用通知调度器注册 1 条“阶段结束”通知

#### 暂停

1. 前端调用 `pause_pomodoro`
2. Rust 暂停计时
3. Rust 取消当前番茄通知

#### 重置

1. 前端调用 `reset_pomodoro`
2. Rust 重置状态
3. Rust 取消当前番茄通知

#### 阶段切换

1. tick 线程检测到 phase change
2. Rust 发出前端事件 `pomodoro-phase-change`
3. Rust 同时立即调度下一阶段结束通知
4. 可选：在切换瞬间立即发一条即时通知

### 6.2 考试

#### 启动重建

1. app setup 完成数据库初始化
2. 读取 `notification_config`
3. 查询全部未来考试
4. 对每个考试按 `exam_offsets_json` 生成若干通知计划
5. 过滤掉已过期计划
6. 调度剩余通知

#### 创建考试

1. `create_exam`
2. 写库成功后读取新纪录
3. 为该考试调度所有 offset 通知

#### 更新考试

1. `update_exam`
2. 更新前先读取旧纪录
3. 取消旧纪录对应的所有通知
4. 更新数据库
5. 为新纪录重新调度所有通知

#### 删除考试

1. 读取待删除考试
2. 取消所有相关通知
3. 软删除数据库记录

### 6.3 设置变更

#### 修改全局通知设置

1. 前端更新 `notification_config`
2. Rust 写库成功
3. Rust 取消全部未来考试通知
4. Rust 按新 offset 全量重建未来考试通知

这是首期最简单可靠的方式。

## 7. 命令与接口设计

### 7.1 新增后端 commands

建议新增：

- `get_notification_config`
- `update_notification_config`
- `request_notification_permission`
- `rebuild_exam_notifications`

其中：

- `get_notification_config / update_notification_config`
  - 供设置页读写全局开关和考试 offset
- `request_notification_permission`
  - 前端在设置页或首次开启时主动触发
- `rebuild_exam_notifications`
  - 调试 / 修复工具，也可供设置保存后显式调用

### 7.2 修改现有 commands

需要修改：

- `start_pomodoro`
- `pause_pomodoro`
- `reset_pomodoro`
- `create_exam`
- `update_exam`
- `delete_exam`

原则：

- command 仍然只负责“业务动作 + 调度器调用”
- 调度细节不内联到 command 内部

## 8. 前端设计

### 8.1 番茄钟页面

移除 [PomodoroTimer.tsx](/D:/proj/Kairos/src/components/pomodoro/PomodoroTimer.tsx:1) 里直接使用 Web Notification API 的逻辑：

- 删除 `Notification.requestPermission()`
- 删除 `new Notification(...)`

前端只保留：

- 状态展示
- 调用 `invoke("start_pomodoro")` / `pause_pomodoro` / `reset_pomodoro`

### 8.2 通知设置 UI

第一期建议在设置页或同步设置相邻区域新增“通知设置”块，最小字段：

- 总开关
- 考试提醒偏移列表
  - 例如固定选项：
  - 提前 1 天
  - 提前 1 小时

不做自由文本输入，不做复杂 rule builder。

### 8.3 考试列表

第一期不在单考试表单中加入提醒字段。  
理由：

- 已确认使用全局统一规则
- 避免前端类型和同步语义过早扩张

## 9. 平台差异与边界

### 9.1 Windows

根据官方文档，Windows 通知应在安装包环境验证。  
因此验收不能只依赖 `tauri dev`。

### 9.2 Linux

Linux 桌面环境差异较大，通知样式可能不一致。  
第一期只要求：

- 能调度
- 能展示
- 标题正文正确

不要求完全一致的 UI 呈现。

### 9.3 Android

Android 是本期重点平台：

- 番茄钟不能继续依赖前端 Web Notification
- 需要确保后台/锁屏时通知仍可按计划触发
- 若插件要求渠道（channel），应在初始化阶段幂等创建

第一期不做：

- 前台服务
- 持续常驻通知
- 厂商省电策略适配说明页

## 10. 权限与配置

### 10.1 Tauri 插件接入

需要增加：

- `package.json` 前端依赖
- `src-tauri/Cargo.toml` Rust 依赖
- `src-tauri/src/lib.rs` 中注册插件

### 10.2 Capability

当前只有 `core:default`。  
通知插件文档说明，潜在危险命令默认是阻止的，因此需要在 capability 中补通知权限配置。

### 10.3 权限请求策略

建议：

- 不在应用启动第一屏强行弹权限
- 用户首次开启通知总开关或进入通知设置页时请求权限

原因：

- 降低无上下文权限打扰
- 用户更容易理解为什么需要通知权限

## 11. 测试与验证策略

### 11.1 后端单元测试

应覆盖：

- offset 过滤逻辑
- 稳定通知 ID 生成
- 考试更新时“先取消旧通知再注册新通知”的编排逻辑
- 番茄钟开始 / 暂停 / 重置时的通知生命周期

其中真正系统通知 API 可以通过窄封装 trait 或 helper 抽离，便于测试调度决策。

### 11.2 手工验证

#### Windows

- 安装包环境验证考试提醒
- 安装包环境验证番茄钟结束提醒

#### Linux

- 在当前目标桌面环境验证两类通知都能显示

#### Android

- 前台运行时番茄钟提醒
- 切后台后番茄钟提醒
- 考试未来提醒至少验证一条短偏移通知

## 12. 风险与取舍

### 风险 1：插件真实平台行为与文档细节存在差异

应对：

- 设计中把系统 API 封装在后端通知模块
- 优先让业务层不直接依赖平台细节

### 风险 2：全量重建考试通知效率问题

首期不构成主要风险。考试数量通常有限，全量重建简单可靠。

### 风险 3：后续扩展到更多提醒对象时复杂度上升

这是刻意接受的延后成本。  
若后续要加入 TODO / 课程 / 复杂通知交互，再升级到 `notification_jobs` 表。

## 13. 最终决策

第一期采用：

- Tauri 官方通知插件
- Rust 后端统一调度
- 全局通知配置表
- 考试全局统一提醒 offset
- 番茄钟运行态调度
- 启动时全量重建考试通知

第一期不采用：

- 前端 Web Notification 作为主路径
- 单考试提醒配置
- 通知作业持久化表
- TODO / 课程提醒
