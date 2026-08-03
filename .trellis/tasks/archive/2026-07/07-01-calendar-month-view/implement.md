# 日历月视图实施计划

## Preconditions

- 当前任务保持 `planning`，只有在用户确认后才执行：
  ```bash
  python ./.trellis/scripts/task.py start 07-01-calendar-month-view
  ```
- 实现前加载 `trellis-before-dev`，读取前端相关规范。
- 处理当前 dirty worktree 的边界：不要把无关未提交变更混入本任务提交。

## Checklist

- [ ] 读取并确认任务产物：`prd.md`、`design.md`、`implement.md`。
- [ ] 读取前端规范：
  - `.trellis/spec/frontend/index.md`
  - `.trellis/spec/frontend/component-guidelines.md`
  - `.trellis/spec/frontend/quality-guidelines.md`
  - `.trellis/spec/frontend/type-safety.md`
  - `.trellis/spec/guides/code-reuse-thinking-guide.md`
- [ ] 梳理 `CalendarView.tsx` 中日期工具和可复用展示组件，避免重复实现。
- [ ] 扩展 `CalendarMode` 为三种视图，更新顶部三段式切换。
- [ ] 引入月锚点和日期选择状态，保证月/周/日视图导航语义一致。
- [ ] 实现月网格日期生成函数：
  - 输入目标月
  - 输出 35/42 个日期单元
  - 周一开始，跨月日期带 `inCurrentMonth` 标记
- [ ] 实现月视图数据加载：
  - 计算覆盖周的 `week_start_date`
  - 并行调用 `get_calendar_week`
  - 归并为 `dateKey -> CalendarEvent[]`
  - 覆盖 loading/error/empty
- [ ] 实现 `CalendarMonthGrid`：
  - 桌面端展示紧凑事件标题
  - 移动端限制摘要数量并显示更多计数
  - 今天、选中日期、非当前月日期状态清晰
  - 点击日期进入日视图
- [ ] 更新日视图数据选择逻辑，使从月视图进入跨周日期时能加载并展示正确周数据。
- [ ] 手动验证：
  - 当前月、有事件月、无事件月
  - 跨月周补齐
  - 从月视图点击日期进入日视图
  - 视图切换不丢失合理选择状态
  - <768px 宽度布局
- [ ] 运行质量命令：
  ```bash
  npm run lint
  npx tsc --noEmit
  ```
- [ ] 使用 `trellis-check` 进行最终质量检查。
- [ ] 使用 `trellis-update-spec` 判断是否需要沉淀新约定。

## Review Gates

- 不新增后端接口，除非先更新 `prd.md` 和 `design.md`。
- 不引入外部网络请求。
- 不复制 `CalendarEvent` / `CalendarWeekResponse` 类型。
- 不把月视图事件摘要做成完整详情页；完整详情仍在日视图承载。
- 不提交与月视图无关的已有 dirty 文件。

## Validation Commands

```bash
npm run lint
npx tsc --noEmit
```

如后续补充组件测试或端到端截图验证，应把命令追加到本文件。
