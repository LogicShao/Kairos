# Implementation: AI 每日摘要生成器（AI Morning Brief）

## Preconditions

- 父任务 `06-26-competitor-ai-research` 的 Phase 2a 子任务
- 分支：**先建 `feat/ai-morning-brief`**（父 PRD 要求 main 恒稳，本功能独立分支）
- 基线：`feat/today-briefing` 已合入 main（含 `get_today_briefing`）
- 本轮不实现：SSE、任务建议、多 Provider、测试连接按钮
- 保持工作区其他脏文件不被顺手修改

## Ordered Steps

### 1. 建分支 + 抽复用函数

- [x] 建分支：`git checkout -b feat/ai-morning-brief`
- [x] `briefing.rs`：命令体抽取 `pub(crate) fn collect_today_briefing(conn, engine) -> Result<TodayBriefingResponse, String>`，`get_today_briefing` 改为调用它（**命令签名与响应字段不变**）

验证：

- [x] `cargo test --manifest-path src-tauri/Cargo.toml commands::briefing`（12 个既有测试通过）

### 2. 迁移 v11 + DB 层

- [x] `db/migrations.rs` 追加 v11：`ai_config` + `ai_morning_brief` 两表（见 design.md 3.1 SQL）
- [x] **同步改 4 处硬断言**：table_count 13→15、migration 记录 10→11
- [x] `db/models.rs` 新增 `AiConfig` / `AiConfigView` / `UpdateAiConfigRequest` / `AiMorningBrief`
- [x] 新增 `db/ai.rs`：`get_ai_config`（无行则插默认）、`update_ai_config`（Option 合并 + key 加密分支）、`get_morning_brief(conn, date) -> Option<AiMorningBrief>`、`upsert_morning_brief`（INSERT OR REPLACE / ON CONFLICT）
- [x] `db/mod.rs` 加 `pub mod ai;`

验证：

- [x] `cargo test --manifest-path src-tauri/Cargo.toml db::ai`（CRUD + 迁移幂等测试）
- [x] `cargo test --manifest-path src-tauri/Cargo.toml`（迁移断言改后全绿）

### 3. 加密模块 `ai/crypto.rs`

- [x] `ensure_key_file(app_data_dir) -> Result<[u8;16]>`：首启用 `Uuid::new_v4().into_bytes()` 生成 16 字节写 `app_data_dir/.ai_encryption_key`；Unix chmod 600
- [x] `encrypt_api_key` / `decrypt_api_key`：AES-128-CBC + 每次随机 IV + PKCS7，格式 `hex(iv):hex(cipher)`
- [x] 单元测试：roundtrip、随机 IV 两次密文不同、key 文件缺失报错

验证：

- [x] `cargo test --manifest-path src-tauri/Cargo.toml ai::crypto`

### 4. AI 核心 `ai/{mod,deepseek,rule,prompt}.rs`

- [x] `ai/mod.rs`：`AIService` trait、`BriefSource`（serde lowercase）、`AiError`、`resolve_service`、`AiBriefResult`
- [x] `ai/prompt.rs`：`SYSTEM_PROMPT` 常量（5 分节 + 反幻觉规则 + `*AI生成，请核实*`）、`MAX_TOKENS=400`、`TEMPERATURE=0.7`、`MAX_CONTEXT_CHARS=4000`、`MODEL_DEFAULT="deepseek-chat"`、`build_user_message(b)`（裁剪 id）
- [x] `ai/deepseek.rs`：`DeepseekProvider`（blocking client 15s、POST `/v1/chat/completions`、错误映射 401/429/超时）、DTO
- [x] `ai/rule.rs`：`render_rule_briefing(b)` → 同构 5 分节 + `*本地生成*` footer

验证：

- [x] `cargo check --manifest-path src-tauri/Cargo.toml`
- [x] 结构校验函数 `validate_ai_output(markdown) -> bool`（5 节标题 + footer）单元测试

### 5. 生成编排 `ai/morning_brief.rs` + 命令层 `commands/ai.rs`

- [x] `generate_today_brief(db, engine, app_handle, force)`：缓存 → in-flight 护栏 → 窄锁快照 → AI/降级 → 结构校验 → double-check upsert → emit
- [x] `today_china()`：+08:00 日期
- [x] `commands/ai.rs`：4 命令（get/update config、get/generate brief）；`update_ai_config` 后调 `reschedule`
- [x] `commands/mod.rs` 加 `pub mod ai;`

验证：

