# LZU API 接入

## Goal

为 Kairos 接入合法第三方 LZU API 能力，优先完成 LZU 课表的官方接口导入链路，让用户可以从 LZU 账号登录、拉取课表、归一化为 Kairos 本地课程，并继续使用现有周视图、日历聚合和同步能力。

本父任务负责整体范围、子任务拆分、跨子任务验收和最终集成复核；直接代码实现由子任务承担。

## Branch Strategy

所有 LZU API 接入相关代码先在独立 feature 分支开发：

```text
feat/lzu-api-integration
```

`main` 分支保持稳定，不直接承载尚未验证的 LZU 登录、课表导入、一卡通或服务入口能力。本任务树下的实现型子任务默认使用同一分支：

- `lzu-auth-client`
- `lzu-schedule-import`
- `lzu-import-ui`
- `lzu-profile-identity`
- `lzu-semester-context`

`lzu-api-research` 是调研任务，不要求切换分支写业务代码，但研究产物仍归属本任务树。`lzu-easytong-services` 是后续扩展任务，默认不阻塞课表 MVP；若复杂度升高，可单独拆为 `feat/lzu-easytong-services`。

## Scope

### MVP Scope

1. 调研并记录 FasterLZU 中 LZU API 的认证、加密、课表和一卡通协议边界。
2. 在 Tauri Rust 后端新增 LZU API 适配层，封装 HTTP、AES、认证状态和错误映射。
3. 从 LZU AppService API 拉取学期信息与课表数据。
4. 将 LZU 课表记录转换为 Kairos 本地课程模型，并写入 SQLite。
5. 在课程导入 UI 中增加 LZU API 来源，保留现有剪贴板导入。

### Deferred Scope

基于 FasterLZU 的后续可学习点按风险拆分：

1. `lzu-profile-identity`：用户资料与登录身份展示，P0，低风险，优先做。
2. `lzu-semester-context`：学期上下文持久化与当前周校准，P1，支撑课程/日历/Today 一致性。
3. `today-briefing-card`：今日课程/下一节课体验已经由现有 Today 任务承接，不在本父任务重复创建。
4. `lzu-easytong-services`：一卡通只读余额、服务目录、WebView SSO 等作为 P2+ 后续能力；支付二维码、订单轮询、远端课程写操作保持更高风险的 deferred。

## Constraints

1. **本地优先**：LZU API 只作为导入/刷新来源，Kairos 的课程展示、周视图、日历聚合仍消费本地 SQLite。
2. **后端封装敏感协议**：账号密码、token、AES/MD5 签名、headers 和 endpoint 细节不暴露给 React 组件。
3. **分支隔离**：实现代码先落在 `feat/lzu-api-integration`，不直接污染 `main`。
4. **可回退**：现有剪贴板导入和手动课程管理必须继续可用。
5. **安全优先**：不持久化明文密码；不把 token 同步到 WebDAV；日志不得输出账号、密码、token、完整加密 payload。
6. **合法边界**：仅使用用户本人授权账号接入正常业务 API；不做漏洞测试，不绕过认证或授权流程，不提取、收集或复用真实 token，不调用生产写操作类接口作为 MVP。

## Task Map

1. `07-04-lzu-api-research`：协议与竞品实现调研。
2. `07-04-lzu-auth-client`：LZU AppService Client、AES、登录和认证状态。
3. `07-04-lzu-schedule-import`：课表拉取、字段映射、本地导入和去重。
4. `07-04-lzu-import-ui`：课程导入 UI 增加 LZU API 来源。
5. `07-04-lzu-profile-identity`：LZU 用户资料与登录身份展示（P0）。
6. `07-04-lzu-semester-context`：LZU 学期上下文持久化与当前周校准（P1）。
7. `07-04-lzu-easytong-services`：一卡通只读能力与服务入口后续扩展（P2+）。

## Cross-Task Acceptance Criteria

- [x] LZU 课表可以通过 API 拉取并导入 Kairos 本地课程表。
- [x] 导入后的课程能出现在现有课程页、周视图和日历聚合中。
- [x] 现有剪贴板导入、手动新增课程、课程编辑删除不回退。
- [x] LZU API 协议细节集中在 Rust 后端 LZU 模块，不散落到前端组件。
- [x] 登录失败、网络失败、解密失败、数据为空、字段无法映射均有可理解错误。
- [x] 不持久化明文密码，不在日志中输出敏感数据。
- [x] 登录和导入流程只使用用户授权账号的正常业务接口，不依赖绕过认证或真实 token 抽取。
- [x] MVP 不依赖 EasyTong，一卡通任务未完成不阻塞课表导入发布。
- [x] `npm run lint`、`npx tsc --noEmit`、`cargo test --manifest-path src-tauri/Cargo.toml` 通过。

## MVP Completion Notes

- `lzu-auth-client`、`lzu-schedule-import`、`lzu-import-ui` 已完成并归档。
- 实现提交：`99c2ade feat(lzu): add API schedule import`。
- 用户在本地 App 使用合法配置验证 LZU 登录、`getXlxx` 和多周 `get_schedule` 请求均返回 `200 OK`。
- `getXlxx.data.zzx` 在真实接口中可能缺失；后端已按 fallback 24 周策略处理，并写入 backend schedule import spec。
- `lzu-easytong-services` 保持 deferred，不阻塞课表 MVP。
- FasterLZU 进一步可学习点已拆入后续任务：`lzu-profile-identity`、`lzu-semester-context`，以及收窄后的 `lzu-easytong-services`。

## Follow-Up Priority

1. **P0 `lzu-profile-identity`**：先让用户明确当前登录身份，降低误账号导入风险。
2. **P1 `lzu-semester-context`**：保存低敏学期上下文，让课程页、日历页、Today 对当前周的判断一致。
3. **P1 `today-briefing-card`**：复用现有 Today 任务承接“今日课程/下一节课”，不在 LZU 树重复实现。
4. **P2 `lzu-easytong-services`**：先做只读余额和服务目录；支付二维码、订单轮询、WebView SSO 需要单独安全确认。
5. **P3 remote schedule writes**：远端新增/删除 LZU 自定义课程属于生产写操作，除非单独设计确认机制，否则不进入近期计划。

## Out of Scope

- 不在 MVP 实现一卡通余额、二维码、订单查询。
- 不在 MVP 实现服务 WebView 统一入口。
- 不在近期任务中实现 LZU 远端自定义课程增删改。
- 不重写现有课程数据库结构，除非课表映射证明必须扩展字段。
- 不引入微服务、云端中转服务或本地大模型。
- 不做 git commit、push 或实际分支切换，除非用户后续明确要求。

## Notes

- FasterLZU 参考目录：`_TEMP/fasterlzu`。
- Kairos 当前课程导入入口：`src/components/courses/ImportModal.tsx`。
- Kairos 当前课程后端命令：`src-tauri/src/commands/courses.rs`。
- Kairos 当前课程模型：`src/types/course.ts` 与 `src-tauri/src/db/models.rs`。
