# LZU API 协议与竞品实现调研 - 执行计划

## Checklist

- [x] 阅读 `_TEMP/fasterlzu/lib/app_config.dart`。
- [x] 阅读 `_TEMP/fasterlzu/lib/core/api/appservice_client.dart`。
- [x] 阅读 `_TEMP/fasterlzu/lib/core/api/app_client.dart`。
- [x] 阅读 `_TEMP/fasterlzu/lib/core/encryption/aes_crypto.dart`。
- [x] 阅读 `_TEMP/fasterlzu/lib/core/encryption/md5_crypto.dart`。
- [x] 阅读 auth、schedule、easytong repository。
- [x] 输出 AppService endpoint matrix。
- [x] 输出 EasyTong endpoint matrix。
- [x] 输出 FasterLZU lessons。
- [x] 标注 MVP / deferred 范围。

## Validation

- [x] 文档能直接支撑 `lzu-auth-client` 和 `lzu-schedule-import` 的实现。
- [x] 每个结论都有本地源码路径依据。
- [x] 没有把敏感 token、账号密码或真实个人数据写入研究文件。

## Completion Notes

- 研究产物已被 LZU auth/client、课表导入和导入 UI 三个后续任务消费。
- EasyTong/一卡通能力保持 deferred，不阻塞课表 MVP。

## Rollback

研究任务无代码回退点。若后续发现协议事实错误，回到本任务修订研究文档，再更新依赖子任务的 PRD/design。
