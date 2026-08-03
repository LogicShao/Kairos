# 今日概览卡片（Today Briefing Card）

## Goal

新增一个纯本地、零 AI 依赖的 `Today` 主工作面，并在其中承载「今日概览卡片（Today Briefing Card）」，把“今天最该关注什么”收敛成一个低摩擦入口。该工作面既服务当前用户直接查看，也作为后续 AI 每日摘要的结构化数据基石。

## What I already know

- `KairosHub.tsx` 当前仍是静态入口卡片列表，更接近“工具入口页”，不适合承载高频的今日决策信息。
- 项目已有可复用的本地数据源：
  - `get_all_tasks`
  - `get_all_exams`
  - `get_all_courses`
  - `get_calendar_week`
  - `get_pomodoro_state`
- `schedule.rs` 已经能把课程、考试、待办聚合成日历事件，但它服务的是周视图，不等于“今日摘要”。
- `Task.due_date` 只有日期，没有具体提醒时间，因此 Today Briefing 的任务逻辑应围绕“今天到期 / 已逾期 / 高优先级”展开，而不是复杂时间调度。
- `PomodoroState.completed_sessions` 是当前运行态字段，不适合作为可靠的“今日累计专注统计”；若要做“今日专注统计”，需要额外聚合 `pomodoro_sessions`。
- FasterLZU 的 Android 小组件思路适合作为 Kairos Today 的产品参考：优先展示“今日课程 / 下一节课 / 地点”，但 Kairos 本任务仍做跨平台应用内 Today 页面，不新增平台专属小组件。

## Constraints

1. **零 AI 依赖**：本任务只做本地规则引擎和卡片渲染，不接入任何模型或远程 API。
2. **服务主线，不扩故事**：目标是增强 Kairos 现有核心流程，不引入新的导航层或聊天式入口。
3. **跨平台兼容**：保持 Windows / Linux / Android 的 Tauri v2 运行方式，不依赖平台专属 UI 能力。
4. **为后续 AI 铺路**：本轮输出的摘要结构应能被后续 AI Morning Brief 直接复用。
5. **轻量实现**：优先单个聚合命令 + 单个首页卡片，不做多模块仪表盘。

## Requirements

### R1: 新增 Today 主工作面

- 新增一个独立的 `Today` 页面，作为“今天先做什么”的主工作面
- `Today Briefing Card` 放在该页面首屏核心位置
- `KairosHub` 保持为工具入口页，不承载 Today Briefing
- `Today` 页面应进入顶级导航，而不是隐藏在二级页面中

### R2: 聚合 4 类今日信息

卡片至少包含以下 4 类信息：

1. **今日课程**
   - 今天课程数量
   - 若有未开始课程，显示下一节课的名称、时间、地点
2. **待办**
   - 今天到期的未完成任务数量
   - 已逾期未完成任务数量
   - 可选显示 1-3 条最需要关注的任务标题
3. **最近考试**
   - 下一场未过去考试
   - 显示考试名称、剩余天数/是否今日、时间
4. **专注状态**
   - 当前番茄钟是否进行中
   - 当前 phase 和剩余时间
   - 若未运行，显示“可开始专注”之类的轻提示

### R3: 规则引擎优先级

- Today Briefing 必须由本地规则引擎排序，不依赖 AI
- 规则需要体现最小产品判断：
  - 逾期待办优先于普通待办
  - 今天课程优先于未来课程
  - 最近考试优先于更远考试
  - 正在进行的番茄状态优先于静态配置展示

### R4: 交互低摩擦

- 卡片中的每个信息块应支持点击跳转到对应页面：
  - 课程 → `courses`
  - 待办 → `todo`
  - 考试 → `exams`
  - 专注 → `pomodoro`
- 不要求块内二级交互（如展开、编辑、删除）

### R5: 作为 AI 基石的结构化输出

- 后端应提供一个明确的聚合返回结构（例如 `TodayBriefingResponse`）
- 该结构应能直接被未来 AI 每日摘要复用
- 字段命名和语义应稳定，不依赖 UI 本地拼接字符串

## Acceptance Criteria

- [x] 存在独立的 `Today` 页面，且 Today Briefing 位于首屏核心位置
- [x] 卡片可展示今日课程、待办、最近考试、专注状态 4 类信息
- [x] 今日课程为空时有明确空状态文案
- [x] 今日待办为空时有明确空状态文案
- [x] 没有未来考试时有明确空状态文案
- [x] 番茄钟运行中时能显示当前 phase 和剩余时间
- [x] 卡片信息块点击后可跳转到对应功能页
- [x] 规则排序体现“逾期/今日/最近/运行中”的优先级
- [x] 后端存在稳定的结构化聚合响应，而不是前端临时拼装
- [x] `npm run lint`、`npx tsc --noEmit`、`cargo test --manifest-path src-tauri/Cargo.toml` 通过

## Definition of Done

- 子任务不引入 AI 依赖，且能作为后续 `AI Morning Brief` 的直接数据基座
- Today 页面成为“主工作面”，KairosHub 保持为“工具入口页”
- 聚合逻辑和 UI 渲染边界清晰
- 规则引擎的核心判断可被文档和代码解释

## Out of Scope

- 不生成自然语言 AI 摘要
- 不引入 LLM、OpenAI 兼容 provider、API key 管理
- 不新增系统通知
- 不新增 Widget / Smartspace / 后台服务
- 不新增 LZU 专属 Today 任务；LZU 导入后的课程仍通过本地课程表进入 Today 聚合
- 不做多卡片插件系统
- 不在本轮实现“今日专注累计分钟数”或周统计图表
- 不把 Today Briefing 塞进 KairosHub

## Technical Notes

- 预计需要新增一个后端聚合命令，例如 `get_today_briefing`
- 预期影响文件：
  - `src-tauri/src/commands/`
  - `src/App.tsx`
  - `src-tauri/src/lib.rs`
  - `src/components/` 下新增 Today 页面组件
  - `src/components/shared/AppShell.tsx`
  - `src/types/` 下新增 briefing 类型
- 父任务来源：`.trellis/tasks/06-26-competitor-ai-research`
- 本子任务对应父任务里的：`Phase 1: 基石（Today Briefing Card）`
- FasterLZU 后续借鉴点中“今日课程/下一节课”由本任务承接；不要在 `lzu-api-integration` 下重复做一个 LZU-only 今日卡片。
