# LZU 课表拉取与本地课程导入

## Goal

基于已完成的 LZU AppService Client，拉取 LZU 学期信息和课表数据，并将其归一化为 Kairos 本地课程模型，写入 SQLite 后复用现有课程页、周视图、日历聚合和同步能力。

## Branch Strategy

实现代码归属 feature 分支：

```text
feat/lzu-api-integration
```

不要在本任务中提交或推送，除非用户后续明确要求。

## Dependencies

- 依赖 `lzu-api-research` 的协议文档。
- 依赖 `lzu-auth-client` 提供登录状态和 AppService client。
- 不依赖 `lzu-import-ui`；本任务应可通过后端 command 或测试独立验证。

## Requirements

### R1: 拉取学期信息

调用 LZU `getXlxx` 接口获取学期上下文，至少用于：

- 当前教学周。
- 学期总周数。
- 学期开始日期。
- 学年与学期标识。

### R2: 拉取课表数据

调用 LZU `getZdyCourse` 接口获取课表数据。

MVP 可以先以当前学期/指定周为入口，但最终导入应避免只导入单周可见课程。若 API 只能按周返回，则需要在实现计划中明确遍历周次策略和去重规则。

### R3: 字段映射到 Kairos 课程模型

将 LZU `CourseInfo` 映射到 Kairos `CreateCourseRequest`：

- `kcmc` -> `name`
- `skxql` -> `day_of_week`
- `skjsl` -> `location`
- `jsxm` -> `teacher`
- `week` / `week_fb` -> `week_pattern`
- `jc` -> `start_time` / `end_time`
- `xn` + `xqm` -> `semester`
- `xlxx.ksrq` 或可靠替代字段 -> `semester_start_date`

### R4: 复用本地导入和去重

导入应复用或抽取现有课程导入去重逻辑，避免为 LZU API 再写一套重复去重算法。

### R5: 返回导入反馈

后端命令应返回结构化导入结果：

- 拉取课程数。
- 成功导入数。
- 跳过重复数。
- 无法映射数。
- 用户可读 message。

## Acceptance Criteria

- [x] 后端可以在已登录状态下拉取 LZU 学期信息。
- [x] 后端可以拉取 LZU 课表数据。
- [x] LZU 课程能转换为 Kairos 本地课程并写入 SQLite。
- [x] 重复课程不会重复写入。
- [x] `jc` 节次 bitmask 能正确映射课程开始/结束时间。
- [x] 周次规则能被现有 `matches_week_pattern` 消费。
- [x] 导入结果返回 parsed/imported/skipped/failed 等统计。
- [x] `cargo test --manifest-path src-tauri/Cargo.toml` 通过。

## Verification

- `cargo test --manifest-path src-tauri/Cargo.toml` 通过，137 passed。
- LZU mapper 单元测试覆盖节次 bitmask、周次格式和 `week_fb` 交集。
- LZU 导入命令复用现有课程导入去重 helper。
- 用户在本地 App 验证 `getXlxx` 与多周 `get_schedule` 请求均返回 `200 OK`。
- 真实接口缺少 `zzx` 时，后端 fallback 到 24 周拉取策略并继续导入流程。

## Out of Scope

- 不实现 UI。
- 不实现一卡通/EasyTong。
- 不向 LZU 写入自定义课程。
- 不自动删除用户本地已有课程。
