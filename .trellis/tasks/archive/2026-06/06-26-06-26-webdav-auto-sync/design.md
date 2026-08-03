# WebDAV 自动同步 — 技术设计

## 架构概览

本次不是单纯补一个后台线程，而是把“同步执行核心”“应用运行期间自动调度”“前端状态刷新”三件事统一起来：

```text
SyncSettings.tsx
  ├─ 保存最新 SyncConfig
  ├─ invoke("sync_now")
  └─ listen("sync-finished") -> 刷新 last_sync_at

commands/sync.rs
  ├─ get_sync_config / update_sync_config / test_sync_connection
  ├─ sync_now -> 调用 sync::execute_sync_command(...)
  └─ update_sync_config -> 控制 AutoSyncState 启停

sync/mod.rs
  ├─ execute_sync(...)             // 统一同步核心
  ├─ try_begin_sync / finish_sync  // 全局串行护栏
  ├─ auto_sync_loop(...)           // 后台调度
  └─ emit_sync_finished(...)       // 成功后通知前端

lib.rs
  └─ setup 时初始化 AutoSyncState，并按配置决定是否启动自动同步线程
```

## 设计目标

- 保持现有分层：`commands/` 负责 Tauri IPC，`sync/` 负责业务逻辑
- 手动同步与自动同步必须复用同一后端执行路径
- 手动与自动同步统一串行，避免两个同步实例并发
- SyncSettings 在当前打开时，能够感知自动同步成功后的 `last_sync_at` 更新
- 配置保存顺序明确，避免“用户刚改完配置，但后续动作仍拿旧值”
- 自动同步范围限定为应用进程存活期间，不扩展到系统级后台服务

## 核心决策

### 1. 同步核心下沉到 `sync/` 层

当前 `sync_now` 的主体逻辑在 `commands/sync.rs` 中，不适合作为自动同步线程直接复用。  
本次将同步主流程下沉到 `src-tauri/src/sync/mod.rs`，提供类似以下入口：

```rust
pub fn execute_sync(conn: &mut Connection) -> Result<SyncResult, String>
```

这个函数负责：

1. 读取 `sync_config`
2. 校验 `server_url`
3. 下载远端快照
4. LWW 合并
5. 条件上传 + ETag 冲突重试
6. 成功后写入 `last_sync_at`、`remote_etag`

`commands::sync::sync_now` 和自动同步线程都只调用这个公共函数。

### 2. 用单一运行态状态对象做串行护栏

新增 `AutoSyncState`，由 Tauri `manage` 持有：

```rust
pub struct AutoSyncState {
    pub enabled: Arc<AtomicBool>,
    pub running: Arc<AtomicBool>,
}
```

职责：

- `enabled`：表示是否允许自动同步线程继续调度
- `running`：表示当前是否已有同步实例在执行

`running` 是全局同步串行护栏，不仅用于“自动同步避免自重入”，也用于“手动同步和自动同步互斥”。

### 3. 手动/自动同步统一互斥语义

与旧版设计不同，本次不接受“自动同步与手动同步可并发，靠 WAL + LWW 兜底”的方案。  
原因：

- 这与 PRD 的“不会同时执行两次”冲突
- 两个同步都涉及远端下载/上传和本地 `last_sync_at` 更新，虽然理论上可收敛，但行为不稳定且难以向用户解释
- 前端已经有“同步中”按钮状态，用户心智天然是“一个同步任务在跑”

因此采用统一语义：

- 任意来源的同步开始前都先尝试获取 `running`
- 获取失败：
  - 自动同步：记录 `info!/warn!` 后跳过
  - 手动同步：返回可恢复错误，例如 `"Sync already in progress"`
- 同步结束时必须在 `finally`/RAII 风格路径中释放 `running`

### 4. 自动同步调度线程只负责调度，不承载业务细节

后台线程形态仍沿用当前项目已有的 `std::thread::spawn` 模式，而不是引入额外 runtime。
它是典型的 session-scoped autosync：应用进程在，就能调度；进程结束，就不再触发。

线程职责：

1. 启动时等待 5 秒
2. 检查 `enabled`
3. 用独立连接 `db::get_connection(&db_path)` 打开数据库
4. 调用统一同步入口
5. 从“完成时刻”开始等待 15 分钟
6. 等待期间周期性检查 `enabled`，便于及时停掉

说明：

- 线程不持有全局数据库锁，避免阻塞 UI
- 线程不直接操作前端 state
- 线程失败仅记录日志，不做弹窗或自动重试

这意味着：

- 应用关闭后不再自动同步
- 应用重新启动时，通过“启动后 5 秒补同步”覆盖离线期间的主要一致性需求
- 如果未来需要处理“系统休眠恢复”“前台恢复”补同步，应作为单独增强，而不是混入本次基础方案

### 5. 通过 Tauri 事件刷新 SyncSettings 的 `last_sync_at`

当前 SyncSettings：

- 首次加载时 `invoke("get_sync_config")`
- 手动同步成功后用 `new Date().toISOString()` 本地更新时间
- 没有监听后端事件

这无法满足“自动同步成功后页面随之刷新”的要求。  
因此本次将同步成功事件显式纳入方案：

