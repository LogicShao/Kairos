# WebDAV 自动同步

## Goal

基于已存在的 `auto_sync` 配置字段和 SyncSettings UI，补齐真正可用的“应用运行期间自动同步”能力，并把相关的前端状态刷新与配置持久化链路一并补全，让用户在开启后无需反复手动干预即可稳定同步。

## Background

- `sync_config.auto_sync` 字段已在 v2 协议中存在于数据库和 UI，但目前仅保存布尔值，没有任何调度逻辑
- `sync_now` 手动同步流程已完整可用（下载 -> LWW 合并 -> 上传 -> ETag 冲突重试）
- SyncSettings 页面当前只在首次加载时读取 `last_sync_at`，手动同步成功后也只是用前端本地时间做近似刷新
- SyncSettings 当前的保存配置调用是 fire-and-forget，切换 `auto_sync`、测试连接、立即同步 都可能没有严格使用到最新配置
- 当前目标是进程内 autosync，不包含应用关闭后的系统级后台同步

## Requirements

### R1: 启动时自动同步

- 应用启动后，若 `auto_sync = true` 且 `server_url` 非空，自动执行一次同步
- 启动同步应有适当延迟（3-5 秒），避免与应用初始化资源竞争
- 启动同步失败（如无网络）应静默处理，不弹出错误提示打扰用户
- 本次自动同步仅在应用进程存活期间生效；应用关闭后不触发同步

### R2: 定时自动同步

- 开启 `auto_sync` 后，每隔固定间隔自动执行一次同步
- 默认间隔为 15 分钟
- 间隔从上一次同步完成后开始计时，而不是按固定时钟点触发
- 关闭 `auto_sync` 或将 `server_url` 清空后，应停止后续自动同步
- 若应用只是最小化但进程仍存活，仍按正常策略参与自动同步；若进程退出或被系统挂起/杀掉，则不保证定时触发

### R3: 手动与自动同步共用同一执行护栏

- 手动同步和自动同步必须复用同一套后端同步核心逻辑
- 任意时刻只允许存在一个同步实例
- 若自动同步触发时已有同步进行中，本轮自动同步应跳过并记录日志
- 若用户手动点击“立即同步”时已有同步进行中，不应再启动第二个同步；前端应得到可恢复的忙碌反馈，而不是崩溃或卡死

### R4: 自动同步结果可被页面感知

- 成功同步后必须更新数据库中的 `last_sync_at`
- 当 SyncSettings 页面处于打开状态时，手动或自动同步成功后，页面展示的“上次同步”时间应刷新为持久化后的 `last_sync_at`
- 当页面重新进入或重新挂载时，也应能读取到最新的 `last_sync_at`

### R5: 同步相关操作必须使用最新配置

- 测试连接、立即同步、自动同步开关切换，必须基于用户刚刚编辑后的最新配置执行
- 配置保存必须先完成，再执行依赖该配置的后续动作
- `auto_sync` 开关切换不能依赖旧 state 闭包值

### R6: 节能与网络友好

- 不做网络可达性预检，直接尝试同步，由现有超时机制兜底
- 自动同步失败不立即重试，等待下一个周期
- 本次仍保持全量快照同步，不引入增量/差分协议

## Acceptance Criteria

- [ ] 开启 `auto_sync` 后，应用启动 5 秒内触发首次自动同步
- [ ] 开启 `auto_sync` 后，自动同步按“完成后再等待 15 分钟”的节奏执行
- [ ] 关闭 `auto_sync` 或清空 `server_url` 后，不再有新的自动同步发生
- [ ] 应用关闭后不要求继续自动同步；重新启动后会在延迟窗口后补做一次同步
- [ ] 自动同步失败（无网络/服务器不可达）只记录日志，不弹出前端错误提示
- [ ] 手动同步和自动同步复用同一后端同步核心，且不会并发执行两个同步实例
- [ ] SyncSettings 页面打开时，自动同步成功后“上次同步”时间会刷新为持久化后的 `last_sync_at`
- [ ] 用户修改 WebDAV 配置后立即点击“测试连接”或“立即同步”，后端使用的是最新配置
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml` 通过
- [ ] `npx tsc --noEmit` 通过
- [ ] `npm run lint` 通过
- [ ] `npm run build` 通过

## Out of Scope

- 事件驱动同步（如每次实体变更后立即同步）
- UI 中可配置自动同步间隔（本次仍硬编码 15 分钟）
- 系统托盘同步状态、全局同步进度条、通知中心
- 网络状态监听与自适应退避策略
- 差分/增量同步协议
- 应用最小化或后台时动态降频
- 应用从系统休眠、后台冻结、前台恢复时的补同步策略

## Definition of Done

- `cargo fmt --manifest-path src-tauri/Cargo.toml --check`
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- `cargo test --manifest-path src-tauri/Cargo.toml`
- `npx tsc --noEmit`
- `npm run lint`
- `npm run build`

## Technical Notes

- 受影响的后端入口主要是 `src-tauri/src/commands/sync.rs`、`src-tauri/src/sync/mod.rs`、`src-tauri/src/lib.rs`
- 受影响的前端入口至少包含 `src/components/sync/SyncSettings.tsx`
- 现有前端已经有 Tauri `listen` 用法（Pomodoro），可以复用同类事件模式处理同步完成后的状态刷新
