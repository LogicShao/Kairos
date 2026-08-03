# AI 每日摘要生成器（AI Morning Brief）

## Goal

在 Kairos 的 `Today` 工作面上新增「AI 每日摘要」：deepseek 读取本地今日数据（复用 Phase 1 已落地的 `TodayBriefingResponse`），生成一份自然语言 Markdown 晨间摘要，支持早 7:00 自动触发与手动一键生成。AI 能力全部**可选**，离线时自动降级到本地规则引擎，生成内容明确标注来源。

## What I already know

- Today Briefing Card（Phase 1，`06-28-today-briefing-card`）已产出结构化 `TodayBriefingResponse`，`briefing.rs` 注释明确「供后续 AI Morning Brief 复用」。
- 项目当前**无** AI 模块、无 markdown 库、无 secure store 依赖；已有 AES-CBC 加解密（`lzu/crypto.rs`，但其文件头注释明确「不要在其他场景复用此实现」）。
- `reqwest` 已开 `json` + `rustls-tls` + `blocking` feature，可直接走 OpenAI 兼容 `POST /v1/chat/completions`（非流式，本轮不做 SSE）。
- WebDAV 同步是**白名单机制**（`SyncData` 只含 tasks/courses/exams/pomodoro_sessions/term_phases），新增 `ai_*` 表天然不进同步快照。
- 定时/通知已有成熟范式：`exam_scheduler`（多 token HashMap）、`pomodoro_scheduler`（单 token cancel）。
- 父任务 PRD 要求：所有 AI 功能可选、离线降级规则引擎、生成内容标注「AI生成，请核实」、key 加密存储不同步 WebDAV、单次 <0.05 美分、main 恒稳独立 feature 分支。

## Constraints

1. **AI 可选**：默认关闭（`enabled=0`），不改变现有用户行为；用户显式开启并配置 key 才启用。
2. **反幻觉**：只总结给定 JSON、绝不编造；输出固定 5 分节 + 结尾 `*AI生成，请核实*`；结构校验失败即降级本地规则，不发起第二次 API 调用。
3. **成本友好**：单次生成 < 0.05 美分；`max_tokens=400` 硬编码护栏；按日缓存避免重复计费；in-flight 护栏防并发双计费。
4. **API key 加密存储、不同步 WebDAV**：密文入 DB，本地随机 key 文件 + AES-CBC 加密；`get` 命令永不解密回传明文。
5. **跨平台**：Windows / Linux / Android；同步 Tauri 命令 + blocking reqwest（与现有 WebDAV 命令先例一致）。
6. **离线降级**：AI 调用任何失败（离线/超时/HTTP/解析/结构校验）→ 本地规则引擎生成同构摘要，footer 为 `*本地生成*`。
7. **main 稳定**：本功能在 `feat/ai-morning-brief` 分支开发；`get_today_briefing` 命令签名与响应契约不变（仅内部抽取复用函数）。

## Requirements

### R1: AIService 抽象

- `AIService` trait（`generate_daily_brief`），两个实现：`DeepseekProvider`（OpenAI 兼容非流式）与 `RuleBasedService`（本地降级）。
- 命令层 `resolve_service()` 分发；AI 调用任何失败 `log::warn` 后回退规则引擎，不让网络错误打断用户。

### R2: 配置与 Key 管理

- `ai_config` 单例表（id=1），默认 `enabled=0`、`base_url=https://api.deepseek.com`（**不含** `/v1`）、`model=deepseek-chat`。
- API key 经 AES-CBC（每次随机 IV + 本地随机 key 文件）加密后入库；`get_ai_config` 只回传 `api_key_configured` 布尔。
- 设置页可开关、填 key、改 base_url/model；保存时未改动的 key 字段不重加密（随机 IV 避免无意义密文 churn）。

### R3: 每日摘要生成

- `get_ai_morning_brief`：读今日缓存（无则 `None`，前端据此显示生成按钮）。
- `generate_ai_morning_brief`：生成并持久化到 `ai_morning_brief`（date UNIQUE），支持 `force` 重生成。
- 7:00 定时与手动共用同一 `generate_today_brief(force)` 入口；先查今日缓存 → in-flight 护栏 → 生成 → double-check upsert。

### R4: 反幻觉与成本

