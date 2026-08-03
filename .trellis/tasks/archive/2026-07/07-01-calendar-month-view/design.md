# 日历月视图技术设计

## Scope

本任务主要落在 `src/components/calendar/CalendarView.tsx`，必要时可在同目录拆分纯展示子组件。默认不修改 Rust 后端和 `src/types/schedule.ts`，继续复用 `CalendarWeekCmd` / `CalendarWeekResponse`。

## Current State

- `CalendarView` 持有 `weekStartDate`、`viewMode`、`selectedDayIndex`、`weekData`。
- `viewMode` 只有 `"week" | "day"`。
- 周/日视图共享一次 `get_calendar_week` 查询结果。
- 日期工具函数已经有 `todayKey`、`dayCellKey`、`startOfWeekKey`、`addDaysToKey`、`formatMonthLabel`。
- `CalendarEvent.day_of_week` 是周内 1-7，并不携带绝对日期；月视图需要将事件与其所属周起始日期一起归并，不能只用 `day_of_week`。

## Proposed Approach

采用“前端月范围编排 + 后端周契约复用”的 MVP：

1. 新增 `CalendarMode = "month" | "week" | "day"`。
2. 新增 `visibleMonthDate` 或等价状态，作为月视图自然月锚点。
3. 根据目标月生成月历网格：
   - 取当月 1 日所在周的周一作为 grid start。
   - 取当月最后一天所在周的周日作为 grid end。
   - 生成 35 或 42 个日期单元。
4. 对覆盖到的每个周一调用现有 `get_calendar_week`。
5. 将每周返回的 `events` 映射为 `dateKey -> CalendarEvent[]`：
   - 绝对日期由 `week.week_start_date + (event.day_of_week - 1)` 得到。
   - 同一天事件按任务、考试、课程分组或按已有日视图规则排序。
6. 月视图展示紧凑摘要；点击日期设置选中日期，并切换到 `day`。
7. 日/周视图继续使用当前选中周的数据；从月视图进入日视图时将 `weekStartDate` 同步到所选日期所在周，并设置对应 `selectedDayIndex`。

## Data Flow

```text
viewMode/month anchor
  -> build month grid week starts
  -> invoke get_calendar_week for each week_start_date
  -> CalendarWeekResponse[]
  -> monthEventsByDate
  -> MonthGrid
  -> date click
  -> weekStartDate + selectedDayIndex + viewMode="day"
```

## Component Shape

- 保留现有 `CalendarWeekTimetable` 和 `EventCard`。
- 新增 `CalendarMonthGrid` 纯展示组件，props 只接收已计算好的日期单元、事件映射和回调。
- 如果 `CalendarView.tsx` 继续膨胀，优先在 `src/components/calendar/` 内抽出 `date-utils.ts` 或 `CalendarMonthGrid.tsx`，避免把跨模块内部组件放到 `shared/`。

## State Model

- `viewMode`: `"month" | "week" | "day"`。
- `weekStartDate`: 周/日视图当前周锚点，仍是单周加载的事实来源。
- `visibleMonthKey` 或 `visibleMonthDate`: 月视图当前自然月，格式可内部使用 `YYYY-MM-01`。
- `selectedDateKey`: 推荐替代单独的 `selectedDayIndex` 作为跨月/跨周选择源；实现时可先保留 `selectedDayIndex`，但要保证从日期能反推周起始和周内索引。
- `monthWeeksData`: 月视图查询结果，建议独立于 `weekData`，避免月视图多周数据污染周/日视图。

## Loading And Error

- 周/日视图沿用当前 `loading/error/weekData`。
- 月视图使用独立 `monthLoading/monthError/monthWeeksData`，避免切换视图时把周视图清空。
- 月视图 4-6 个周查询可并行执行；任一周失败时显示月视图错误和重试按钮。
- 如果所有周都成功但事件为空，显示“本月暂无安排”的空状态。

## UX Constraints

- 桌面端可以展示更多事件标题；移动端每格限制摘要数量，超过显示 `+N`。
- 日期格使用稳定高度或最小高度，避免事件数量改变导致布局跳动过大。
- 事件摘要不直接承担完整信息展示；完整详情仍由日视图负责。
- 顶部切换控件使用三段式，不能挤压标题；窄屏允许换行。

## Tradeoffs

### 复用周接口（推荐）

优点：
- 无后端改动，契约稳定，风险低。
- 符合当前 `CalendarWeekCmd.week_start_date` 设计。
- 可快速交付 MVP。

缺点：
- 月视图需要 4-6 次 IPC。
- 需要前端处理多周加载和日期归并。

### 新增后端月接口（暂不采用）

优点：
- 一次 IPC 返回完整月数据，前端更简单。
- 后续可统一处理跨周排序和聚合性能。

缺点：
- 需要新增 Rust command、类型镜像和测试。
- 当前需求未证明必须新增后端契约，容易扩大范围。

## Branch Strategy

- 当前任务建议分支：`feat/calendar-month-view`。
- 基准分支：当前工作分支 `feat/today-briefing`，因为任务元数据创建时该分支已是当前上下文，且日历页已有未提交改动。
- 不建议在当前 dirty worktree 直接切分支；应先处理或暂存现有 13+ 个未提交变更，再从 `feat/today-briefing` 创建任务分支。
- 建议提交粒度：
  1. `feat(calendar): add month view state and data aggregation`
  2. `feat(calendar): render responsive month grid`
  3. `test/check(calendar): validate month navigation and empty states`（如有测试或验证脚本变更）
- 不在规划阶段执行 `git checkout`、`git branch`、`git commit`。

## Rollback

- 若月视图实现导致风险过大，可删除 `month` 分支逻辑和 `CalendarMonthGrid`，恢复 `CalendarMode` 为 `"week" | "day"`。
- 若周接口聚合无法满足要求，回到 planning：补充后端 `get_calendar_month` 需求、设计和测试矩阵后再实现。
