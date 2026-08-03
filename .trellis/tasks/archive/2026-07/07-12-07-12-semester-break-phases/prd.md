# 寒暑假/非学期多阶段模型

## Goal

在现有学期系统（`semester_context`）基础上，引入 `term_phases` 多阶段模型，自动识别教学周、考试周、假期三种阶段，并在通知、番茄钟、课程表三个模块中触发对应的行为切换。

## 背景

当前 Kairos 通过 `semester_context` 表管理学期锚点，课程和考试通过周次规则（`week_pattern`）控制可见性。但系统没有显式的"假期"概念——只是超出周次范围的课程不显示，考试提醒仍然触发。用户需要：

1. 假期期间自动暂停考试/课程通知
2. 考试周自动切换到高强度番茄钟配置
3. 课程表在非教学周显示合适的空状态
4. 支持手动调整阶段边界（混合模式）

## Requirements

### R1: 学期阶段数据模型

- `term_phases` 表：将一个学期（term_label）划分为多个阶段
- 每个阶段有：类型（teaching/exam/break）、起止周、行为开关、番茄钟 profile
- 同一学期内允许多个同类型阶段（如国庆穿插的 break）
- 支持 WebDAV 同步（含 sync_id + 墓碑）

### R2: 番茄钟 Profile 系统

- `pomodoro_profiles` 表：命名的番茄钟配置预设
- 内置 3 个 profile：default（25/5/15/4）、intense（50/10/30/3）、relaxed（25/10/20/4）
- 用户可创建自定义 profile
- 内置 profile 不可删除（is_builtin 标记）

### R3: 核心判定引擎

- `get_current_phase_status(source)` 命令：根据当前日期和学期上下文返回当前阶段
- 判定优先级：`term_phases` 表匹配 > 自动推断 fallback
- Fallback 规则：`current_week <= total_weeks` → teaching，否则 → break
- 无学期上下文时返回 `phase_type = "unknown"`

### R4: 通知行为控制

- 考试通知：仅 teaching 和 exam 阶段发送，break 阶段静默
- 番茄钟通知：不受阶段影响（始终发送）
- 阶段切换时自动重新调度考试通知

### R5: 课程表假期视图

- `build_week_schedule()` 在 `courses_visible = false` 时返回空课程列表
- 前端课程表显示阶段标签（"考试周" / "假期"）而非空白
- 日历视图同样过滤非教学周的课程事件

### R6: 今日概览集成

- `get_today_briefing()` 响应追加当前阶段信息
- 今日页顶部显示阶段标签

### R7: 前端配置 UI

- 学期阶段设置页：可视化时间线，拖拽调整阶段边界
- 番茄钟 Profile 管理：查看/创建/删除自定义 profile
- 设置入口：通知设置附近增加"学期阶段"导航项

### R8: LZU 导入自动生成

- 导入 LZU 课程时自动调用 `ensure_default_phases`：
  - 周 1..(total_weeks-2) → teaching
  - 周 (total_weeks-1)..total_weeks → exam
  - 之后 → break（自动推断）

## Constraints

- 离线优先：阶段判定不依赖网络
- 向后兼容：无 term_phases 记录时，系统行为与当前一致
- 数据一致性：`reset_all_semester_start_dates` 后 phases 的周序号仍然有效（周序号是相对值）
- 不修改现有 `semester_context` 表结构（纯扩展）

## Acceptance Criteria

- [x] `term_phases` 和 `pomodoro_profiles` 表创建成功，迁移通过
- [x] 3 个内置 pomodoro profile 自动初始化
- [x] `get_current_phase_status` 在无 phases 记录时返回正确的 fallback 结果
- [x] 创建 teaching→exam→break 三个阶段后，判定引擎返回正确的当前阶段
- [x] break 阶段不发送考试通知（`schedule_exam_notifications` 静默返回）
- [x] 课程表在 break 阶段返回 `courses_visible: false` + phase 标签
- [x] 今日概览包含 phase 信息
- [x] 前端可以 CRUD term_phases 和 pomodoro_profiles
- [x] WebDAV 同步包含 term_phases 数据
- [x] LZU 导入自动生成默认 phases
- [x] 所有现有测试通过
- [x] `grep -r 'https\?://' src/` 0 匹配（不含注释）
- [x] `cargo test` 全部通过

## Out of Scope

- 自动切换番茄钟 profile（本次仅提供数据基础，前端可手动切换或后续迭代自动切换）
- AI 智能推断假期（基于日历/天气等）
- 移动端（Android）专项适配
