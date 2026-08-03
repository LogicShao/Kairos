# 课程导入 UI 增加 LZU API 来源 - 执行计划

## Checklist

- [x] 阅读 `.trellis/spec/frontend/index.md`。
- [x] 阅读现有 `ImportModal` 和课程页状态管理。
- [x] 定义 LZU auth/import 前端类型。
- [x] 封装 Tauri invoke 调用。
- [x] 将导入 modal 拆成可维护的来源面板，避免单组件过大。
- [x] 实现 LZU 登录面板。
- [x] 实现 LZU 导入按钮和结果反馈。
- [x] 导入成功后刷新课程列表。
- [x] 覆盖空状态和错误状态。

## Validation

```bash
npm run lint
npx tsc --noEmit
cargo test --manifest-path src-tauri/Cargo.toml
```

## Review Gates

- [x] 剪贴板导入没有回退。
- [x] 密码字段不会持久化。
- [x] 前端没有硬编码 token 或加密细节。
- [x] 组件拆分符合现有目录和 UI 风格。
- [x] 移动端布局不溢出。

## Verification Notes

- `npm run lint` 通过。
- `npx tsc --noEmit` 通过。
- `cargo test --manifest-path src-tauri/Cargo.toml` 通过，137 passed。
- `npx jscpd "src-tauri/src/lzu" "src-tauri/src/commands/lzu.rs" --threshold 10 --reporters console --format rust` 通过，0 clone。
- `python ./.trellis/scripts/task.py validate "07-04-lzu-import-ui"` 通过。
- 用户在本地 App 验证 LZU 登录、`getXlxx` 与多周 `get_schedule` 请求均返回 `200 OK`；缺失 `zzx` 时 fallback 拉取策略生效。

## Rollback

- UI 面板可单独移除，保留后端命令。
- 若 LZU API 不稳定，可隐藏 LZU tab，不影响剪贴板导入。
