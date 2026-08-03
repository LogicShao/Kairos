# Design: 今日概览卡片（Today Briefing Card）

## 1. Scope

本任务只实现：

- 一个新的后端聚合命令 `get_today_briefing`
- 一个新的前端类型定义 `TodayBriefingResponse`
- 一个独立的 `Today` 页面
- `Today` 页面上的 Today Briefing Card

不实现：

- AI 摘要生成
- 复杂个性化排序
- 插件化卡片系统
- 通知 / MCP / Provider 选择

## 2. Architecture Decision

### 2.1 采用“单后端聚合命令”而不是前端多次 invoke

不推荐前端在 `Today` 页面中分别调用：

- `get_all_tasks`
- `get_all_exams`
- `get_all_courses`
- `get_pomodoro_state`

再本地拼成一个摘要。

原因：

- 规则判断会散落到 React 中，不利于后续复用给 AI
- 前端需要重复实现“今天/逾期/最近考试”的日期逻辑
- 多次 invoke 增加首页加载噪音

因此本轮定义新的后端聚合响应：

```text
TodayBriefingResponse
├── date
├── weekday_label
├── courses
├── tasks
├── exam
└── pomodoro
```

前端只负责渲染，不做核心业务判断。

### 2.2 规则引擎最小化

本轮规则不是“智能推荐系统”，只是显式编码最小优先级：

- 课程：今天课程数量 + 下一节未开始课程
- 待办：逾期未完成 > 今日到期未完成 > 其他任务不显示
- 考试：选最近一场未开始考试
- 专注：若番茄钟运行中则展示剩余时间，否则展示可开始状态

这符合“基石”定位，避免过早设计推荐系统。

## 3. Data Contract

建议新增前后端一致的数据结构：

```ts
interface TodayBriefingResponse {
  date: string
  weekday_label: string
  courses: {
    today_count: number
    next_course: {
      title: string
      start_time: string
      end_time: string
      location: string
    } | null
  }
  tasks: {
    overdue_count: number
    due_today_count: number
    spotlight: Array<{
      id: number
      title: string
      priority: "high" | "medium" | "low"
      due_date: string | null
    }>
  }
  exam: {
    id: number
    course_name: string
    exam_datetime: string
    days_until: number
    location: string
  } | null
  pomodoro: {
    is_running: boolean
    phase: "work" | "short_break" | "long_break"
    remaining_seconds: number
    completed_sessions: number
  }
}
```

## 4. Backend Design

### 4.1 新增 `get_today_briefing`

建议新增命令：

```rust
#[tauri::command]
pub fn get_today_briefing(...) -> Result<TodayBriefingResponse, String>
```

它内部做：

1. 读取今日日期
2. 查询课程、考试、任务
3. 读取当前 `PomodoroState`
4. 组装结构化返回

### 4.2 课程逻辑

优先复用已有 `courses` / `schedule` 规则，不重新发明课程周次判断。

可行策略：

- 从 `get_all_courses` 拿课程
- 过滤当前学期课程
- 基于今天星期与当前日期判断哪些课程是“今天课程”
- 在今天课程中找 `start_time >= now` 的下一节课

如果周次判断过于分散，可抽出 schedule 层 helper，但不在本轮做大范围重构。

### 4.3 任务逻辑

输入来源：`get_all_tasks` / db tasks

只关注：

- `status != done`
- `due_date < today` → overdue
- `due_date == today` → due today

`spotlight` 排序：

1. `high` 优先级优先
2. 有 `due_date` 的优先于无截止日期
3. 更早的截止日期优先

### 4.4 考试逻辑

输入来源：`get_all_exams`

逻辑：

- 过滤掉已开始/已过去的考试
- 取 `exam_datetime` 最早的一场
- 计算 `days_until`

### 4.5 番茄逻辑

输入来源：当前运行态 `PomodoroEngine`

不新增历史统计逻辑，只返回：

- 是否运行中
- 当前 phase
- 剩余秒数
- completed_sessions

这一步是“状态摘要”，不是“生产力报告”。

## 5. Frontend Design

### 5.1 新增独立 `Today` 页面

Today Briefing 不应继续放在 `KairosHub` 中。

原因：

- `KairosHub` 是工具入口页，不是日常主工作面
- Today Briefing 是高频、决策型信息，应处于主导航的一层
- 放在 `KairosHub` 会把“今天先做什么”埋进二级入口

因此本轮建议：

- 在 `App.tsx` / `AppShell` 导航体系中加入一个独立 `today` 页面
- `Today` 成为和 `calendar` / `todo` / `pomodoro` 同级的工作面
- `KairosHub` 继续承担工具与设置入口职责

### 5.2 Today 页面结构

页面结构建议：

- 顶部：今天日期 + “今日概览”
- 中部：4 个摘要块
- 每个块可点击跳转

可选：

- 页面中暂时只放 Today Briefing Card，一个页面只做一件事
- 不额外叠加设置卡、入口卡、统计图

### 5.3 呈现原则

- 不做长文案
- 不做复杂表格
- 只呈现“最需要知道”的一句信息

例如：

- 今日课程：`今天 2 节课，下一节 10:20 软件工程 @ A402`
- 待办：`1 个逾期，2 个今天到期`
- 最近考试：`离高数期末还有 3 天`
- 专注：`正在专注，剩余 18:24`

### 5.4 空状态

必须有简洁空状态：

- 无课程：`今天没有课程`
- 无紧急待办：`今天暂无到期待办`
- 无考试：`近期没有考试`
- 番茄未运行：`可以开始今天的第一轮专注`

## 6. Tradeoffs

### 方案 A：前端多 invoke 聚合

优点：

- 开发快
- 不加新命令

缺点：

- 规则散落前端
- 不利于后续 AI 复用
- 首页逻辑会迅速变脏

### 方案 B：后端单命令聚合（本轮选择）

优点：

- 结构化输出可复用
- 规则集中
- 更符合“基石”定位

缺点：

- 需要新增类型和命令
- 首轮实现稍重一点

## 7. Rollback

- 若页面位置判断有争议，可先保留后端命令，只替换前端承载位置
- 若后端聚合过重，可退回前端简单聚合，但这应视为降级方案，不是首选
