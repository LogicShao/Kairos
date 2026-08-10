# AI 设置 WebDAV 加密同步

## Goal

将 AI 配置（base_url / model / api_key）以信封加密（envelope encryption）方式同步到 WebDAV：payload 用数据加密密钥（DEK）以 AES-256-GCM 加密，DEK 再用 WebDAV 密码派生密钥（PBKDF2-HMAC-SHA256）包裹存文件头。

改密码不丢数据（DEK 跨设备恒定），支持恢复密钥（DEK hex）用于重装设备场景。

## Requirements

- **加密**：AI 设置页新增「同步到 WebDAV」开关，开启后随主快照同步。
- **信封结构**：DEK → AES-256-GCM(payload)，KEK → PBKDF2(password, salt) → AES-256-GCM(DEK)。AAD 域隔离：`"kairos-ai-settings"`。
- **本地 DEK 副本**：`load_or_create_dek` 首次同步时生成并落盘（Unix 0600）。改密码场景用本地 DEK 解密，需上传前自动重裹 DEK。
- **恢复密钥**：DEK hex 表示，可通过 `get_ai_sync_recovery_key` 读取、`set_ai_sync_recovery_key` 写入。
- **LWW 合并**：整包胜负（不做字段级合并），`updated_at` 字符串比较。平局保本地以保证跨设备收敛。
- **非致命降级**：AI 设置同步失败仅打 warn log，不阻塞主快照同步。
- **数据库**：v15 迁移幂等加列（`ai_config.sync_enabled`, `sync_config.ai_settings_remote_etag`）。
- **远端文件**：`kairos-ai-settings.enc`，条件上传（If-Match ETag），HTTP 412 时自动重试。
- **前端**：AiSettings.tsx 新增同步开关 + 恢复密钥展示/复制/粘贴区域；同步开关禁用时需提示"请先在同步设置中配置 WebDAV"。

## Acceptance Criteria

- [ ] `cargo test` 全部通过（含 `ai_settings.rs` 9 个测试 + `db::ai`、`db::sync`、`db::migrations` 测试）
- [ ] `npm run build` 通过
- [ ] 加密包中不出现 API 密钥明文、base_url、model 名（已验证：已有 `test_blob_is_not_plaintext`）
- [ ] 错误的 WebDAV 密码 + 无恢复密钥 → 返回可理解的错误提示
- [ ] 错误的 WebDAV 密码 + 有本地 DEK → 自动回退并标记 `needs_rewrap`（下次上传重裹）
- [ ] 重装设备场景：粘贴恢复密钥后同步正常可用
- [ ] `rg -i "kairos-ai-settings.enc\|ai_settings_remote_etag" src-tauri/src/` 路径清晰无残留引用
- [ ] 同步开关未开启时 `execute_sync` 不访问 WebDAV（检查 `sync_enabled` 短路）

## Constraints

- 如果该功能本就不该添加或者是无用的重复代码，应当删除整个模块而非标注 deprecated。
- 确保人工智能主题色与图标符合整体设计语言。

## Out of Scope

- 不修改 AI 晨报生成逻辑。
- 不涉及桌面小组件。

## Notes

- 代码已在 dirty 工作区中实现完毕，本任务负责质量检查与提交。
- 功能本身与 LZU 自动导入共享 dirty，prd.md 反映已实现功能以确保 check 正确。