- [x] `cargo check` + `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- [x] 单元测试：`validate_ai_output`、`render_rule_briefing` 骨架、缓存命中/force 重生成逻辑（用临时表）

### 6. 7:00 调度 `ai/scheduler.rs` + lib.rs 接线

- [x] `ensure_scheduled` / `reschedule` / `next_seven_am_utc`（单 cancel token，仿 pomodoro_scheduler）
- [x] 调度线程：sleep 到 +08:00 07:00 → `generate_today_brief(force=false)` → 系统通知 → 循环次日
- [x] `lib.rs`：`pub mod ai;`、setup 中 `ensure_key_file` + `ensure_scheduled`、invoke_handler 注册 4 命令
- [x] 通知 ID：`ids::stable_id("ai:morning-brief")`

验证：

- [x] `cargo test --manifest-path src-tauri/Cargo.toml ai::scheduler`（`next_seven_am_utc` 边界：跨天、恰好 07:00、23:00）
- [x] `cargo check`

### 7. 前端类型 + Markdown 子集 + AiBriefCard

- [x] `src/types/ai.ts`（见 design.md 3.3）
- [x] `src/lib/markdown-subset.tsx`（React 节点渲染，无 dangerouslySetInnerHTML）
- [x] `src/pages/today/AiBriefCard.tsx`（config-loading → 未启用隐藏 / idle / loading / ready / error；监听 `ai-brief-generated`）
- [x] `TodayPage.tsx` 挂载 AiBriefCard

验证：

- [x] `npx tsc --noEmit`、`npm run lint`

### 8. 设置页 + 导航接线

- [x] `src/components/settings/AiSettings.tsx`（save-on-leave + inputClass + password placeholder）
- [x] `App.tsx` 加 `ai-settings` 路由
- [x] `AppShell.tsx` `isKairosArea` 加 `'ai-settings'`
- [x] `KairosHub.tsx` `HUB_ENTRIES` 加「AI 设置」入口

验证：

- [x] `npx tsc --noEmit`、`npm run lint`

### 9. 全量质量门

- [x] `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`
- [x] `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- [x] `cargo test --manifest-path src-tauri/Cargo.toml`
- [x] `npm run lint`、`npx tsc --noEmit`
- [x] `npm run build`（Vite 现有 chunk 警告可接受）
- [x] `npx jscpd . --threshold 10 --reporters console --format rust,typescript`（<10% 重复）
- [x] 回归断言：导出 JSON 不含 `ai_config` / `ai_morning_brief`

## Validation Commands

```powershell
cargo fmt --manifest-path "src-tauri/Cargo.toml"
cargo clippy --manifest-path "src-tauri/Cargo.toml" --all-targets -- -D warnings
cargo test --manifest-path "src-tauri/Cargo.toml"
npm run lint
npx tsc --noEmit
npm run build
```

## Review Gates

Result 2026-07-31:

- 分支 `feat/ai-morning-brief` 已创建，任务状态 in_progress。
- `cargo fmt --check` passed。
- `cargo clippy --manifest-path "src-tauri/Cargo.toml" --all-targets -- -D warnings` passed。
- `cargo test --manifest-path "src-tauri/Cargo.toml"` passed：205 tests（新增 ai::* 14、db::ai 5、exporter 回归 1，基线 185）。
- `npm run lint` passed；`npx tsc --noEmit` passed；`npm run build` passed（Vite 既有 >500 kB chunk 警告）。
- `npx jscpd . --threshold 10 --reporters console --format rust,typescript` 8.33% < 10%。
- 同步回归断言 `test_export_excludes_ai_tables` 验证 ai_* 表不进 WebDAV 快照。

### Gate 1：反幻觉与成本护栏

- AI 输出只来自给定 JSON；结构校验失败即降级，无第二次 API 调用
- `max_tokens=400` 硬编码，不信任前端；按日缓存 + in-flight 护栏防重复计费

### Gate 2：加密与不同步

- key 仅密文入库；`get_ai_config` 永不过桥明文
- `ai_*` 表不在 WebDAV 白名单导出；有回归断言

### Gate 3：main 稳定

- `get_today_briefing` 命令契约不变
- `feat/ai-morning-brief` 独立分支，默认 `enabled=0` 现有用户无感知

### Gate 4：跨平台

- 同步命令 + blocking reqwest（Android 已验证先例）
- 无平台专属 API；无新依赖

## Risks

- **阻塞命令线程**：15s 网络调用占一条 Tauri 命令线程。前端 loading 兜底；若报错多改为 spawn 线程 + 事件（后续优化，非本轮）
- **并发竞态残余**：极端下 in-flight 护栏 + double-check 仍可能双生成一次，成本上限约 0.04 美分/次，可接受
- **key 文件丢失**（清数据/卸载重装）：解密失败 → 明确报错「AI key 文件缺失，请重新配置」；UI 提示重新录入
- **迁移断言漏改**：4 处硬编码计数（table_count 13→15、记录 10→11）必须同步，否则 CI 红
- **Android 后台限制**：7:00 跟随进程存活，进程被杀错过触发（与考试通知同级，PRD 已接受）
- **模型不守分节**：结构校验兜底降级规则引擎，不产生额外成本
