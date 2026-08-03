# LZU API 接入 - 执行计划

## Order

1. 完成 `lzu-api-research`，记录协议事实和风险。
2. 完成 `lzu-auth-client`，建立后端 AppService client 和登录状态。
3. 完成 `lzu-schedule-import`，跑通课表拉取到 SQLite。
4. 完成 `lzu-import-ui`，把导入链路暴露给用户。
5. 最后回到父任务做集成回归。
6. `lzu-profile-identity` 作为 P0 后续任务，补齐登录身份确认。
7. `lzu-semester-context` 作为 P1 后续任务，统一学期上下文和当前周判断。
8. `today-briefing-card` 已承接“今日课程/下一节课”，不在 LZU 父任务重复实现。
9. `lzu-easytong-services` 作为 P2+ 后续扩展，默认不阻塞 MVP。

## Parent Review Checklist

- [ ] 每个 MVP 子任务都有通过状态或明确剩余风险。
- [ ] 分支元数据均指向 `feat/lzu-api-integration`。
- [ ] 课表链路不依赖 EasyTong。
- [ ] 前端没有硬编码 LZU endpoint、AES key、token。
- [ ] 日志检查确认没有敏感数据输出。
- [ ] 现有课程导入和课程管理仍工作。
- [ ] 后续任务排序明确：身份展示 -> 学期上下文 -> Today 复用 -> 只读一卡通/服务入口。
- [ ] 支付二维码、订单轮询、远端课程写操作均未被误纳入近期 P0/P1。

## Final Validation

```bash
npm run lint
npx tsc --noEmit
cargo test --manifest-path src-tauri/Cargo.toml
```

## Rollback Points

- Auth client 完成后再接课表，避免 UI 先行绑定不稳定接口。
- Schedule import 完成后先用后端测试验证映射，再接 UI。
- UI 只新增入口，不替换剪贴板导入。
