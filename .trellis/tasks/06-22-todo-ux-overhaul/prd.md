# Todo page UX overhaul: FAB, chips, empty state, bottom sheet

## Goal

重构待办事项页面的 UX：用 FAB 悬浮按钮、胶囊筛选标签、情感化空状态、底部半屏弹窗替代现有实现，提升移动端操作效率与视觉专业度。

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
- 底部导航栏选中状态保持一致性

### R6: 专注 / 待办页面设计语言一致性
- 抽象共享 `PageShell` 组件，统一两页的「页面外壳」层（标题栏 + 入场动画 + 宽度容器 + 间距）
- 专注页新增 `"专注"` 标题，与待办的 `"待办事项"` 标题风格对齐（同字号/字重）
- 专注页设置入口从「圆环右上角浮动图标」上移到统一标题栏的操作槽
- 统一两页入场动画为同一种
- 保留合理差异：内容宽度（专注 md / 待办 2xl）、垂直定位（专注居中 / 待办顶对齐）—— 由内容形态决定
- `PageShell` 设计为可复用，后续可推广到 courses / exams / sync
- 现有功能（计时、任务增删改查）不受影响

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
- [ ] AC11: 专注与待办两页共用 `PageShell`，标题栏字号/字重/动画一致
- [ ] AC12: 专注页设置入口移至标题栏，圆环上方无浮动按钮遮挡
- [ ] AC13: 专注计时功能与待办增删改查功能均不受外壳重构影响

## Out of Scope

- 不修改 Todo 后端 API 或数据模型
- 不修改 AppShell 的导航结构
- 手势滑动关闭 Bottom Sheet（低优先级，可延后）
- 横向滚动的多维度筛选标签栏
- 桌面端专属 UX 适配

## Decisions Made

- **D1**: Bottom Sheet 复用 Radix Dialog 改样式（扩展 `Modal` 组件增加 `variant: "center" | "bottom"` prop），不引入新依赖。
- **D2**: 筛选 Chip 弹出菜单用 Popover 浮层（absolute 定位在 Chip 下方），不用 Bottom Sheet。
- **D3**: 空状态插画用 lucide 图标组合（`ListTodo` 48px，`text-muted-foreground/40`），不引入 SVG 文件。

## Open Questions

- **Q1 (R6 ↔ R1 耦合)**: 待办采用 FAB（R1）后顶部不再有新建按钮，则 `PageShell` 标题栏的 `action` 槽在待办页为空。两页一致性策略需二选一：
  - 选项 A：专注也改用 FAB（右下角设置/开始按钮），与待办彻底对齐，`PageShell` 不需要 `action` 槽
  - 选项 B：保留 `action` 槽（专注放设置按钮），待办页该槽留空或放筛选入口
  - _待用户审查时拍板，再定 `PageShell` 的 `action` 语义。_
