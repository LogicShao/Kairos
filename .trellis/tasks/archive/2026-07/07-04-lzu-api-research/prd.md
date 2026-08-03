# LZU API 协议与竞品实现调研

## Goal

基于 `_TEMP/fasterlzu` 的现有实现，调研并固化 LZU API 接入所需的协议事实、接口边界、字段模型、安全约束和实现风险，为后续 Rust 后端接入提供可复用依据。

本任务只产出研究和规划文档，不实现业务代码。

## Branch Strategy

归属分支元数据：`feat/lzu-api-integration`。

本任务不要求实际切换 git 分支，也不要求提交。研究产物写入本任务目录，供后续子任务引用。

## Requirements

本调研限定在用户授权使用的正常业务 API 兼容性分析；不做漏洞测试，不绕过认证，不提取真实 token，也不把真实账号凭据写入研究产物。

### R1: 记录 API 边界

调研并记录至少两套 API 边界：

- `https://appservice.lzu.edu.cn`
  - 登录、登出、用户信息、课表、学期信息、`st`。
  - AES-CBC hex 加解密。
  - `gateway_token` / `login_token` 关系。
- `http://app.lzu.edu.cn:8080`
  - EasyTong token 交换。
  - 一卡通余额、二维码、订单查询。
  - MD5 签名。

### R2: 形成 endpoint matrix

每个 endpoint 至少记录：

- base URL
- path
- method
- request headers
- request body/query
- 是否加密
- response 格式
- 依赖 token
- 对应 FasterLZU 文件位置

### R3: 记录数据模型

重点记录后续课表导入需要的字段：

- 登录响应 token 字段。
- 学期信息 `XlxxData` 字段。
- 课表 `CourseInfo` 字段。
- EasyTong 相关字段作为后续扩展记录。

### R4: 记录安全和实现风险

必须记录：

- 不持久化明文密码。
- token 不进入前端持久化状态。
- 日志脱敏要求。
- FasterLZU 中不建议照搬的实现点。
- 网络超时、接口变更、字段缺失的处理建议。

## Deliverables

- `research/lzu-api-contract.md`
- `research/fasterlzu-lessons.md`

## Acceptance Criteria

- [x] `research/lzu-api-contract.md` 存在，并包含 AppService 和 EasyTong endpoint matrix。
- [x] `research/fasterlzu-lessons.md` 存在，并包含可学习点与应规避点。
- [x] 明确 MVP 只依赖 AppService 课表链路，EasyTong 不阻塞 MVP。
- [x] 明确后续 Rust 模块边界和字段映射输入。
- [x] 研究文档包含 FasterLZU 关键源文件引用路径。

## Verification

- `research/lzu-api-contract.md` 已覆盖 AppService 登录、登出、`getSt`、`getXlxx`、`getZdyCourse` 与 EasyTong 相关 endpoint。
- `research/fasterlzu-lessons.md` 已记录可复用实现点、应规避点和安全边界。
- 后续 `lzu-auth-client`、`lzu-schedule-import`、`lzu-import-ui` 已基于本研究完成实现并通过验证。
- 研究文件未记录真实账号、密码、token、headers 原文或密钥。

## Out of Scope

- 不实现 Rust client。
- 不实现 UI。
- 不做真实账号登录测试，除非用户另行提供测试条件。
- 不提取、保存或复用真实 token。
- 不做漏洞测试或认证绕过验证。
- 不触发任何生产写操作。