- 后端在成功同步后 emit 一个事件，例如 `sync-finished`
- payload 至少包含最新的 `last_sync_at`，可选附带 `SyncResult`
- `SyncSettings.tsx` 在挂载时 `listen("sync-finished", ...)`
- 收到事件后以事件 payload 或重新 `get_sync_config` 的方式刷新页面状态

事件方式优于轮询：

- 与现有 Pomodoro 事件模式一致
- 页面打开时能实时刷新，页面关闭时无额外开销
- 不需要引入前端定时轮询

### 6. 明确前端配置保存顺序

当前 `SyncSettings.tsx` 中：

- `saveConfig()` 是异步 `invoke("update_sync_config")`，但调用方不等待
- `handleTestConnection()` 和 `handleSyncNow()` 先调 `saveConfig()`，随后立刻继续执行
- 切换 `autoSync` 用 `setAutoSync(!autoSync)` 再 `setTimeout(saveConfig, 0)`，依赖旧闭包值

这会导致计划即使后端完成，前端仍可能用旧配置触发测试连接/同步。  
因此本次设计要求：

- 抽出 `saveConfig(nextConfig): Promise<SyncConfig>` 风格的顺序化保存入口
- 测试连接、立即同步、自动同步开关切换都必须先 await 保存成功
- 前端本地 `lastSyncAt` 不再用“当前时间猜测”，而是采用后端持久化后的真实值

## 数据流

### 启动时自动同步

1. `lib.rs` setup 打开数据库
2. 读取 `sync_config`
3. 初始化 `AutoSyncState`
4. 若 `auto_sync = true && server_url != ""`，spawn 自动同步线程
5. 线程延迟 5 秒后尝试执行统一同步入口
6. 成功后写库并 emit `sync-finished`

### 运行时切换自动同步

1. SyncSettings 保存新配置
2. `commands::sync::update_sync_config` 先写库
3. 根据新旧配置决定：
   - 从不可运行 -> 可运行：开启 `enabled`，必要时启动线程
   - 从可运行 -> 不可运行：关闭 `enabled`
4. 已运行线程在下一个检查点自然退出

### 手动点击“立即同步”

1. SyncSettings 先保存最新配置
2. 调用 `invoke("sync_now")`
3. `commands::sync::sync_now` 尝试获取 `running`
4. 获取成功则执行统一同步入口
5. 成功后写库并 emit `sync-finished`
6. 前端监听到事件后刷新 `last_sync_at`

## 边界条件

### 空 `server_url`

- 启动时：不启动自动同步线程
- 运行时切换为空：关闭后续自动同步
- 手动同步：保持当前错误语义，返回 `"Server URL not configured"`

### 自动同步失败

- 记录日志
- 不弹窗
- 不改写 `last_sync_at`
- 下个周期继续尝试

### 手动同步遇到已有同步进行中

- 返回可恢复错误
- 前端维持当前错误展示区域即可，不新增复杂 UI

### 页面未打开时的自动同步成功

- 后端照常更新数据库
- 不要求后台缓存前端状态
- 页面下次打开时重新读取 `get_sync_config` 即可获得最新 `last_sync_at`

### 应用关闭、后台或系统挂起

- 应用关闭：自动同步停止，不再触发
- 最小化但进程仍存活：仍按当前线程调度策略运行
- 被操作系统挂起、睡眠或后台冻结：不保证按周期准时触发
- 恢复后立即补同步不属于本次范围；当前依赖“下次启动补同步”覆盖主要场景

## 文件变更边界

- `src-tauri/src/sync/mod.rs`
  - 新增统一同步入口
  - 新增自动同步调度与串行护栏辅助函数
- `src-tauri/src/commands/sync.rs`
  - 命令层改为薄包装
  - `update_sync_config` 负责配置写入后触发状态启停
- `src-tauri/src/lib.rs`
  - 初始化并 `manage` `AutoSyncState`
- `src/components/sync/SyncSettings.tsx`
  - 顺序化保存配置
  - 监听同步完成事件
  - 用后端真实 `last_sync_at` 刷新 UI
- `src/types/sync.ts`
  - 仅在需要为新事件 payload 定义前端类型时调整

## Tradeoffs

- **统一互斥 vs 允许手动/自动并发**：选统一互斥。行为更稳定，符合需求，也更容易测试和解释。
- **事件刷新 vs 前端轮询配置**：选事件刷新。与现有 Tauri 架构一致，成本更低，页面打开时体验更直接。
- **session-scoped autosync vs 系统级后台同步**：选 session-scoped autosync。当前需求足够、复杂度更低，也符合现有 Tauri 应用结构。
- **`std::thread::spawn` vs 异步 runtime**：选 `std::thread::spawn`。与当前项目定时任务模式一致，避免额外复杂度。
- **独立 DB 连接 vs 共享全局连接**：选独立连接。减少 UI 锁竞争，配合统一 `running` 互斥足够安全。

## Rollback

- 第一层回滚：保留同步核心下沉，但撤回自动同步线程与前端事件刷新
- 第二层回滚：若前端事件方案带来额外复杂度，保留数据库 `last_sync_at` 更新，仅保证页面重新进入后可见最新状态
- 第三层回滚：若统一互斥导致交互不理想，可保留自动同步互斥、单独评估手动同步 busy 提示，但不恢复并发双同步方案
