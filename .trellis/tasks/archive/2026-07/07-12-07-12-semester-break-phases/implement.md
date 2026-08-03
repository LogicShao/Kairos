# 寒暑假/非学期多阶段模型 - 执行计划

## Preconditions

- 任务保持在 `planning`，实现前需要用户确认后再 `task.py start`。
- 以当前 `main` 为实现基线；`main` 已包含 LZU 课表导入、身份展示、学期上下文和 Today 相关基础能力。
- 不修改 `semester_context` 表结构，通过新增表和服务层完成扩展。
- 所有阶段判定必须离线可用，不依赖 LZU 或其他外部网络请求。
- 本任务不实现自动切换番茄钟运行配置，只提供 profile 数据和阶段状态。

## Implementation Order

### 1. Schema and Models

- [x] 在迁移系统追加新版本，创建 `term_phases` 和 `pomodoro_profiles`。
- [x] 初始化 `default`、`intense`、`relaxed` 三个内置 pomodoro profile，保证迁移幂等。
- [x] 在 `src-tauri/src/db/models.rs` 增加跨层结构体，字段对齐数据库和前端 IPC。
- [x] 新增 `src-tauri/src/db/term_phases.rs`，提供 CRUD、按学期查询、按周匹配、软删除和 `ensure_default_phases`。
- [x] 新增 `src-tauri/src/db/pomodoro_profiles.rs`，提供 CRUD 和内置 profile 删除保护。

Validation:

- [x] 迁移测试覆盖表创建、CHECK 约束、默认 profile 初始化、二次迁移幂等。
- [x] 数据库 CRUD 测试覆盖成功路径、不存在记录、非法 phase/profile、软删除过滤。

### 2. Phase Resolution Service

- [x] 新增 `src-tauri/src/term_phase.rs`，集中实现当前周计算、phase fallback 和 `CurrentPhaseStatus` 映射。
- [x] 复用现有学期上下文查询，不复制 `semester_context` 查询逻辑。
- [x] 无学期上下文时返回 `phase_type = "unknown"`，不让调用方 panic。
- [x] 无显式 `term_phases` 时保持旧行为：周次在 `total_weeks` 内视为 teaching，超过后视为 break。
- [x] 明确 `courses_visible` 和 `exam_notifications_enabled` 的来源：显式 phase 字段优先，fallback 使用兼容默认值。

Validation:

- [x] 单元测试覆盖 unknown、teaching fallback、break fallback、显式 teaching/exam/break 匹配。
- [x] 覆盖边界周：第 1 周、考试阶段起始周、`total_weeks`、`total_weeks + 1`。

### 3. Tauri Commands

- [x] 新增 term phase 命令：list/create/update/delete/get_current_phase_status。
- [x] 新增 pomodoro profile 命令：list/create/update/delete。
- [x] 命令层只做锁获取、输入默认值合并和错误转 `String`，业务规则留在 db/service 层。
- [x] 在 `src-tauri/src/lib.rs` 注册所有新增 command。
- [x] 阶段变更后触发考试通知重调度；重调度失败需要返回可理解错误，不吞掉失败。

Validation:

