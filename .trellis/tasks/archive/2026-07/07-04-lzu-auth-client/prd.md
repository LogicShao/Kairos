# LZU 认证与 AppService Client

## Goal

在 Kairos 的 Tauri Rust 后端实现 LZU AppService API 的基础 client 和认证能力，为后续课表导入提供可靠、可测试、可脱敏的后端边界。

## Branch Strategy

实现代码归属 feature 分支：

```text
feat/lzu-api-integration
```

不要在本任务中提交或推送，除非用户后续明确要求。

## Dependencies

- 依赖 `lzu-api-research` 的协议文档。
- 不依赖 `lzu-schedule-import`、`lzu-import-ui` 或 `lzu-easytong-services`。

## Requirements

认证链路只服务于用户本人授权账号的正常登录和后续业务请求；实现不得依赖漏洞测试、认证绕过或真实 token 抽取。

### R1: 新增后端 LZU AppService Client

在 Rust 后端新增清晰的 LZU 模块边界，用于封装：

- base URL
- headers
- request timeout
- response 解密/JSON 解析
- 错误类型映射

### R2: 实现 AES 加解密

实现兼容 FasterLZU 行为的 AES-CBC hex 加解密：

- key 和 IV 来源一致。
- 明文 JSON 加密为 hex 字符串。
- 加密响应可解密并 trim zero padding。
- 明文 JSON 响应也能兼容解析。

### R3: 实现登录状态

提供 Tauri command 或后端 service 能力：

- `lzu_login`
- `lzu_logout`
- `lzu_get_auth_status`

登录成功后后端保存必要 token 状态，不把完整 token 交给 React 持久化。

### R4: 安全和脱敏

- 不持久化明文密码。
- 不在日志中输出密码、token、完整 Authorization、完整加密 payload。
- 失败错误返回用户可理解的信息，同时保留后端可调试分类。

## Acceptance Criteria

- [x] Rust 后端存在独立 LZU AppService client 模块。
- [x] AES 加解密有单元测试覆盖。
- [x] 登录 command 返回登录状态摘要，不泄露完整 token。
- [x] 登录失败、网络失败、解密失败、响应结构异常都有稳定错误。
- [x] 前端无需知道 endpoint、AES、headers、token 细节。
- [x] `cargo test --manifest-path src-tauri/Cargo.toml` 通过。

## Verification

- `cargo test --manifest-path src-tauri/Cargo.toml` 通过，137 passed。
- `npm run lint` 通过。
- `npx tsc --noEmit` 通过。
- LZU 范围 Rust jscpd 通过，0 clone。
- 用户在本地 App 使用合法配置验证登录接口返回 `200 OK`，后端记录 `LZU 登录成功`。
- 日志未包含密码、token、完整 Authorization、加密 payload 或密钥原文。

## Out of Scope

- 不实现课表导入。
- 不实现一卡通/EasyTong。
- 不实现 UI 登录表单。
- 不设计长期账号密码存储。
- 不实现认证绕过、token 提取或真实凭据采集能力。
