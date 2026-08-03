# LZU 一卡通与服务入口接入

## Goal

在 LZU 课表 MVP 稳定后，基于 LZU App / EasyTong API 扩展低风险校园服务能力。首轮只考虑只读余额和服务目录；支付二维码、订单轮询、WebView SSO 等能力必须单独安全确认后再进入实现。

该任务是 P2+ 后续扩展，不阻塞 LZU 课表导入 MVP，也不阻塞 `lzu-profile-identity` 和 `lzu-semester-context`。

## Branch Strategy

默认归属分支元数据：

```text
feat/lzu-api-integration
```

如果实现范围扩大或风险较高，可在开始前拆为独立分支：

```text
feat/lzu-easytong-services
```

不要在本任务中提交或推送，除非用户后续明确要求。

## Dependencies

- 依赖 `lzu-api-research` 中的 EasyTong 协议调研。
- 依赖 `lzu-auth-client` 提供 `st` 获取能力。
- 不阻塞 `lzu-schedule-import`、`lzu-import-ui`、`lzu-profile-identity` 或 `lzu-semester-context`。

## Requirements

本任务仅面向用户授权账号的正常一卡通/服务入口能力；不做漏洞测试，不绕过认证，不提取或复用真实 token。

### Risk Split

1. **Phase A: 只读基础能力**
   - ST -> EtToken 交换。
   - 账户信息查询。
   - 钱包余额查询。
   - 服务目录读取和搜索。
2. **Phase B: 受控打开官方服务**
   - 使用 ST 构造官方服务入口。
   - 优先打开外部浏览器或受限 WebView。
   - 明确 host allowlist、cookie/token 生命周期和日志策略。
3. **Phase C: 支付相关能力**
   - 付款二维码。
   - 订单轮询。
   - 该阶段默认不实现，除非单独任务通过安全设计评审。

### R1: EtToken 交换

通过 AppService `st` 换取 EasyTong `EtToken` 和账号信息。

### R2: EasyTong Client

封装 `http://app.lzu.edu.cn:8080` client：

- base URL
- User-Agent
- form-urlencoded body
- Authorization header
- XML / JSON 混合响应解析
- MD5 签名

### R3: 一卡通只读能力

首轮只实现：

1. 查询账户信息。
2. 查询钱包余额。

付款二维码和订单状态查询不进入首轮验收。

### R4: 服务入口

调研并实现可控的服务目录能力：

- 获取服务详情。
- 展示服务分类、名称、图标、介绍、是否需要登录等低敏信息。
- 支持搜索/筛选。
- 打开服务页面必须单独确认 WebView/浏览器安全边界；默认不在 Phase A 实现 SSO 注入。

## Acceptance Criteria

- [x] EasyTong client 与 AppService client 分离。
- [x] MD5 签名有单元测试。
- [x] XML 响应解析对缺字段有容错。
- [x] 余额查询可独立验证。
- [x] 首轮不实现支付二维码或订单轮询。
- [x] 服务目录可以在不影响课表导入的情况下独立失败/降级。
- [x] 服务入口不会影响课表导入链路。

## Out of Scope

- 不在课表 MVP 前实现。
- 不做充值、支付确认或任何资金写操作。
- 不把二维码保存到本地文件。
- 不在 Phase A 实现付款二维码或订单轮询。
- 不在 Phase A 注入 cookie/localStorage 打开 H5 服务。
- 不绕过用户授权打开服务。
- 不实现 token/cookie 抽取、认证绕过或未授权服务访问。
