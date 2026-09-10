# AI 晨报（SSE 流式 + 规则降级）

## Goal

实现 `internal/domain/ai`：AI 提供商（DeepSeek 非流式 + SSE 流式，`/v1/chat/completions`）、晨报生成（提示词/输出校验/规则降级）、按日缓存 + in-flight 护栏、AI 配置（api_key AES-256-GCM 加密存储 + 恢复密钥）、今日概览聚合复用 W5。

**架构差异（app→web）**：Rust `ai/scheduler.rs`（7:00 自动生成）的**触发移入 W9 集中调度器**（SMTP）；W8 只做生成侧与配置。SSE 流式替代 Tauri Channel。

## Requirements

- G1 `internal/domain/ai`：provider 接口（DeepSeek 非流式 + SSE 流式）、提示词/输出校验（对齐 `ai/prompt.rs`）、规则降级（对齐 `ai/rule.rs`）、API key AES-256-GCM 加解密
- G2 晨报：按日缓存、in-flight 护栏、`POST /api/ai/morning-brief/generate` SSE 流式、`?sync=true` 非流式、今日概览聚合复用 W5
- G3 handler：`GET/PATCH /api/ai/config`、`GET/POST /api/ai/morning-brief`、`GET/POST /api/ai/sync-recovery-key`
- G4 DTO 对齐 `src/types/ai.ts`

## Acceptance Criteria

- [ ] `go test ./internal/domain/ai/...` 绿（流式分块输出、规则降级、缓存幂等、加解密 roundtrip，对齐 Rust 测试）
- [ ] `go test ./internal/...` 全绿
- [ ] curl 冒烟：config 读写、晨报生成（可用 mock provider 或规则降级路径）、缓存命中
- [ ] dev PG 容器仅验证期间临时启动，验证后已关闭

## Notes

- 移植锚点：Rust `src-tauri/src/ai/{deepseek.rs,prompt.rs,rule.rs,morning_brief.rs,crypto.rs}`（1682 行）+ 其 `#[cfg(test)]`
- 加密：AES-256-GCM（W7 已建信封加密基建，复用模式）
- **不移植** `ai/scheduler.rs` 的 7:00 后台触发——W9 集中调度器接 SMTP
- DeepSeek 端点 `/v1/chat/completions` 是 OpenAI 兼容（非 LZU v1，勿混淆）