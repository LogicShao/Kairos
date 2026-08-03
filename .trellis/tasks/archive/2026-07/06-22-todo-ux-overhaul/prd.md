# Todo page UX overhaul: FAB, chips, empty state, bottom sheet

## Goal

聚焦重构待办事项页面本身的 UX：用 FAB 悬浮按钮、胶囊筛选标签、情感化空状态、底部半屏弹窗替代现有实现，提升移动端操作效率与视觉专业度。

## Confirmed Facts (from codebase inspection)

**现有架构：**
- `TaskList.tsx` — 主页面，包含顶部标题栏（带 + 按钮）、两个 `<select>` 筛选器、空状态、任务卡片列表
- `TaskForm.tsx` — 新建/编辑表单，在 `<Modal>` 中渲染
- `modal.tsx` — 基于 Radix Dialog 的居中弹窗，毛玻璃面板 + 缩放动效
- `AppShell.tsx` — 底部有移动端 Tab Bar（16px 高度），内容区 `pb-16` 预留空间
- `AcrylicPanel` — 毛玻璃容器组件，`bg-card/80 backdrop-blur-xl glass-edge`
- 主题系统：`use-theme.ts` 管理 `data-accent` 和 `.dark`，CSS 变量全部有 Chromium 兼容回退

**现有空状态实现：**
- `<AcrylicPanel className="p-8 text-center">` 包裹的白色卡片
- 文案："暂无任务" + "点击'新建'按钮添加第一个任务"

**现有筛选器实现：**
- 两个 `<select>` 元素，`w-full` 并排（sm:flex-row）
- 分别筛选 status 和 priority
- 占用较多纵向空间

**现有新建任务入口：**
- 顶部右侧 `<Button size="sm">`，带 `<Plus>` 图标
- 点击 → Modal 弹窗 → TaskForm

**Modal 现有实现：**
- Radix Dialog Portal，居中定位
- 主题色跟随系统：`bg-card/95`、`border-border/60`
- 内置 close 按钮、focus trap、Esc 关闭

## Requirements

### R1: FAB 悬浮按钮
- 移除顶部标题栏右侧的 + 按钮
- 在页面右下角放置圆形悬浮按钮 (FAB)
- 圆角程度：rounded-full，尺寸 56x56，内部大号 + 图标
- 使用 primary 主题色，带阴影
- 滚动时不跟随，固定定位
- 不与底部 Tab Bar 重叠（在 pb-16 区域内上方）

### R2: 胶囊筛选标签 (Chips)
- 替换两个 `<select>` 元素为并排胶囊按钮
- 每个 Chip 显示当前选中值 + ChevronDown 图标
- 点击弹出筛选选项（后续可改为 Bottom Sheet）
- 紧凑布局：一行内并排，节省纵向空间
- 选中状态有明显的视觉反馈（primary 色高亮）

### R3: 空状态优化
- 去掉外层 AcrylicPanel 白底卡片框
- 内容直接居中显示在页面背景上
- 添加矢量插画（使用 lucide-react 图标组合或 CSS 图形）
- 调整文案为更友好的表达
- 直接在空状态中添加 "添加任务" 操作按钮

### R4: 底部半屏弹窗 (Bottom Sheet)
- 新建/编辑任务表单从居中 Modal 改为底部滑出面板
- 使用 Radix Dialog 或自定义实现
- 圆角顶部、占屏幕 60-75% 高度
- 背景蒙层点击可关闭
- 支持滑动手势关闭（可选，复杂度较高可延后）
- 筛选选项菜单也改用 Bottom Sheet（或至少统一暗色主题）

### R5: 排版与视觉细节
- "待办事项" 标题加大（Large Title 风格）
- 任务列表页增加标题的层级感
- 优先级标签增加颜色圆点视觉辅助
- 本轮不改导航结构，只保证 Todo 页面自身视觉层级更清晰

### R6: 一键完成任务
- 任务卡片必须提供无需进入编辑表单的快速完成入口
- 快速完成入口优先使用类似 Microsoft To Do 的圆形 check affordance，位置在任务标题左侧
- 点击快速完成入口将任务状态更新为 `done`
- 快速完成后任务应从默认待办主视图中消失，形成"完成并移除出当前列表"的体验
- 不将快速完成实现为 `delete_task`；删除仍保留为独立破坏性操作
- 已完成任务可通过状态筛选查看，避免误操作后无法追溯

### Deferred: 专注 / 待办页面设计语言一致性
- `PageShell` 跨页面统一
- 专注页标题与设置入口重排
- Focus / Todo 的统一外壳语言
- 相关工作延期到独立任务，不纳入本轮实现

## Acceptance Criteria

- [ ] AC1: FAB 在页面右下角固定显示，不与 Tab Bar 重叠
- [ ] AC2: 点击 FAB 打开新建任务表单
- [ ] AC3: 胶囊筛选标签替换了 `<select>`，一行内并排显示
- [ ] AC4: 点击 Chip 可弹出筛选选项
- [ ] AC5: 空状态不再使用外层卡片，直接显示在背景上
- [ ] AC6: 空状态包含操作按钮可直接添加任务
- [ ] AC7: 新建/编辑表单使用底部半屏弹窗
- [ ] AC8: Bottom Sheet 在浅色模式下显示浅色背景
- [ ] AC9: 所有改动在 Chromium 110 WebView 上正常渲染
- [ ] AC10: 不破坏桌面端（md+）的侧边栏布局
- [ ] AC11: 未完成任务卡片可一键标记为已完成，无需打开编辑表单
- [ ] AC12: 一键完成调用 `update_task` 更新状态，不调用 `delete_task`
- [ ] AC13: 默认待办主视图不展示已完成任务，已完成任务仍可通过状态筛选查看

## Out of Scope

- 不修改 Todo 后端 API 或数据模型
- 不修改 AppShell 的导航结构
- 不修改 Pomodoro / Focus 页面行为
- 不做跨页面的 `PageShell` 统一
- 手势滑动关闭 Bottom Sheet（低优先级，可延后）
- 横向滚动的多维度筛选标签栏
- 桌面端专属 UX 适配
- 完成后物理删除或软删除任务数据

## Decisions Made

- **D1**: Bottom Sheet 复用 Radix Dialog 改样式（扩展 `Modal` 组件增加 `variant: "center" | "bottom"` prop），不引入新依赖。
- **D2**: 筛选 Chip 弹出菜单用 Popover 浮层（absolute 定位在 Chip 下方），不用 Bottom Sheet。
- **D3**: 空状态插画用 lucide 图标组合（`ListTodo` 48px，`text-muted-foreground/40`），不引入 SVG 文件。
- **D4**: 快速完成采用"完成并移出默认视图"，不绑定删除语义；删除继续由 Trash 操作承担。

## Open Questions

- 无。当前已决定本轮只做 Todo 页面，不处理跨页面一致性问题。
