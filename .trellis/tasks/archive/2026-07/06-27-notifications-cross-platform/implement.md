# 跨平台通知能力实施计划

## 1. 实施目标

基于已确认的 MVP 范围，实现一套覆盖 `Windows / Linux / Android` 的本地通知能力，包含：

- 番茄钟阶段结束通知
- 考试考前提醒通知
- 全局通知配置

## 2. 分阶段执行

### Phase A：基础接入

1. 接入 `tauri-plugin-notification`
2. 在 `src-tauri/src/lib.rs` 注册通知插件
3. 补 capability / permission 配置
4. 前端移除 `PomodoroTimer.tsx` 中的 Web Notification 逻辑

完成标准：

- 项目可正常编译
- 前后端通知依赖已连通

### Phase B：全局通知配置

1. 为 `notification_config` 增加 migration
2. 在 `db/models.rs` 定义配置结构
3. 新增 `db/notifications.rs`（或等价模块）负责 CRUD
4. 新增 `commands/notifications.rs`
5. 在前端增加类型定义和最小设置 UI

完成标准：

- 可读取通知总开关
- 可保存考试默认提醒 offset

### Phase C：考试通知调度

1. 新增 `notifications/ids.rs`
2. 新增 `notifications/exam_scheduler.rs`
3. 在 app setup 时重建未来考试通知
4. 在 `create_exam / update_exam / delete_exam` 后接入调度器
5. 设置更新后触发全量重建

完成标准：

- 新建考试会产生对应定时提醒
- 修改考试时间会取消旧提醒并生成新提醒
- 删除考试会取消关联提醒

### Phase D：番茄钟通知调度

1. 新增 `notifications/pomodoro_scheduler.rs`
2. 修改 `start_pomodoro / pause_pomodoro / reset_pomodoro`
3. 在 tick loop 检测 phase change 时重排下一阶段提醒
4. 视实现简单度决定是否在 phase change 时追加即时通知

完成标准：

- 开始番茄后会注册当前阶段结束提醒
- 暂停/重置会取消提醒
- 阶段切换后下一阶段提醒自动重建

### Phase E：验证与收尾

1. 补 Rust 单元测试
2. 跑 lint / type-check / Rust test
3. 做三平台手工验证记录
4. 评估是否需要 spec 更新

## 3. 具体文件清单

预期新增/修改：

- `package.json`
- `src-tauri/Cargo.toml`
- `src-tauri/src/lib.rs`
- `src-tauri/capabilities/default.json` 或新增通知 capability 文件
- `src-tauri/src/db/migrations.rs`
- `src-tauri/src/db/models.rs`
- `src-tauri/src/db/mod.rs`
- `src-tauri/src/db/notifications.rs` 或等价文件
- `src-tauri/src/commands/mod.rs`
- `src-tauri/src/commands/notifications.rs`
- `src-tauri/src/commands/exams.rs`
- `src-tauri/src/commands/pomodoro.rs`
- `src-tauri/src/notifications/mod.rs`
- `src-tauri/src/notifications/ids.rs`
- `src-tauri/src/notifications/exam_scheduler.rs`
- `src-tauri/src/notifications/pomodoro_scheduler.rs`
- `src/components/pomodoro/PomodoroTimer.tsx`
- `src/types/` 下新增通知配置类型
- 现有设置页或新增通知设置组件

## 4. 校验命令

前端：

```powershell
npm run lint
npx tsc --noEmit
```

Rust：

```powershell
cargo test --manifest-path "src-tauri/Cargo.toml"
cargo check --manifest-path "src-tauri/Cargo.toml"
```

若项目已有统一命令，可在执行时替换为项目标准命令。

## 5. 关键检查点

### 检查点 1：插件接入后是否破坏现有构建

如果 `cargo check` 或 Android 构建受影响，先停止后续功能开发，优先解决依赖与 capability 问题。

### 检查点 2：通知权限请求是否可控

必须确认不会在应用一启动就无条件弹权限框。

### 检查点 3：考试通知是否具备幂等性

多次启动应用、多次保存同一设置后，不应累积重复通知。

### 检查点 4：番茄钟通知是否与状态一致

暂停、重置、切换阶段时不得残留过期通知。

## 6. 回滚点

### 回滚点 A：插件接入失败

回滚到仅保留规划文档，不继续推进实现代码。

### 回滚点 B：Android 后台行为与预期严重不符

保留桌面端通知架构，Android 进入单独子任务处理；但这需要重新修订 PRD，不应静默降级。

### 回滚点 C：考试全局 offset 配置引起 UI/模型复杂度膨胀

先固定为内置默认值（例如 `1 天 + 1 小时`），把可配置 UI 延后到下一轮。

## 7. 风险控制策略

- 所有系统通知操作收敛到 Rust 后端一个模块
- 业务 command 只做编排，不直接写平台细节
- 考试提醒使用“取消并全量重建”保证一致性
- 番茄钟使用“单条当前阶段通知”保证生命周期可控

## 8. 完成定义

满足以下条件才允许 `task.py start` 后进入实现：

- `prd.md` 已锁定 MVP 范围、平台范围和考试提醒粒度
- `design.md` 已定义模块边界、数据流和最小数据模型
- `implement.md` 已定义执行顺序、验证命令和回滚点

满足以下条件才可视为功能完成：

- 三端通知能力可验证
- 前端不再依赖 Web Notification 作为主实现
- 番茄钟与考试提醒均由 Rust 后端统一调度
- lint / type-check / tests 通过
