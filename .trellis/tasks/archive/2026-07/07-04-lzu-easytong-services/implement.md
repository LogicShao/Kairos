# LZU 一卡通与服务入口接入 - 执行计划

## Checklist

- [x] 复核 `lzu-api-research` 的 EasyTong endpoint matrix。
- [x] 实现 MD5 签名函数和单元测试。
- [x] 实现 `ExchangeEtToken`。
- [x] 实现账户信息查询。
- [x] 实现钱包余额查询。
- [x] 实现服务目录读取/搜索所需的低敏模型。
- [x] 新增只读余额或服务目录 UI，不能塞进课程导入流程。
- [x] 记录二维码接口安全策略，但不实现付款二维码。
- [x] 记录服务入口 WebView/cookie 策略，但不实现 SSO 注入。

## Validation

```bash
cargo test --manifest-path src-tauri/Cargo.toml
npm run lint
npx tsc --noEmit
```

实际验证：

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
npm run lint
npx tsc --noEmit
npm run build
npx jscpd . --threshold 10 --reporters console --format rust,typescript
```

结果：全部通过；`cargo test` 170 passed；jscpd 总重复率 8.41%，低于 10% 阈值。

## Review Gates

- [x] 不影响 LZU 课表导入。
- [x] Phase A 没有 QR code / 订单轮询实现。
- [x] EtToken / AccNum 不持久化、不同步、不进日志。
- [x] XML 解析无 panic 路径。
- [x] 签名字段顺序固定、可测试。
- [x] 服务目录失败时可降级，不影响课程/Today。

## Rollback

- 余额查询和服务目录可独立发布；二维码和服务入口 SSO 必须保持关闭。
- 如 WebView 风险不可控，只保留数据查询，不开放服务入口。
