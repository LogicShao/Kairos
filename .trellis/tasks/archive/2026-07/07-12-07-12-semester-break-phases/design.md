# 方案 1+ 技术设计

## 架构决策

**方案**: 学期上下文扩展 + term_phases 多阶段子表（方案 1+）

**核心原则**:
- 不修改 `semester_context` 表结构，通过 1:N 子表扩展
- 判定优先级: `term_phases` 精确匹配 > 自动推断 fallback
- 向后兼容: 无 phases 数据时行为不变

## 数据模型

### term_phases (v9 迁移)

```sql
CREATE TABLE term_phases (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    sync_id TEXT NOT NULL,
    term_label TEXT NOT NULL,          -- FK-like: 关联 semester_context.term_label
    phase_type TEXT NOT NULL CHECK(phase_type IN ('teaching', 'exam', 'break')),
    start_week INTEGER NOT NULL CHECK(start_week >= 1),
    end_week INTEGER NOT NULL CHECK(end_week >= start_week),
    affects_courses INTEGER NOT NULL DEFAULT 1,
    affects_exam_notifications INTEGER NOT NULL DEFAULT 1,
    pomodoro_profile TEXT NOT NULL DEFAULT 'default',
    notification_rules_json TEXT NOT NULL DEFAULT '{}',
    sort_order INTEGER NOT NULL DEFAULT 0,
    deleted_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX idx_term_phases_term_label ON term_phases(term_label, sort_order);
CREATE UNIQUE INDEX idx_term_phases_sync_id ON term_phases(sync_id)
    WHERE sync_id IS NOT NULL AND sync_id != '';
```

### pomodoro_profiles (v9 迁移)

```sql
CREATE TABLE pomodoro_profiles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    work_seconds INTEGER NOT NULL DEFAULT 1500,
    short_break_seconds INTEGER NOT NULL DEFAULT 300,
    long_break_seconds INTEGER NOT NULL DEFAULT 900,
    sessions_before_long_break INTEGER NOT NULL DEFAULT 4,
    is_builtin INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- 内置数据
INSERT INTO pomodoro_profiles (name, work_seconds, short_break_seconds, long_break_seconds, sessions_before_long_break, is_builtin, created_at, updated_at)
VALUES
  ('default',  1500, 300, 900,  4, 1, datetime('now'), datetime('now')),
  ('intense',  3000, 600, 1800, 3, 1, datetime('now'), datetime('now')),
  ('relaxed',  1500, 600, 1200, 4, 1, datetime('now'), datetime('now'));
```

## 核心模块: term_phase.rs

### 判定流程

```
get_current_phase_status(conn, source="lzu")
│
├─ 1. get_latest_semester_context_by_source(conn, "lzu")
│     └─ None → CurrentPhaseStatus { phase_type: "unknown", ... }
│
├─ 2. compute_current_week(ctx.start_date) → current_week
│
├─ 3. get_term_phase_by_week(conn, ctx.term_label, current_week)
│     └─ Some(phase) → 映射为 CurrentPhaseStatus
│
└─ 4. Fallback: infer_phase(ctx.term_label, current_week, ctx.total_weeks)
      ├─ current_week <= total_weeks.unwrap_or(20) → teaching
      └─ 否则 → break
```

### compute_current_week 复用

与 `schedule.rs::current_week_index_from_start()` 逻辑一致: `(today - start_date).days / 7 + 1`, 最小为 1。

## 通知集成点

### exam_scheduler.rs 修改

在 `schedule_exam_notifications()` 函数开头插入:

```rust
let phase = crate::term_phase::get_current_phase_status(conn, "lzu")?;
if !phase.exam_notifications_enabled {
    log::info!("exam notifications suppressed in phase: {}", phase.phase_type);
    cancel_all_exam_notifications()?;
    return Ok(());
}
```

触发重新调度的时机（已有逻辑）:
- 通知配置更新 (`update_notification_config`)
- 应用启动
- 考试 CRUD

**新增触发**: `create_term_phase` / `update_term_phase` / `delete_term_phase` 后调用 `schedule_exam_notifications`

## schedule.rs 修改

`WeekScheduleResponse` 追加两个字段:
```rust
pub phase_type: String,      // "teaching" | "exam" | "break"
pub courses_visible: bool,   // false 时前端显示假期视图
```

`build_week_schedule()` 逻辑:
```rust
let phase = term_phase::get_current_phase_status(conn, "lzu")?;

if !phase.courses_visible {
    return WeekScheduleResponse {
        courses_visible: false,
        phase_type: phase.phase_type,
        items: vec![],
        // ... 其他字段照常填充
    };
}
// 原有课程过滤逻辑不变
```

## WebDAV 同步

`SyncData` 新增:
```rust
pub term_phases: Vec<TermPhase>,
```

导出/导入复用现有 LWW 合并管线（按 sync_id 匹配）。

## 前端类型

位置: `src/types/notification.ts` 追加（保持与现有 notification 类型同文件，因为是相关领域）

```typescript
export interface TermPhase { ... }
export interface CurrentPhaseStatus { ... }
export interface PomodoroProfile { ... }
export interface CreateTermPhaseRequest { ... }
export interface UpdateTermPhaseRequest { ... }
```

## 前端组件

### SemesterPhaseSettings.tsx (新增)

- AcrylicPanel 卡片布局，与 NotificationSettings 风格一致
- 学期选择器: 从现有 semester_context 列表选择
- 阶段时间线: 水平分段条，颜色区分类型（蓝=教学, 红=考试, 灰=假期）
- 编辑弹窗: 修改阶段类型/起止周/行为开关/profile
- Profile 管理子面板: 列表 + 创建/删除

### CourseSchedule.tsx 修改

```tsx
// 当 weekData?.courses_visible === false 时
if (!weekData?.courses_visible) {
  const labels = { teaching: "教学周", exam: "考试周", break: "假期" }
  return <PhaseEmptyState phase={weekData.phase_type} label={labels[weekData.phase_type]} />
}
```

## 文件变更总览

| 操作 | 文件 |
|------|------|
| 修改 | `src-tauri/src/db/migrations.rs` |
| 修改 | `src-tauri/src/db/models.rs` |
| 修改 | `src-tauri/src/db/mod.rs` |
| **新增** | `src-tauri/src/db/term_phases.rs` |
| **新增** | `src-tauri/src/db/pomodoro_profiles.rs` |
| **新增** | `src-tauri/src/term_phase.rs` |
| 修改 | `src-tauri/src/lib.rs` |
| 修改 | `src-tauri/src/notifications/exam_scheduler.rs` |
| 修改 | `src-tauri/src/schedule.rs` |
| 修改 | `src-tauri/src/commands/briefing.rs` |
| 修改 | `src-tauri/src/commands/courses.rs` |
| 修改 | `src-tauri/src/sync/exporter.rs` |
| 修改 | `src/types/notification.ts` |
| 修改 | `src/types/schedule.ts` |
| 修改 | `src/types/briefing.ts` |
| **新增** | `src/components/settings/SemesterPhaseSettings.tsx` |
| 修改 | `src/components/courses/CourseSchedule.tsx` |
| 修改 | `src/components/calendar/CalendarView.tsx` |
| 修改 | `src/pages/today/TodayPage.tsx` |

总计: 4 新增文件, 14 修改文件
