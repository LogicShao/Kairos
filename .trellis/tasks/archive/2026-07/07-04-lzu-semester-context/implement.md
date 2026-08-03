# LZU 学期上下文持久化与当前周校准 - 执行计划

## Checklist

- [x] 盘点现有课程 `semester_start_date`、`schedule.rs`、`commands::briefing` 的周次计算入口。
- [x] 设计并实现 `SemesterContext` 存储层，或明确复用现有配置表的方案。
- [x] 为 LZU `XlxxData` 增加 mapper：只抽取低敏学期字段。
- [x] 在 LZU 导入成功后保存/刷新学期上下文。
- [x] 调整周视图/日历/Today 后端命令，使其优先复用统一上下文。
- [x] 保留现有手动“统一学期开始日期”能力。
- [x] 增加单元测试覆盖当前周推导、`zzx=None`、课程日期覆盖。

## Validation

```powershell
cargo test --manifest-path "src-tauri/Cargo.toml"
cargo clippy --manifest-path "src-tauri/Cargo.toml" --all-targets -- -D warnings
npm run lint
npx tsc --noEmit
npx jscpd . --threshold 10 --reporters console --format rust,typescript
```

如果新增迁移或同步字段，再补：

```powershell
npm run build
```

已验证：以上命令均通过；`jscpd` 总重复率 8.69%，低于 10% 阈值。

## Review Gates

- [x] 当前周逻辑不在前端重复实现。
- [x] `zzx` 缺失不会污染持久化数据。
- [x] 不保存 LZU 原始响应、token 或账号密码。
- [x] 不破坏剪贴板导入和手动课程维护。

## Rollback

如果新增持久化表风险过高，先把学期上下文作为后端运行期/配置缓存，只用于 LZU 导入后的本次会话校准；后续再迁移到 SQLite。