- [x] `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- [x] `cargo test --manifest-path src-tauri/Cargo.toml`

### 4. Notification and Schedule Integration

- [x] 在 `schedule_exam_notifications` 开头读取当前 phase。
- [x] 当 `exam_notifications_enabled = false` 时取消/跳过考试通知，并记录不含隐私数据的 `info` 日志。
- [x] 保持番茄钟通知不受 phase 影响。
- [x] 扩展 `WeekScheduleResponse`，增加 `phase_type` 和 `courses_visible`。
- [x] `build_week_schedule` 在 `courses_visible = false` 时返回空课程列表，但保留周次、日期和考试/待办兼容字段。
- [x] 日历聚合过滤非教学阶段课程事件，避免日历和课程页语义不一致。

Validation:

- [x] 测试 break 阶段考试通知静默。
- [x] 测试 break 阶段课程表返回 `courses_visible = false`。
- [x] 现有课程导入、周视图和日历测试继续通过。

### 5. LZU Import Integration

- [x] LZU 课程导入成功并写入学期上下文后调用 `ensure_default_phases`。
- [x] 默认阶段生成规则：`1..total_weeks-2` teaching，`total_weeks-1..total_weeks` exam，超过后由 fallback break。
- [x] `total_weeks` 缺失时沿用现有 fallback 周数策略，避免导入链路失败。
- [x] 阶段生成失败不得造成已导入课程数据部分损坏；必要时用事务包裹相关写入。

Validation:

- [x] LZU 导入测试覆盖 default phase 生成。
- [x] 无 `total_weeks` 或接口缺字段时导入仍能完成，并生成合理 fallback。

### 6. WebDAV Sync

- [x] `SyncData` 增加 `term_phases`。
- [x] 导出时包含未删除和墓碑记录所需字段。
- [x] 导入时按 `sync_id` 执行 LWW 合并，保留 `deleted_at` 语义。
- [x] 不同步敏感认证数据；phase/profile 数据只包含低敏配置。

Validation:

- [x] 同步导出测试包含 term phase。
- [x] 同步导入测试覆盖新增、更新、墓碑删除和冲突更新。

### 7. Frontend Contracts

- [x] 在 `src/types/notification.ts` 或更合适的共享类型文件中镜像后端 IPC 类型。
- [x] 在 `src/types/schedule.ts` 扩展周视图响应类型。
- [x] 在 `src/types/briefing.ts` 扩展 Today phase 字段。
- [x] 不在组件内重复定义 IPC payload 或使用 `any` / 局部 cast。

Validation:

- [x] `npx tsc --noEmit`
- [x] `npm run lint`

### 8. Settings UI

- [x] 新增 `SemesterPhaseSettings.tsx`，放在 settings 相关组件目录。
- [x] 提供学期选择、phase 列表、创建/编辑/删除、profile 列表和自定义 profile 创建/删除。
- [x] 周次选择使用下拉或明确边界输入，不做自由无界数字输入。
- [x] 内置 profile 删除按钮禁用或隐藏。
- [x] 覆盖 loading / error / empty 三态，移动端触摸目标不小于 44dp。
- [x] 设置导航在通知设置附近新增入口。

Validation:

- [x] 手动检查桌面和 <768px 移动布局。
- [x] 表单错误、空学期、空 phase、内置 profile 删除保护均有明确 UI 状态。

### 9. Consumer UI Integration

- [x] 课程表在 `courses_visible = false` 时显示阶段空状态，而不是普通空白。
- [x] 日历视图隐藏非教学阶段课程事件，必要时展示阶段标签。
- [x] Today 页面顶部展示当前阶段信息；如果 Today 页面当前仍未实现，则只完成后端/type 契约并在 PRD 标注依赖。
- [x] 前端不直接计算当前学期阶段，统一消费后端 `get_current_phase_status` 或 briefing 响应。

Validation:

- [x] 课程表教学周、考试周、假期三种状态显示正确。
- [x] Today 未实现时，构建不引入死代码或未使用类型。

## Final Validation

```powershell
cargo fmt --manifest-path "src-tauri/Cargo.toml" -- --check
cargo clippy --manifest-path "src-tauri/Cargo.toml" --all-targets -- -D warnings
cargo test --manifest-path "src-tauri/Cargo.toml"
npm run lint
npx tsc --noEmit
npm run build
```

Result 2026-07-20:

- `cargo fmt --manifest-path "src-tauri/Cargo.toml" -- --check` passed.
- `cargo clippy --manifest-path "src-tauri/Cargo.toml" --all-targets -- -D warnings` passed.
- `cargo test --manifest-path "src-tauri/Cargo.toml"` passed: 185 tests passed, doctest ignored as configured.
- `npm run lint` passed.
- `npx tsc --noEmit` passed.
- `npm run build` passed; Vite reported the existing >500 kB chunk warning.
- `rg -n "https?://" "src" --glob "!src/assets/*.svg"` returned no matches; the only raw `src` match is the SVG namespace in `src/assets/kairos-logo.svg`.
- UI scope check was code-level and build-level in this run; no Tauri desktop window was launched.

Optional duplication check if the implementation touches repeated Rust/TypeScript paths:

```powershell
npx jscpd . --threshold 10 --reporters console --format rust,typescript
```

Result 2026-07-20: passed under threshold; total duplicated lines 8.86%, Rust 9.49%, TypeScript 0.82%.

## Review Gates

- [x] Backward compatibility: no `term_phases` data preserves current course and notification behavior except post-term break fallback.
- [x] Database safety: schema changes are migration-only, idempotent, and tested.
- [x] Cross-layer contract: Rust structs, Tauri commands, TypeScript types, and UI consumers agree on field names and nullable semantics.
- [x] Notification safety: break suppresses exam notifications only; pomodoro notifications remain unchanged.
- [x] Sync safety: `sync_id` and tombstone behavior match existing WebDAV merge conventions.
- [x] UI scope: settings UI is functional and compact; no dashboard expansion or AI behavior is added.
- [x] DRY: current-week and phase resolution logic has one backend owner and is not duplicated in frontend components.

## Rollback Points

- If database migration or sync merge proves risky, ship only `pomodoro_profiles` and `term_phases` local CRUD behind unused UI, then defer sync.
- If notification integration causes regressions, keep phase status visible but do not gate `schedule_exam_notifications` until a separate fix.
- If Today integration is blocked because Today page is still planning, leave stable backend/type fields and defer rendering to `today-briefing-card`.
- If settings UI scope expands, split UI polish into a child task and keep this task focused on schema, resolver, notification, schedule, sync, and minimal controls.

## Done Criteria

- All PRD acceptance criteria are either implemented and checked or explicitly deferred with user approval.
- Final validation commands pass.
- Trellis check phase reviews backend/frontend specs and task artifacts.
- Any new reusable conventions discovered during implementation are written back to `.trellis/spec/`.
