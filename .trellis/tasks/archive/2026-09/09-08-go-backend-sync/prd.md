# WebDAV 同步核心

## Goal

实现 `internal/domain/sync`：v2 快照协议（导出/导入：LWW 合并、sync_id 匹配、墓碑传播、ETag 条件上传）、WebDAV 客户端（Basic Auth/ETag/超时）、AI 设置信封加密同步（AES-256-GCM + PBKDF2 + DEK 恢复密钥）、手动同步端点。

**架构差异（app→web）**：取消 Rust 的常驻自动同步线程；改为**页面打开时触发 + 手动端点**（`POST /api/sync/now` 请求-响应，无需事件）。

## Requirements

- G1 `internal/domain/sync`：v2 快照 export_all/import_all（LWW 合并、sync_id 匹配、墓碑传播），对齐 Rust `sync/exporter.rs`（1206 行）
- G2 WebDAV client（Basic Auth、ETag 条件上传/304、超时），对齐 `sync/webdav.rs`
- G3 AI 设置加密同步（AES-256-GCM + PBKDF2 + DEK 恢复密钥），对齐 `sync/ai_settings.rs`
- G4 handler：`GET/PATCH /api/sync/config`、`POST /api/sync/test`、`POST /api/sync/now`、`GET/POST /api/sync/ai-recovery-key`
- G5 DTO 对齐 `src/types/sync.ts`

## Acceptance Criteria

- [ ] `go test ./internal/domain/sync/...` 绿（LWW 冲突合并、etag、AI 设置加解密 roundtrip，对齐 Rust 测试）
- [ ] `go test ./internal/...` 全绿
- [ ] curl 冒烟：config 读写、test 连接、now 同步（可用 httptest WebDAV 或跳过真实远端）
- [ ] dev PG 容器仅验证期间临时启动，验证后已关闭

## Notes

- 移植锚点：Rust `src-tauri/src/sync/{exporter.rs,webdav.rs,ai_settings.rs,mod.rs,ids.rs}`（2650 行）+ 其 `#[cfg(test)]`
- 加密：AES-256-GCM（`crypto/aes`+`crypto/cipher`）+ PBKDF2（`golang.org/x/crypto/pbkdf2`）
- **不移植**常驻自动同步线程；改页面触发/手动
- WebDAV 用 `net/http`（Basic Auth + ETag If-Match/If-None-Match）
