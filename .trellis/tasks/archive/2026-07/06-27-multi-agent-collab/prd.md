# 多 Agent 协作规范

## Goal

为 Kairos 项目建立一套适用于 `Codex + Claude Code + DeepSeek` 混合工作的多 Agent 协作规范，降低跨工具切换时的上下文丢失、重复劳动和互相覆盖风险。

## What I already know

- 当前实际工作模式不是单平台多 agent，而是跨两个工作面的人肉调度：
- `GPT/Codex` 在 Codex 中工作
- `DeepSeek` 在 Claude Code 中工作
- 现有 Trellis 工作流已能管理 task / prd / design / implement / spec，但没有明确规定跨平台 handoff 协议。
- 项目已使用 `.trellis/tasks/` 作为任务事实源，适合承载跨 Agent 交接材料。

## Requirements

- 规范必须明确各 Agent 的职责边界，而不是笼统写“按需协作”。
- 规范必须定义单一事实源，避免以聊天记录为工作依据。
- 规范必须定义 handoff 机制，至少覆盖：
- 当前做到哪一步
- 改了哪些文件
- 哪些风险/阻塞仍存在
- 跑了哪些检查
- 规范必须限制并行编辑风险，明确哪些情况禁止双端同时改同一任务。
- 规范必须给出推荐工作流，覆盖：
- GPT/Codex 负责规划
- DeepSeek/Claude Code 负责实施
- GPT/Codex 负责 review/check/收口
- 规范必须区分哪些任务适合双端协作，哪些任务应由单一 Agent 直接完成。
- 规范应落到 `.trellis/spec/guides/`，成为长期项目规则。

## Acceptance Criteria

- [ ] 已产出一份可直接复用的多 Agent 协作 guide。
- [ ] Guide 已定义角色分工、handoff 模板、切换清单和禁止并行场景。
- [ ] Guide 已加入 `.trellis/spec/guides/index.md`，后续会被 Agent 当作项目规范读取。
- [ ] 规范内容已适配 Kairos 当前的 `Codex + Claude Code + DeepSeek` 现实工作方式，而非泛泛而谈。

## Out of Scope

- 不改 Trellis 脚本本身。
- 不自动化 Claude/Codex 间的任务切换。
- 不引入新的 orchestrator 或调度框架。

## Technical Notes

- 目标落点：`.trellis/spec/guides/multi-agent-collaboration-guide.md`
- 需要同步更新：`.trellis/spec/guides/index.md`
