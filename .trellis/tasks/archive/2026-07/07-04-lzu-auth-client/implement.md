# LZU 认证与 AppService Client - 执行计划

## Checklist

- [x] 阅读 `.trellis/spec/backend/index.md` 和相关错误处理/日志规范。
- [x] 根据研究文档确认 AES 参数和登录 endpoint。
- [x] 新增 `src-tauri/src/lzu` 模块。
- [x] 实现 AES-CBC hex 加解密。
- [x] 实现 AppService HTTP client。
- [x] 实现登录请求/响应模型。
- [x] 实现后端登录状态。
- [x] 暴露最小 Tauri commands。
- [x] 添加单元测试。
- [x] 更新 `src-tauri/src/lib.rs` command 注册。

## Validation

```bash
cargo test --manifest-path src-tauri/Cargo.toml
npm run lint
npx tsc --noEmit
```

## Review Gates

- [x] 没有把 endpoint 和 token 逻辑放到 React。
- [x] 没有缓存明文密码。
- [x] 日志脱敏。
- [x] 每个请求的加密行为是局部配置，不修改全局 client 状态。

## Verification Notes

- `cargo test --manifest-path src-tauri/Cargo.toml` 通过，137 passed。
- `npm run lint` 通过。
- `npx tsc --noEmit` 通过。
- `npx jscpd "src-tauri/src/lzu" "src-tauri/src/commands/lzu.rs" --threshold 10 --reporters console --format rust` 通过，0 clone。
- 用户在本地 App 验证 LZU 登录成功；真实配置文件未被任务执行过程读取或提交。

## Rollback

- 若 AES 实现不稳定，保留模块边界但撤回 commands 暴露。
- 若登录 API 不稳定，不继续接 UI，先回到 research 修订协议文档。
