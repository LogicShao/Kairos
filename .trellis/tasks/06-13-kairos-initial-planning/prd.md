# Kairos Initial Planning and Architecture

## Goal

构建跨平台时间管理桌面应用 **Kairos**（καιρός — "恰当时机"），使用 Rust + Tauri 技术栈，优先支持 Windows 11 和 Arch Linux + Hyprland。集成番茄钟、TODO、课程表、考试倒计时四大核心功能，支持 WebDAV 自建服务同步。

## What I Already Know

- 项目名称：Kairos
- 技术栈：Rust (Tauri v2) + React + TypeScript
- GUI 框架：Tauri v2
- 前端框架：React + TypeScript
- 本地存储：SQLite (rusqlite)
- 服务器同步：WebDAV 协议
- 目标平台：Windows 11 + Arch Linux (Hyprland)
- MVP 范围：全部四个功能模块一起开发
- 开发目录：`~/project/kairos/`

## Requirements

### 1. 番茄钟 (Pomodoro Timer)
- 可配置的工作/休息时长（默认 25min 工作 / 5min 短休 / 15min 长休）
- 可配置长休触发间隔（默认 4 个番茄后长休）
- 视觉倒计时显示（环形进度条 + 数字）
- 系统托盘显示剩余时间
- 桌面通知（工作结束 / 休息结束）
- 会话日志记录（开始/结束时间、类型、关联任务）

### 2. TODO 管理
- 任务 CRUD（创建、读取、更新、删除）
- 优先级标记（高/中/低）
- 截止日期
- 状态流转（待办 → 进行中 → 已完成）
- 标签/分类支持
- 与番茄钟关联（任务 → 番茄会话）

### 3. 课程表
- 周视图网格布局
- 课程 CRUD（名称、星期、起止时间、地点、教师、颜色）
- 学期管理（当前学期切换）
- 颜色标记区分课程

### 4. 考试倒计时
- 考试列表 CRUD
- 距今天数倒计时
- 考前提醒通知
- 关联课程信息

### 5. WebDAV 数据同步
- 服务器 URL + 凭据配置
- 手动/自动同步切换
- 数据导出为结构化 JSON → PUT 到 WebDAV
- 启动时 GET JSON → 比对 `updated_at` → 合并
- 冲突策略：最后写入胜出 + 手动解决界面

## Acceptance Criteria

- [ ] 番茄钟可正常启动/暂停/重置，通知正常弹出
- [ ] TODO 可完整 CRUD，可按优先级/截止日期排序
- [ ] 课程表周视图正确渲染，课程颜色正确
- [ ] 考试列表显示倒计时天数，临近考试触发通知
- [ ] WebDAV 同步可成功上传/下载数据，冲突时可手动选择
- [ ] 系统托盘图标显示，右键菜单可操作
- [ ] Windows 11 和 Arch+Hyprland 均可正常编译运行

## Definition of Done

- TypeScript 类型覆盖完整，无 `any`
- Rust 端无 `unwrap()` 裸调用，全部使用 `Result` 传播
- SQLite schema 有 migration 机制
- 前端组件有基本单元测试
- Tauri 构建通过 (`cargo tauri build`)
- 用户文档（README 含安装/配置说明）

## Technical Approach

### 架构分层

```
┌──────────────────────┐
│   React Frontend     │  TypeScript + Zustand + Tailwind + shadcn/ui
├──────────────────────┤
│   Tauri IPC Bridge   │  #[tauri::command] → invoke()
├──────────────────────┤
│   Rust Core Logic    │  业务逻辑 (timer, task mgmt, schedule)
├──────────────────────┤
│   SQLite Storage     │  rusqlite + migrations
├──────────────────────┤
│   WebDAV Sync        │  reqwest + XML parsing
└──────────────────────┘
```

### 数据模型 (SQLite)

**pomodoro_config**: id, work_seconds, short_break_seconds, long_break_seconds, sessions_before_long_break
**pomodoro_sessions**: id, started_at, ended_at, session_type(work/short_break/long_break), task_id(FK nullable)
**tasks**: id, title, description, status(todo/in_progress/done), priority(high/medium/low), due_date, tags(json), created_at, updated_at
**courses**: id, name, day_of_week(1-7), start_time, end_time, location, teacher, color, semester, created_at, updated_at
**exams**: id, course_name, exam_datetime, location, notes, course_id(FK nullable), created_at, updated_at

### 项目结构

```
~/project/kairos/
├── src-tauri/          # Rust 后端
│   ├── src/
│   │   ├── main.rs         # Tauri 入口
│   │   ├── lib.rs           # 命令注册
│   │   ├── db/              # SQLite 模块
│   │   ├── sync/            # WebDAV 同步模块
│   │   └── commands/        # Tauri IPC handlers
│   └── Cargo.toml
├── src/                # React 前端
│   ├── components/         # UI 组件
│   ├── hooks/              # 自定义 hooks
│   ├── stores/             # Zustand stores
│   └── types/              # TypeScript 类型
└── .trellis/           # Trellis 工作流
```

## Decision (ADR-lite)

**Context**: 需要选择一个 Rust 跨平台 GUI 框架
**Decision**: Tauri v2 — 使用系统原生 WebView，打包体积小（<10MB），Windows/Linux 均支持良好，Rust 后端 + Web 前端的分离架构成熟
**Consequences**: 前端需要用 Web 技术栈（React/TS），不适合纯原生 UI 需求场景。WebView 在各平台的渲染一致性需要测试。

## Research References

(待 Phase 1.2 研究后填充)

## Out of Scope

- macOS 平台支持（后续迭代）
- 移动端（iOS/Android）
- 多人协作/团队功能
- AI 智能排程
- iCalendar / CalDAV 集成
- 语音提醒
- 用户系统/多账户

## Edge Cases & Decisions

- **休眠/锁屏**：番茄钟暂停计时，唤醒后从暂停处继续
- **WebDAV 不可达**：本地正常使用，标记"待同步"状态，恢复后自动重试
- **SQLite 并发**：Tauri 单进程模型无并发风险，启用 WAL 模式优化性能
- **课程-考试关联**：`exams.course_id` FK 关联 `courses.id`，删除课程时级联置空
- **同步冲突**：最后写入胜出（LWW），提供手动解决 UI

## Technical Notes

- Tauri v2 需要 Rust 1.77+ 和系统 WebView 依赖
- Arch Linux 需要 `webkit2gtk-4.1` 包
- Windows 11 需要 WebView2 Runtime（通常预装）
- 项目在 `~/project/kairos/` 下，已初始化 git + Trellis
