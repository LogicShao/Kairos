# LZU 登录后自动导入课表

## Goal

在用户完成 LZU 统一认证登录后，后台静默拉取课表并自动导入，无需用户手动点击「导入课表」按钮。登录流程本身不受影响（前台即时返回成功，后台落库成功后通过事件推送结果）。

## Requirements

- **提取核心逻辑**：将 `import_lzu_courses` 命令中的导入逻辑抽取为纯入参函数 `import_lzu_courses_inner(db, client, session)`，同时服务手动导入与自动导入。
- **登录后自动触发**：`lzu_login` 成功后在 `tauri::async_runtime::spawn` 中静默调用 `auto_import_lzu_after_login`。
- **前端事件通知**：自动导入完成后 emit `lzu-auto-import` 事件，前端 `LzuImportPanel` 监听并展示结果，刷新课表页面。
- **不影响登录流程**：后台导入失败不影响登录成功状态的返回；登录耗时不因导入而增加。
- **幂等与静默**：登录后自动导入是全量拉取，与手动点击无异；失败只记错误日志，不出现在登录错误中。

## Acceptance Criteria

- [ ] `cargo test` 全部通过
- [ ] `npm run build` 通过
- [ ] 现有 LZU 命令测试不因重构受损（重点验证 `lzu_login` 与 `import_lzu_courses` 签名不变）
- [ ] 手动 `import_lzu_courses` 仍然正常可用（重构未破坏现有入口）
- [ ] 自动导入的错误不被前端误当登录错误展示

## Out of Scope

- 不修改课表解析算法与学期上下文逻辑。
- 不涉及 AI 相关功能。