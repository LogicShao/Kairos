# LZU 用户资料与登录身份展示 - 执行计划

## Checklist

- [x] 读取当前 `src-tauri/src/lzu/` 与 `src-tauri/src/commands/lzu.rs` 的 auth status 结构。
- [x] 为 LZU AppService client 增加 `user_info` 请求方法。
- [x] 新增低敏 `ProfileSummary` DTO 和字段脱敏函数。
- [x] 增加命令或扩展 `lzu_get_auth_status`，向前端暴露身份摘要。
- [x] 更新 `src/types/lzu.ts`。
- [x] 更新 `LzuImportPanel` 的 logged-in 状态展示。
- [x] 为脱敏/字段筛选补单元测试。

## Validation

```powershell
cargo test --manifest-path "src-tauri/Cargo.toml" # passed, 139 tests
npm run lint # passed
npx tsc --noEmit # passed
```

如果修改 Rust LZU 模块，再补：

```powershell
npx jscpd "src-tauri/src/lzu" "src-tauri/src/commands/lzu.rs" --threshold 10 --reporters console --format rust # passed, 0 clones
```

## Review Gates

- [x] 前端没有完整个人资料模型。
- [x] 日志不输出用户资料、token、payload。
- [x] 资料接口失败不阻塞课表导入。
- [x] UI 不新增复杂个人页，只做登录身份确认。

## Rollback

如果资料接口不稳定，保留后端方法但前端降级为“已登录：用户名”，不影响 LZU 课表导入。
