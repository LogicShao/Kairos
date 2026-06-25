# Code quality and architecture comments

## Goal

提高 Kairos 代码库在多 AI session 维护场景下的可理解性和一致性：为数据结构、跨层契约、同步协议、状态/时间计算等高风险代码补齐最小必要注释，并顺手修复审计中发现的低风险类型契约或质量问题。目标不是扩大重构范围，而是降低后续 AI 误读字段语义、协议边界和架构约束的概率。

## What I Already Know

- 用户明确认为当前大部分代码缺少最小注释，尤其数据结构和重要架构代码，且项目几乎全部由 AI 编写，跨 session 理解一致性是核心风险。
- 后端已有 `.trellis/spec/backend/comment-guidelines.md`，规定跨模块结构体字段、非显而易见纯函数、协议常量和通信协议模块需要最小注释。
- 后端 `src-tauri/src/sync/exporter.rs`、`src-tauri/src/commands/sync.rs`、`src-tauri/src/sync/webdav.rs` 已有较多同步协议注释，整体方向正确。
- 后端 `src-tauri/src/db/models.rs` 已为 `sync_id`、`deleted_at`、`remote_etag`、`device_id`、`dataset_id` 等 v2 同步关键字段补充了注释。
- 前端 `.trellis/spec/frontend/type-safety.md` 仍是占位内容，缺少 Tauri IPC 类型、后端契约镜像、字段注释和运行时校验边界规范。
- 前端 `src/types/*.ts` 是后端 Tauri command 的返回/入参契约镜像，但大部分接口字段没有说明语义、日期格式、取值边界或是否与后端字段一一对应。
- 初步审计发现前端 `Task`、`Course`、`Exam` 类型尚未包含后端 v2 同步字段 `sync_id` / `deleted_at`，需要确认这是有意隐藏还是类型契约滞后。
- `src/types/schedule.ts` 已有整体“与后端对齐”的注释，但字段级语义仍较少；后端 `src-tauri/src/schedule.rs` 的响应结构也是跨层契约，适合纳入第一批注释治理。

## Requirements

- 第一批治理范围采用“高风险最小范围”：只处理 `src/types/*.ts`、前端 type-safety spec、后端 schedule/sync/db model 中明显注释缺口。
- 建立前端最小注释和类型契约规范，补齐 `.trellis/spec/frontend/type-safety.md`，使后续 AI 明确哪些 TypeScript 类型必须写字段语义。
- 将现有后端注释规范落实到第一批高风险模块，而不是全仓库机械补注释。
- 优先覆盖跨层数据契约：`src/types/*.ts`、对应 Rust model/response struct、Tauri command payload。
- 优先覆盖同步相关架构边界：WebDAV 同步、v2 快照、ETag、LWW、墓碑、设备/数据集标识。
- 优先覆盖时间/日程计算相关结构：课程周次、考试时间、日历周响应、任务日历事件。
- 注释必须解释“为什么 / 语义边界 / 不变量 / 格式约束”，不得重复代码表面含义。
- 在审计过程中发现低风险类型不一致时可以一并修复；涉及行为变化、数据库迁移或大范围重构时必须拆出新任务。
- 所有改动保持 KISS / YAGNI：只补当前可验证的缺口，不为了“看起来完整”添加抽象或无意义注释。

## Acceptance Criteria

- [x] `.trellis/spec/frontend/type-safety.md` 不再是占位文档，包含 Kairos 实际的 TypeScript 类型组织、Tauri IPC 契约、字段注释和禁止模式。
- [x] 第一批高风险前端类型文件已补充最小注释，至少覆盖同步、任务、课程、考试、日程/日历类型。
- [x] 第一批高风险后端结构体或协议边界已按现有 `comment-guidelines.md` 补齐缺失注释。
- [x] 如果前后端类型存在字段滞后，已选择并执行一致策略：显式补齐、显式隐藏并注释原因，或拆出独立行为任务。
- [x] 没有给自解释局部变量、普通 CRUD 包装函数、纯展示组件添加噪音注释。
- [x] `npm run lint` 通过。
- [x] `npm run build` 或等价 TypeScript 检查通过。
- [x] Rust 格式、lint 或测试按影响范围运行，且失败项被记录。

## Definition of Done

- 注释规范已进入 Trellis spec，后续 AI 能在开发前读取。
- 第一批代码注释和小型质量修复已完成并验证。
- 任务结束前记录本次发现的注释/类型契约规则，必要时更新 backend/frontend spec。
- 不主动执行 git commit；只有用户明确要求时才提交。

## Out of Scope

- 不做全仓库注释覆盖率工程。
- 不重写同步协议、数据库 schema 或日程计算逻辑。
- 不引入新的文档生成工具或注释检查依赖，除非后续单独确认。
- 不为了注释而重构组件结构或拆分大型文件。
- 不处理已有独立规划任务中的 UI 改造。

## Open Questions

- None.

## Technical Notes

- 当前任务目录：`.trellis/tasks/06-25-code-quality-comments`
- 用户已选择范围方案 1：高风险最小范围。
- 已审计规范：`.trellis/spec/backend/comment-guidelines.md`、`.trellis/spec/backend/quality-guidelines.md`、`.trellis/spec/frontend/quality-guidelines.md`、`.trellis/spec/frontend/type-safety.md`、`.trellis/spec/guides/cross-layer-thinking-guide.md`
- 已审计代码：`src-tauri/src/db/models.rs`、`src-tauri/src/sync/exporter.rs`、`src-tauri/src/commands/sync.rs`、`src-tauri/src/sync/webdav.rs`、`src-tauri/src/schedule.rs`、`src/types/sync.ts`、`src/types/task.ts`、`src/types/course.ts`、`src/types/exam.ts`、`src/types/schedule.ts`
- 复杂度分类：Complex。需要在实现前补充 `design.md` 和 `implement.md`。
