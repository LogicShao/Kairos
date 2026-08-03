# Implementation: 今日概览卡片（Today Briefing Card）

## Preconditions

- 这是父任务 `06-26-competitor-ai-research` 的 Phase 1 基石子任务
- 本轮不接入任何 AI 能力
- 保持工作区其他脏文件不被顺手修改
- FasterLZU 的“今日课程小组件”只作为体验参考；实现仍基于 Kairos 本地课程/日历聚合，不直接调用 LZU API

## Ordered Steps

### 1. 定义契约

- [x] 在 `src-tauri` 后端定义 `TodayBriefingResponse` 及其子结构
- [x] 在 `src/types/` 中镜像同名 TypeScript 类型
- [x] 明确字段只传结构化数据，不传最终文案

验证：

- [x] `cargo check`
- [x] `npx tsc --noEmit`

### 2. 实现后端聚合命令

- [x] 新增 `get_today_briefing`
- [x] 复用现有 tasks / exams / courses / pomodoro 数据源
- [x] 实现最小规则引擎：
  - 课程：今天课程数 + 下一节课
  - 待办：逾期数 + 今日到期数 + spotlight
  - 考试：最近一场
  - 番茄：当前运行态

验证：

- [x] 为规则函数补单元测试
- [x] `cargo test --manifest-path src-tauri/Cargo.toml`

### 3. 接入首页卡片

- [x] 新增独立 `Today` 页面组件
- [x] 在 `App.tsx` / `AppShell` 中接入 `today` 顶级导航
- [x] 在 `Today` 页面中调用 `get_today_briefing`
- [x] 渲染 Today Briefing Card
- [x] 每个摘要块支持跳转到对应页面
- [x] 做 loading / error / empty state

验证：

- [x] `npm run lint`
- [x] `npx tsc --noEmit`

### 4. 视觉打磨

- [x] 保持卡片信息密度高，但不过度堆叠
- [x] 移动端首屏可见
- [x] 与现有主工作面视觉风格一致
- [x] `KairosHub` 不再承载 Today Briefing

### 5. 最终检查

- [x] 不存在 AI 依赖
- [x] 导航结构的变更只限于加入一个 `today` 顶级入口
- [x] 不把 Today Briefing 塞进 KairosHub
- [x] 结构化响应可复用给后续 AI Summary
- [x] LZU API 导入后的课程无需特殊分支，能自然进入 Today 今日课程/下一节课逻辑

## Validation Commands

```powershell
cargo test --manifest-path "src-tauri/Cargo.toml"
npm run lint
npx tsc --noEmit
```

如实现涉及构建链路变化，再补：

```powershell
npm run build
```

Result 2026-07-20:

- `cargo fmt --manifest-path "src-tauri/Cargo.toml"` passed.
- `cargo test --manifest-path "src-tauri/Cargo.toml" commands::briefing` passed: 12 tests passed.
- `cargo clippy --manifest-path "src-tauri/Cargo.toml" --all-targets -- -D warnings` passed.
- `cargo test --manifest-path "src-tauri/Cargo.toml"` passed: 185 tests passed, doctest ignored as configured.
- `npm run lint` passed.
- `npx tsc --noEmit` passed.
- `npm run build` passed; Vite reported the existing >500 kB chunk warning.
- `npx jscpd . --threshold 10 --reporters console --format rust,typescript` passed under threshold: total duplicated lines 8.85%.

## Review Gates

### Gate 1：契约是否真能复用给后续 AI

- 不能把 Today Briefing 做成纯 UI 文案拼接
- 后端必须输出结构化字段

### Gate 2：是否保持“基石”定位

- 不能引入 AI
- 不能把首页做成大仪表盘
- 不能把任务扩成复杂推荐系统
- 不能把 Today 信息藏进工具入口页

### Gate 3：是否真的帮助“今天先做什么”

- 信息必须聚焦，而不是泛信息罗列

## Risks

- 课程周次判断可能需要复用现有 schedule 逻辑，避免出现首页和课程页不一致
- 若 Today Briefing 规则写在前端，会削弱后续 AI 复用价值
- 若 Today 页面承载过多附加内容，会失去“今天先做什么”的聚焦感
