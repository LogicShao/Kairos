# WebDAV 自动同步 — 实施计划

## Preconditions

- 保留所有现有未提交变更，不进行 git 操作
- 本次只修 Trellis 计划，不在本步骤实现代码
- 后续真正进入实现阶段时，需同时覆盖 Rust 后端与 SyncSettings 前端配合项
- 保持既有分层：`commands/` 只做 Tauri IPC 包装，复用逻辑下沉到 `sync/` 业务层
- 当前目标限定为“应用运行期间自动同步”，不扩展到关闭应用后的系统级后台同步

## Steps

### 1. 统一需求边界

**目标**: 先消除 PRD 内部与 PRD/Design 之间的冲突

- 明确“手动同步与自动同步不会同时执行两次”是硬性需求，不接受并发双同步方案
- 明确“自动同步成功后前端可感知”不只是数据库更新，还包含页面打开时的刷新路径
- 明确“测试连接 / 立即同步 / 自动同步开关”都必须使用最新保存的配置
- 明确当前 autosync 是 session-scoped：启动后补同步 + 进程存活期间周期同步

**产物**:
- `prd.md` 的 Requirements / Acceptance Criteria 对齐

### 2. 统一技术路线

**目标**: 让 `design.md` 和需求完全一致

- 将同步核心明确下沉到 `sync/` 业务层
- 将 `running` 明确为手动同步与自动同步共享的全局串行护栏
- 将 SyncSettings 的事件监听和顺序化保存纳入设计，而不是默认前端“自然会跟上”
- 删除或改写任何“可并发双同步，靠 WAL + LWW 兜底”的描述

**产物**:
- `design.md` 的架构图、数据流、边界条件、tradeoff 一致

### 3. 统一执行步骤

**目标**: 让 `implement.md` 真正能指导后续实现

- 执行步骤必须同时覆盖：
  - Rust 同步核心下沉
  - AutoSyncState 与后台线程
  - `update_sync_config` 启停控制
  - SyncSettings 的配置保存顺序修复
  - SyncSettings 的同步完成事件刷新
- 验证步骤必须区分：
  - 后端单元测试
  - 前端类型/构建校验
  - 手动联调验证

**产物**:
- `implement.md` 的步骤、验证、review gate、rollback point 与 `design.md` 一致

### 4. 后端实现拆分建议

后续真正开始写代码时，建议按以下顺序执行：

1. 在 `sync/` 层提取统一同步入口
2. 引入 `AutoSyncState` 与统一 `running` 护栏
3. 在 `lib.rs` 接入启动自动同步
4. 在 `update_sync_config` 中接入运行时启停
5. 增加同步完成事件 emit

**检查点**:
- 手动 `sync_now` 行为与当前一致
- 自动同步与手动同步不会并发
- 自动同步成功后数据库 `last_sync_at` 正常更新
- 应用关闭后不要求继续同步；重新启动后会补做启动同步

### 5. 前端实现拆分建议

后续真正开始写代码时，建议按以下顺序执行：

1. 重构 `SyncSettings.tsx` 的 `saveConfig` 为可等待的顺序化保存入口
2. 修复“测试连接 / 立即同步 / autoSync 切换”对旧配置闭包的依赖
3. 监听后端 `sync-finished` 事件
4. 以事件 payload 或重新读取配置的方式刷新 `last_sync_at`
5. 手动同步成功后不再用前端本地时间猜测同步时间

**检查点**:
- 修改配置后立刻测试连接，使用的是最新配置
- 修改配置后立刻手动同步，使用的是最新配置
- 页面打开期间自动同步成功后，`last_sync_at` 会刷新

### 6. 测试策略

#### 6.1 后端测试

- 新增或调整 `sync/mod.rs` 的纯逻辑测试
- 验证 `running` 已占用时：
  - 自动同步会跳过
  - 手动同步返回可恢复错误
- 验证空 `server_url` 的启动/切换边界

#### 6.2 前端校验

- `npx tsc --noEmit`
- `npm run lint`
- `npm run build`

#### 6.3 手动联调

- 预置 `auto_sync = true` + 有效 `server_url`，确认启动约 5 秒内触发首次同步
- 打开 SyncSettings，等待自动同步成功，确认“上次同步”刷新
- 修改配置后立即点击“测试连接”，确认使用最新配置
- 修改配置后立即点击“立即同步”，确认使用最新配置
- 自动同步进行中触发手动同步，确认不会并发第二个同步
- 关闭应用后等待超过 15 分钟，不要求发生同步；重新打开应用后确认会在延迟窗口后补同步

### 7. 全量验证命令

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
npx tsc --noEmit
npm run lint
npm run build
```

## Review Gates

- Gate 1：`prd.md` 已消除需求冲突，尤其是并发语义和页面感知语义
- Gate 2：`design.md` 已覆盖前端事件刷新、配置顺序保存，以及 session-scoped autosync 的边界
- Gate 3：`implement.md` 已同时覆盖后端与前端实施步骤
- Gate 4：三份文档的术语一致：
  - “统一同步核心”
  - “全局串行护栏”
  - “sync-finished 事件”
  - “持久化后的 last_sync_at”
  - “应用运行期间自动同步 / session-scoped autosync”

## Rollback Points

- 若后续认为“页面打开时实时刷新 `last_sync_at`”范围过大，可降级为“页面重新进入后读取到最新值”，但必须同步回退 PRD 与 Design
- 若后续认为手动同步 busy 错误文案需要另议，可只保留“不得并发”约束，把文案留到实现时细化
- 若后续前端事件名或 payload 设计调整，三份计划文档必须一起更新，不能只改实现计划
- 若后续决定支持“休眠恢复/前台恢复补同步”，应作为新一轮 planning 增强，不要在当前基础版实现中隐式扩 scope
