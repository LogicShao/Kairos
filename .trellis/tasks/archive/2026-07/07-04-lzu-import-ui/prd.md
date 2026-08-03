# 课程导入 UI 增加 LZU API 来源

## Goal

在现有课程导入入口中增加 LZU API 导入来源，让用户可以通过 UI 登录 LZU、拉取课表并导入 Kairos 本地课程，同时保留剪贴板导入和手动课程管理能力。

## Branch Strategy

实现代码归属 feature 分支：

```text
feat/lzu-api-integration
```

不要在本任务中提交或推送，除非用户后续明确要求。

## Dependencies

- 依赖 `lzu-auth-client`。
- 依赖 `lzu-schedule-import`。
- 不依赖 `lzu-easytong-services`。

## Requirements

### R1: 入口整合

在课程导入体验中新增 LZU API 来源。可以采用 tabs、segmented control 或同级入口，但不能替换现有剪贴板导入。

### R2: 登录状态

UI 至少支持：

- 未登录状态：输入账号和密码。
- 登录中状态。
- 已登录状态：显示当前账号摘要。
- 登录失败状态：显示用户可理解错误。
- 退出登录或切换账号入口。

### R3: 导入流程

UI 至少支持：

- 拉取课表。
- 显示导入中状态。
- 显示导入结果统计。
- 导入成功后刷新课程列表。

### R4: 安全交互

- 密码输入框使用 password 类型。
- 不把密码写入 React 持久状态、localStorage 或 URL。
- 错误提示不显示 token、headers、加密 payload。

### R5: 失败与空状态

UI 需要覆盖：

- 未登录时点击导入。
- 网络失败。
- 登录过期。
- 课表为空。
- 部分课程无法映射。

## Acceptance Criteria

- [x] 现有剪贴板导入仍可使用。
- [x] UI 可以触发 LZU 登录。
- [x] UI 可以触发 LZU 课表导入。
- [x] 导入结果包含成功、跳过、失败统计。
- [x] 导入完成后课程页/周视图能看到新增课程。
- [x] 密码不会被持久化到浏览器存储。
- [x] 错误提示不泄露敏感信息。
- [x] `npm run lint` 和 `npx tsc --noEmit` 通过。

## App Verification

2026-07-04 用户在本地 App 使用合法配置完成 LZU 登录与课表拉取验证：

- LZU 登录接口返回 `200 OK`，后端记录登录成功。
- `getXlxx` 返回 `200 OK`。
- 当 `zzx` 缺失时，后端按 fallback 24 周策略继续拉取课表。
- 多周 `get_schedule` 请求返回 `200 OK`。
- 验证日志未包含 token、headers、加密 payload 或密钥原文。

## Out of Scope

- 不实现一卡通 UI。
- 不实现服务 WebView。
- 不做自动后台刷新课表。
- 不做账号长期记住密码。
