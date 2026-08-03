# AI 设置 WebDAV 加密同步 + LZU 登录自动导入课表

## Goal

父任务：承载两个独立可验证的后端增强子任务，共享同一段 dirty 工作区改动。

- 子任务 1：**AI 设置 WebDAV 加密同步** — 以信封加密（envelope encryption）将 AI 设置与 API 密钥同步到 WebDAV。
- 子任务 2：**LZU 登录后自动导入课表** — 登录统一认证成功后静默拉取课表并导入，无需手动点击。

## Requirements

- 两个子任务均可独立规划、校验与归档。
- 共享文件（`src-tauri/src/db/models.rs`、`src-tauri/src/db/migrations.rs`、`src-tauri/src/lib.rs`、`src-tauri/Cargo.lock`）由子任务 1 主责，子任务 2 只改动 `commands/lzu.rs`、`LzuImportPanel.tsx`。
- 子任务 2 依赖子任务 1 完成数据库迁移注册（已在一份 dirty 中，check 时需分别验证）。

## Acceptance Criteria

- [ ] 子任务 1 验收通过（见 `08-03-ai-settings-webdav-sync/prd.md`）
- [ ] 子任务 2 验收通过（见 `08-03-lzu-auto-import/prd.md`）
- [ ] 全量 `cargo test` 与 `npm run build` 通过
- [ ] 两个子任务提交后 dirty 工作区无残留

## Out of Scope

- 不涉及桌面小组件移除（属 `08-02-remove-desktop-widget`）。
- 不修改 AI 晨报生成逻辑。