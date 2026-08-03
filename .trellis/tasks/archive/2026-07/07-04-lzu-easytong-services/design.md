# LZU 一卡通与服务入口接入 - 设计

## Architecture

EasyTong 作为独立 client，不混入 AppService：

```text
lzu::auth -> get st
lzu::easytong -> exchange EtToken
lzu::card -> account / wallet
lzu::services -> service detail / directory
```

二维码、订单轮询和 WebView SSO 不进入 Phase A 架构面；后续需要单独设计 host allowlist、token 生命周期和用户确认。

## Token Flow

1. 使用 AppService auth session 获取 `st`。
2. 调用 `ExchangeEtToken`。
3. 保存运行期 `EtToken`、`AccNum`、必要 card account 信息。
4. 后续一卡通请求使用 `Authorization: EtToken`。

Token 只保存在运行期内存。除非后续任务单独设计安全存储，否则不落库、不进同步、不进日志。

## Signing

MD5 签名规则由研究文档固化。实现时不要依赖 HashMap 随机迭代顺序，必须显式按接口要求构造签名字段顺序。

## Response Parsing

EasyTong 返回 JSON 与 XML 混合格式。XML 解析必须容错：

- 缺字段返回 `None` 或业务错误。
- 根节点异常返回解析错误。
- 不因一个可选字段缺失 panic。

## Security

- Phase A 不生成 QR code。
- QR code 属于敏感短期凭据，后续即便实现也不做持久化。
- 日志不得输出 QR 内容、EtToken、AccNum 完整值。
- WebView/cookie 注入必须限制目标 host，并且不属于 Phase A。

## UI Strategy

一卡通 UI 不应塞进课程导入流程。Phase A 建议入口：

- Today 或设置页的小型“校园卡余额”只读块。
- 独立“LZU 服务”页展示服务目录。

支付二维码、订单状态、H5 SSO 入口默认隐藏，直到单独安全评审完成。
