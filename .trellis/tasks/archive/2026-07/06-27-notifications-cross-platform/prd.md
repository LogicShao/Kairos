# 跨平台通知能力评估与方案设计

## Goal

评估 Kairos 现有技术栈是否足以支撑跨平台本地通知，并收敛出一个可落地的 MVP 范围，为后续通知功能实现提供明确边界。

## What I already know

- 当前技术栈是 `Tauri 2.11 + React 19 + TypeScript + Rust + SQLite`。
- Rust 入口位于 [src-tauri/src/lib.rs](/D:/proj/Kairos/src-tauri/src/lib.rs:1)，已经存在常驻线程能力（番茄钟 tick、自动同步 loop）。
- 当前依赖中尚未接入 `tauri-plugin-notification`，`package.json` 与 [src-tauri/Cargo.toml](/D:/proj/Kairos/src-tauri/Cargo.toml:1) 都没有通知插件。
- 当前 capability 只有 [src-tauri/capabilities/default.json](/D:/proj/Kairos/src-tauri/capabilities/default.json:1) 的 `core:default`。
- 现有业务数据里：
- `Task.due_date` 只有日期，没有时分秒，见 [src/types/task.ts](/D:/proj/Kairos/src/types/task.ts:1)。
- `Exam.exam_datetime` 已有精确时间，见 [src/types/exam.ts](/D:/proj/Kairos/src/types/exam.ts:1)。
- 番茄钟当前依赖应用内线程与事件，不是系统通知驱动，见 [src-tauri/src/lib.rs](/D:/proj/Kairos/src-tauri/src/lib.rs:1)。
- 历史规划里已经明确提过“桌面通知（工作结束 / 休息结束）”和“考前提醒通知”，见 [06-13-kairos-initial-planning/prd.md](/D:/proj/Kairos/.trellis/tasks/archive/2026-06/06-13-kairos-initial-planning/prd.md:1)。
- 项目近期目标平台已扩展到 Windows / Linux / Android，见 [06-20-android-multiplatform/prd.md](/D:/proj/Kairos/.trellis/tasks/archive/2026-06/06-20-android-multiplatform/prd.md:1)。

## Assumptions (temporary)

- 第一阶段讨论的是“本地系统通知”，不涉及推送服务端。
- 第一阶段优先覆盖当前项目真实目标平台：Windows、Linux、Android；macOS/iOS 先作为技术兼容加分项，不作为 MVP 硬要求。
- 第一阶段不引入常驻后台服务或独立守护进程，优先使用 Tauri 官方插件与系统原生通知能力。

## Open Questions

- 无。当前 MVP 范围、平台范围与考试提醒粒度已确认。

## Requirements (evolving)

- 基于现有技术栈完成一次可证据化的通知能力评估，不靠主观印象下结论。
- 明确区分三类能力：
- 应用运行中即时通知。
- 指定未来时间的本地调度通知。
- 应用进入后台、被系统挂起或重启后的可靠性边界。
- 明确现有数据模型是否足以承载提醒：
- 番茄钟是否可直接调度结束通知。
- 考试是否可直接基于 `exam_datetime` 生成提醒。
- TODO 是否需要新增提醒时间字段或提醒策略字段。
- 第一阶段通知 MVP 只包含：
- 番茄钟阶段结束通知。
- 考试考前提醒通知。
- 第一阶段验收平台为：
- Windows。
- Linux。
- Android。
- 第一阶段不包含：
- TODO 截止提醒。
- 课程上课前提醒。
- 输出一个建议的 MVP 范围，避免一次性覆盖全部实体导致设计过重。

## Acceptance Criteria (evolving)

- [ ] 已明确现有技术栈是否可以支持 Windows / Linux / Android 的本地通知。
- [ ] 已明确通知能力的 MVP 建议范围，以及不建议首期纳入的部分。
- [ ] 已明确至少一项必须补充的数据模型约束或配置项。
- [ ] 已明确第一阶段的验收平台范围。
- [ ] 已明确考试提醒第一期的配置粒度。
- [ ] 已将外部文档调研结论持久化到 `research/` 文件，而不是只留在对话里。

## Decision (ADR-lite)

**Context**：通知功能候选范围包括番茄钟、考试、TODO、课程提醒，但现有数据模型与实现复杂度并不一致。  
**Decision**：第一阶段只做“番茄钟阶段结束通知 + 考试考前提醒通知”，并要求 Windows / Linux / Android 三端覆盖；考试提醒采用全局统一 offset 规则，而不是每个考试单独配置。  
**Consequences**：可以复用现有 `Pomodoro` 运行态与 `Exam.exam_datetime`，避免为 `Task.due_date`、课程实例推导和单考试提醒配置过早引入新模型与复杂批量调度逻辑；同时必须在设计阶段显式处理 Android 后台与 Linux 桌面环境差异。

## Out of Scope (explicit)

- 推送通知服务端。
- 邮件、短信、IM Bot 等远程提醒渠道。
- iOS / macOS 作为本轮必须验证的平台。
- 通知中心、历史通知列表、复杂通知交互编排。
- TODO 截止提醒。
- 课程上课前提醒。

## Technical Notes

- 代码检查文件：
- [src-tauri/src/lib.rs](/D:/proj/Kairos/src-tauri/src/lib.rs:1)
- [src-tauri/Cargo.toml](/D:/proj/Kairos/src-tauri/Cargo.toml:1)
- [src-tauri/tauri.conf.json](/D:/proj/Kairos/src-tauri/tauri.conf.json:1)
- [src-tauri/src/db/models.rs](/D:/proj/Kairos/src-tauri/src/db/models.rs:1)
- [src-tauri/src/db/migrations.rs](/D:/proj/Kairos/src-tauri/src/db/migrations.rs:1)
- [src/types/task.ts](/D:/proj/Kairos/src/types/task.ts:1)
- [src/types/exam.ts](/D:/proj/Kairos/src/types/exam.ts:1)
- 外部调研见 `research/notification-capability.md`