- 系统提示词强制：只总结、逐字引用、考试一律用后端算好的 `days_until`、空节写「暂无」、禁止编造 JSON 之外字段。
- 本地结构校验（5 个节标题 + 结尾 footer）失败 → 降级规则引擎，零额外成本。
- `max_tokens=400`、`temperature=0.7`、`stream=false` 显式传入；user 消息裁剪掉 `id` 字段减 token。

### R5: Today 页集成

- 新增 `AiBriefCard` 组件挂载在现有 Today Briefing 卡片之下：idle（未生成）→ loading → ready / error。
- ready 态用轻量 Markdown 子集渲染器（**不引新依赖**）渲染 + 来源徽标（AI/本地）+ model + 重新生成按钮。
- 未启用 AI 时整卡隐藏（AI 可选，不打扰默认用户）。
- 监听 `ai-brief-generated` 事件：7:00 自动生成时若页面打开则自动刷新。

### R6: 7:00 定时触发

- 仿 `pomodoro_scheduler` 单 cancel-token 模式；按 +08:00 计算到下一个 07:00 的时长 sleep，到点调用 `generate_today_brief(force=false)` 并发系统通知，随后循环次日。
- 仅在 AI 已启用且有 key 时调度；配置变更（enabled/base_url/model/key）触发 reschedule。
- Android 跟随进程存活（与现有考试通知同级，PRD 已确认接受），不承诺系统级准点。

### R7: 设置页

- `KairosHub` 新增「AI 设置」入口，新增 `AiSettings` 页面，仿 `SyncSettings`/`NotificationSettings` 范式：
  - enabled 开关、base_url、model、api_key（password，已配置时 placeholder「已保存密钥，留空不变」）。
  - 成本说明（单次约 <1000 token）、隐私说明（密钥仅本机 AES 加密存储，不同步 WebDAV；课表/任务/考试数据会发往第三方 API）。

## Acceptance Criteria

- [ ] 默认关闭 AI，现有用户无感知；未启用时 Today 页不显示 AI 卡片
- [ ] 配置 key + base_url/model 后，手动生成返回 AI 摘要，Markdown 含固定 5 分节 + `*AI生成，请核实*` footer
- [ ] 同一天重复打开 Today 页 / 重复生成返回缓存不再计费；`force=true` 才重新调用 API
- [ ] 离线 / 网络失败 / HTTP 错误 / 结构校验失败时降级为本地规则摘要，footer `*本地生成*`
- [ ] API key 仅以密文存 DB，`get_ai_config` 不回传明文；`ai_config` / `ai_morning_brief` 不在 WebDAV 同步快照中（有回归断言）
- [ ] 7:00 定时触发（进程存活时）生成摘要并发系统通知；页面打开时收到事件自动刷新
- [ ] 单次生成成本 < 0.05 美分（`max_tokens=400` 硬编码护栏，不信任前端参数）
- [ ] 同步 Tauri 命令 + blocking reqwest，Windows / Linux / Android 编译通过
- [ ] `cargo test`、`cargo clippy -- -D warnings`、`npm run lint`、`npx tsc --noEmit` 通过
- [ ] 迁移硬编码断言同步更新（table_count 13→15、migration 记录 10→11）

## Definition of Done

- `AIService` trait 承载 AI 能力，未来 Provider 切换 / 任务建议 / 番茄反思可直接复用。
- 反幻觉、成本、加密、降级四道护栏全部落地且有测试。
- 分支 `feat/ai-morning-brief` 独立可合入、可废弃，`main` 保持可构建可运行。

## Out of Scope

- SSE 流式渲染与打字机效果（后续打磨）
- 智能任务优先级建议、番茄 AI 反思（父任务 Phase 2 的独立后续子任务）
- 多 Provider 切换 / Provider 下拉（父任务 Phase 3）
- 「测试连接」按钮（生成按钮即验证连接）
- Kairos MCP Server、插件卡片系统

## Technical Notes

- 预期新增后端模块：`src-tauri/src/ai/{mod,deepseek,rule,prompt,crypto,scheduler,morning_brief}.rs`、`db/ai.rs`、`commands/ai.rs`。
- 预期新增前端：`src/types/ai.ts`、`src/lib/markdown-subset.tsx`、`src/pages/today/AiBriefCard.tsx`、`src/components/settings/AiSettings.tsx`。
- 迁移 v11 新增 2 表；`get_today_briefing` 内部抽取 `pub(crate) fn collect_today_briefing` 供 AI 复用（命令契约不变）。
- 父任务来源：`.trellis/tasks/06-26-competitor-ai-research`，对应 PRD「Phase 2a: AI摘要」。
