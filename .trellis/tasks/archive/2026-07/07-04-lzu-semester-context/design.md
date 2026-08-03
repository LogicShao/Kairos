# LZU 学期上下文持久化与当前周校准 - 设计

## Boundary

学期上下文属于后端数据契约。前端可以展示和触发刷新，但不得自行解析 LZU `xlxx` 字段或计算权威当前周。

```text
LZU getXlxx
  -> lzu::models::XlxxData
  -> semester context mapper
  -> local persistence
  -> schedule / calendar / briefing commands
```

## Data Model

建议模型：

```text
SemesterContext {
  id: i64,
  source: String,              // "lzu"
  academic_year: Option<String>,
  term: Option<String>,
  term_label: Option<String>,
  start_date: String,          // YYYY-MM-DD
  current_week: Option<i64>,
  total_weeks: Option<i64>,
  refreshed_at: String,
}
```

如果现有课程表字段足以支撑部分逻辑，也不要把上下文埋进任意一门课程。学期上下文应能被 `schedule.rs`、`commands::briefing` 和课程导入逻辑共同读取。

## Current Week Rule

优先级：

1. 用户显式传入的 week / week_start_date。
2. 本地 `SemesterContext.start_date` 推导出的自然当前周。
3. LZU `dqrqszzc` 作为导入时的参考值。
4. 现有 fallback。

不要让 `dqrqszzc` 永久压过本地日期推导，否则跨天/跨周后会过期。

## Missing `zzx`

真实接口可能没有 `zzx`。持久化时使用 `None`；拉取课表时可以继续使用运行期 fallback 上限，例如 24 周，但 UI/数据层不要声称“总周次=24”。

## Sync

如果新增表参与 WebDAV 同步，只同步低敏学期信息，不同步 LZU 账号、token、payload 或用户资料。若同步成本高，可以先标记为本地配置，不进入本任务首轮。

## Compatibility

现有课程 `semester_start_date` 仍保留。学期上下文用于提供默认值和校准，不应破坏用户手动编辑课程日期的能力。
