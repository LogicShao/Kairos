# LZU 用户资料与登录身份展示

## Goal

在现有 LZU 登录与课表导入链路上补齐“当前登录身份”反馈，让用户能明确知道正在使用哪个 LZU 账号导入课程，并为后续服务入口/一卡通能力提供最小用户资料上下文。

这是 FasterLZU 中 `userInfo` / `userImg` 能力在 Kairos 的低风险落点。任务优先级为 P0，因为它能降低误账号导入风险，且不涉及支付、WebView 或远端写操作。

## Branch Strategy

默认落在现有 LZU feature 分支：

```text
feat/lzu-api-integration
```

不要在本任务中提交、打 tag 或 push，除非用户后续明确要求。

## Dependencies

- 依赖已完成的 `lzu-auth-client`：提供登录状态、`login_token` / `gateway_token`。
- 依赖已完成的 `lzu-import-ui`：课程导入 UI 中已有 LZU 登录面板。
- 参考 FasterLZU：
  - `_TEMP/fasterlzu/lib/core/auth/repositories/auth_repository.dart`
  - `_TEMP/fasterlzu/lib/core/auth/models/auth_model.dart`

## Requirements

### R1: 用户资料拉取

后端在已登录状态下支持调用 AppService 用户资料接口：

- `/api/eusp-unify-terminal/app-user/userInfo`
- 可选：`/api/eusp-unify-terminal/app-user/userImg`

首轮只需要用户身份摘要，不要求头像。

### R2: 身份摘要模型

只向前端返回低敏字段，用于身份确认：

- 姓名 `xm`
- 学号/人员编号 `rybh` 或用户名
- 单位/学院 `dwmc`
- 人员类别 `rylb`
- 校园卡号脱敏展示，例如只显示后 4 位

不得向前端返回身份证号、手机号、邮箱、完整校园卡号、完整原始响应。

### R3: UI 展示

在 LZU 导入面板登录成功状态中展示身份摘要：

- 当前登录用户
- 学院/单位
- 登录状态刷新入口
- 资料获取失败时不阻塞课表导入，只显示“身份信息暂不可用”

### R4: 安全与日志

- 不持久化完整个人资料，除非后续任务明确设计本地资料缓存。
- 不在日志输出完整用户资料。
- 不把头像 base64 或敏感字段写入 WebDAV 同步数据。

## Acceptance Criteria

- [x] 后端存在独立命令或 auth status 扩展，可返回 LZU 身份摘要。
- [x] 前端 LZU 导入面板能显示当前登录身份。
- [x] 身份资料获取失败不会影响登录/导入课表。
- [x] 前端和日志均不暴露身份证号、手机号、完整校园卡号、token 或原始响应。
- [x] 有测试覆盖用户资料响应到摘要模型的字段筛选/脱敏逻辑。
- [x] `npm run lint`、`npx tsc --noEmit`、`cargo test --manifest-path src-tauri/Cargo.toml` 通过。

## Out of Scope

- 不实现头像展示。
- 不实现个人资料编辑。
- 不实现一卡通余额、二维码或订单查询。
- 不实现服务目录或 WebView SSO。
- 不持久化完整用户资料。

## Notes

- 这是 LZU 后续任务中风险最低、用户感知最直接的一步。
- 如果后续实现用户资料缓存，必须先设计数据最小化和同步排除策略。
