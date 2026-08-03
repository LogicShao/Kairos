# LZU 学期上下文持久化与当前周校准

## Goal

把 LZU `getXlxx` 返回的学期上下文从一次性导入辅助数据，升级为 Kairos 本地可复用的学期状态，用于更稳定地计算当前周、课程日历周、Today 今日课程和后续 LZU 刷新能力。

这是 FasterLZU “缓存学期信息 + 当前周加载课表”模式在 Kairos 的合理落点。任务优先级为 P1，因为它影响课程、日历和 Today 的一致性。

## Branch Strategy

默认落在现有 LZU feature 分支：

```text
feat/lzu-api-integration
```

不要在本任务中提交、打 tag 或 push，除非用户后续明确要求。

## Dependencies

- 依赖已完成的 `lzu-schedule-import`：已有 `getXlxx` 与课程导入映射。
- 关联 `today-briefing-card`：Today 的今日课程/下一节课应复用同一套当前周判断。
- 参考 FasterLZU：
  - `_TEMP/fasterlzu/lib/core/schedule/providers/schedule_provider.dart`
  - `_TEMP/fasterlzu/lib/core/schedule/repositories/schedule_repository.dart`
  - `_TEMP/fasterlzu/lib/core/schedule/models/schedule_model.dart`

## Requirements

### R1: 学期上下文模型

定义 Kairos 本地学期上下文，至少包含：

- 来源：`lzu`
- 学年 `xn`
- 学期 `xq` / `xqm`
- 学期开始日期 `ksrq`
- 当前周 `dqrqszzc`
- 总周次 `zzx`（允许缺失）
- 最后刷新时间

### R2: 持久化策略

学期上下文应写入本地 SQLite 或已有配置存储，供后端 schedule / briefing 复用。不得写入 token、账号密码或完整 LZU 原始响应。

### R3: 当前周校准

当用户通过 LZU 导入课表后，Kairos 应使用学期上下文校准：

- 课程的 `semester_start_date`
- 周视图默认周次
- 日历周定位
- Today 今日课程/下一节课判断

如果 `zzx` 缺失，保留现有 fallback 策略，但应记录为“总周次未知”，而不是制造一个持久化的假值。

### R4: 手动覆盖

用户仍可通过现有“统一学期开始日期”能力修正本地课程。手动覆盖不应被后台静默覆盖；再次 LZU 导入时可以提示将使用新的 LZU 学期上下文。

## Acceptance Criteria

- [x] 后端存在可复用的学期上下文模型/存取层。
- [x] LZU 导入成功后会保存低敏学期上下文。
- [x] `zzx` 缺失时不会报错，也不会持久化虚假总周次。
- [x] 周视图、日历和 Today 使用一致的学期开始日期/当前周判断。
- [x] 现有手动课程与剪贴板导入不回退。
- [x] 有单元测试覆盖当前周计算、`zzx` 缺失和手动覆盖边界。
- [x] `npm run lint`、`npx tsc --noEmit`、`cargo test --manifest-path src-tauri/Cargo.toml` 通过。

## Out of Scope

- 不实现 LZU 后台自动刷新课表。
- 不实现多学校/多账号学期管理。
- 不实现远端自定义课程增删改。
- 不实现服务入口或一卡通能力。

## Notes

- 这项任务比 UI 更偏后端一致性。不要把当前周计算散落到多个前端组件。
- 如果需要新增数据库表，应包含迁移与同步策略说明；低敏学期上下文可以同步，但必须确认不会包含账号标识或 token。
