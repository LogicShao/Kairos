# LZU 课表拉取与本地课程导入 - 设计

## Backend Shape

建议扩展 LZU 模块：

```text
src-tauri/src/lzu/
  schedule.rs
  mapper.rs
```

`schedule.rs` 负责 API 调用和响应模型，`mapper.rs` 负责 LZU 字段到 Kairos 课程字段的转换。数据库写入继续使用 `db::courses` 或从 `commands::courses` 抽出的共享导入函数。

## Import Flow

```text
authenticated session
  -> getXlxx
  -> getZdyCourse
  -> map CourseInfo[] to CreateCourseRequest[]
  -> import with duplicate detection
  -> return ImportTextResult-like feedback
```

## Mapping Rules

### Semester

优先从 LZU 学期信息构造稳定学期标识。建议先保持 Kairos 现有格式，例如 `2026S1`，具体映射规则由研究文档确认。

### Week Pattern

若 LZU `week` 是逗号分隔周次，转换为 Kairos 可解释的规则文本。第一版可以保守输出连续区间和单双周：

- `1,2,3,4` -> `1-4周全周`
- `1,3,5` -> `1-5周单周`
- `2,4,6` -> `2-6周双周`
- 非规则列表可保留为多个片段，前提是 `matches_week_pattern` 能消费；否则需扩展测试。

### Class Time

使用 LZU 节次时间表：

- 第 1 节 `08:30-09:15`
- 第 2 节 `09:25-10:10`
- 第 3 节 `10:30-11:15`
- 第 4 节 `11:25-12:10`
- 第 5 节 `14:30-15:15`
- 第 6 节 `15:25-16:10`
- 第 7 节 `16:30-17:15`
- 第 8 节 `17:25-18:10`
- 第 9 节 `19:00-19:45`
- 第 10 节 `19:55-20:40`
- 第 11 节 `21:00-21:45`
- 第 12 节 `21:55-22:40`

FasterLZU 还包含中午节次，映射前需要确认 Kairos UI 是否接受这些时间段；若接受，则完整保留。

## Duplicate Detection

建议复用现有导入 key：

```text
semester + name + day_of_week + start_time + end_time + week_pattern + semester_start_date + location + teacher
```

如需跨来源稳定去重，后续再扩展 source metadata；MVP 不新增数据库字段。

## Error Handling

部分课程字段缺失时，不应导致全量导入失败。无法映射的课程计入 failed，并返回摘要。认证失效、API 整体失败、解密失败则返回整体错误。
