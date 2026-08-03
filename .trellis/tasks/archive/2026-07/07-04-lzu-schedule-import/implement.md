# LZU 课表拉取与本地课程导入 - 执行计划

## Checklist

- [x] 阅读 `.trellis/spec/backend/index.md`。
- [x] 阅读 `.trellis/spec/backend/schedule-import-guidelines.md`。
- [x] 阅读 `src-tauri/src/commands/courses.rs` 的导入去重实现。
- [x] 根据研究文档定义 `Xlxx` 和 `CourseInfo` serde model。
- [x] 实现学期信息拉取。
- [x] 实现课表拉取。
- [x] 实现 `CourseInfo` -> `CreateCourseRequest` mapper。
- [x] 抽取或复用现有导入去重函数。
- [x] 新增 Tauri command，例如 `import_lzu_courses`。
- [x] 添加 mapper 单元测试：节次、周次、缺字段、重复课程。
- [x] 添加后端导入逻辑测试。

## Validation

```bash
cargo test --manifest-path src-tauri/Cargo.toml
npm run lint
npx tsc --noEmit
```

## Review Gates

- [x] 没有复制一套与现有导入逻辑冲突的去重实现。
- [x] 课程写入仍走本地 SQLite，UI 不直接消费远程课表。
- [x] 字段缺失处理是局部失败，不轻易中断整批导入。
- [x] 不调用 LZU 写入/修改课表接口。

## Verification Notes

- `cargo test --manifest-path src-tauri/Cargo.toml` 通过，137 passed。
- `npm run lint` 通过。
- `npx tsc --noEmit` 通过。
- LZU 范围 Rust jscpd 通过，0 clone。
- 用户在本地 App 验证 `getXlxx` 和多周 `get_schedule` 请求返回 `200 OK`。
- `zzx` 缺失已按 fallback 24 周策略处理，并写入 backend schedule import spec。

## Rollback

- 如果 mapping 不稳定，先只返回预览结果，不写数据库。
- 如果 API 只能按周返回，先限制为“当前周预览”，不默认全量导入，直到遍历周次策略验证完成。
