# FasterLZU 实现经验教训

> 基于 `_TEMP/fasterlzu` 源码分析，面向 Kairos Rust 后端的实现建议。
> 研究日期：2026-07-04

---

## 1. 值得学习的实现点

### 1.1 EncryptInterceptor 的 AOP 模式

`lib/core/api/appservice_client.dart:25-53` 中，通过 Dio 拦截器自动处理请求加密和响应解密，业务代码不需要关心加解密细节。

**学习点：**
- 拦截器/中间件模式适合处理"协议级别"的横切关注点。
- Kairos 的 Rust HTTP client 可以通过 `tower::Layer` 或 `reqwest_middleware` 实现类似效果。
- 但要注意：**不是所有请求都加密**（如 `Transfer-Encrypt` header 控制），中间件需要支持按请求开关。

### 1.2 Token 存储使用平台安全存储

`FlutterSecureStorage`（平台 Keychain/KeyStore 封装）存储 token，而非 SharedPreferences。

**学习点：**
- Kairos 后端应使用加密存储（如 `seaorm` + 加密列，或 `aes-gcm` 加密后再存 DB）。
- 前端不应持有原始 token（通过 Tauri command 调用后端，由后端附加 token）。

### 1.3 学期开始日期的利用

`XlxxData.ksrq` 让周次→具体日期的换算成为可能，这对于课表展示很关键。

**学习点：**
- 将 `ksrq` 作为 `semester_start_date` 映射到 Kairos 数据模型中。
- 缓存 `XlxxData` 避免频繁请求。

### 1.4 节次时间表的集中配置

`AppConfig.classTimes` / `classStartTimes` / `classEndTimes` 集中管理所有节次时间，便于维护。

**学习点：**
- Kairos 可将此配置硬编码在 Rust 后端常量中，或做成可配置的 JSON 文件。
- 注意兰州大学的节次表可能与其他学校不同。

### 1.5 请求头的精细控制

FasterLZU 在每个 Repository 方法中按需覆盖 headers（`Options(headers: {...})`），而非全局设置。

**学习点：**
- 按请求级别控制 headers 比全局修改更可预测、更安全。

### 1.6 用户名作为 token key 前缀

`${username}login_token` / `${username}gateway_token` 使用用户名作为 token 存储 key 的前缀，支持多账号切换。

**学习点：**
- Kairos 后端可以考虑以 user_id 为 token 的关联键，支持多用户。

---

## 2. 不建议照搬的问题

### 2.1 ⚠️ IV 与 Key 相同

```dart
final iv = e.IV.fromUtf8(key);  // key 和 iv 是同一个值
final cipher = e.AES(e.Key.fromUtf8(key), mode: e.AESMode.cbc, padding: null);
```

**问题：** 这在密码学上是弱配置（CBC 模式下 IV 与 Key 相同降低了安全性）。

**Kairos 建议：** 既然这是与 LZU 服务器兼容的要求，必须如此实现。但：
- 确保 Rust 实现的 AES-CBC 与 FasterLZU 行为完全一致（包括 `\0` padding）。
- 用集成测试验证加解密互操作性。

### 2.2 ⚠️ 非标准 padding

```dart
// padding: null — 无自动填充，手动补 \0
final paddedLen = dataLen + (blockSize - (dataLen % blockSize)) % blockSize;
final padded = Uint8List(paddedLen);
padded.setRange(0, dataLen, plaintext);
// ... 解密后从尾部去除 \0
```

**问题：** 不使用标准 PKCS7，而是用 `\0` 填充。如果明文字节本身就包含 `\0` 会出问题。

**Kairos 建议：**
- 必须保持兼容，使用相同的 `\0` 填充方式。
- 在 Rust 中使用 `aes::Aes128` + `Cbc` 模式，手动处理 padding。
- 用测试向量验证加解密一致性。

### 2.3 ⚠️ 通过修改全局 Dio 实例的 headers 来切换加密行为

```dart
// schedule_repository.dart:65-76
_dio.options.headers.remove('Transfer-Encrypt');
// ... 请求 ...
_dio.options.headers['Transfer-Encrypt'] = 'true';
```

**问题：** 并发环境下线程不安全。如果两个请求同时执行，一个修改了全局 `Transfer-Encrypt` 会影响另一个请求。

**Kairos 建议：**
- 每个请求独立控制加密开关，使用 `reqwest::RequestBuilder::header()`，而非修改全局 client。
- 或者通过自定义中间件按请求属性判断。

### 2.4 ⚠️ MD5 签名依赖 Map 迭代顺序

```dart
static String sign(Map<String, String> data) {
  String signStr = '';
  for (var v in data.values) {  // 依赖 Map.values 的迭代顺序
    signStr += '$v|';
  }
  return encrypt(signStr + AppConfig.md5Key);
}
```

**问题：** Dart 的 `Map` 是 LinkedHashMap（保留插入顺序），但 Rust 的 `HashMap` **不保证**迭代顺序。直接移植会导致签名不匹配。

**Kairos 建议：**
- Rust 实现必须使用 `indexmap::IndexMap` 或 `BTreeMap` 来固定字段顺序。
- 严格遵循字段插入顺序：AccNum → EPID → Time（以 GetWalletMoney 为例）。
- 建议为每个签名场景写显式测试验证签名结果。

### 2.5 ⚠️ XML 解析没有缺字段保护

```dart
code: int.parse(xml.findAllElements('Code').first.innerText),
```

**问题：** 如果 XML 中缺少某个元素，`.first` 会抛出 `StateError`（no element）。

**Kairos 建议：**
- Rust 中 XML 解析（如 `quick-xml`）需要处理字段缺失的情况，返回 `Option<String>` 或使用默认值。
- 参考 `GetOrderByCodeResponse.fromXml` 已经使用了 `firstOrNull`，这是好的模式。

### 2.6 ⚠️ schedule 请求中 Authorization token 的混用

```dart
// schedule_repository.dart:35
final st = await _authRepository.gatewayToken;
// 作为 Authorization 使用
```

`schedule_repository.dart` 将 `gatewayToken` 取名为 `st` 变量，容易让人误以为用的是 Service Ticket。实际上 `gateway_token` 和 `st` 是两个不同的 token。

**Kairos 建议：**
- 在 Kairos 的 token 管理中清晰区分：`login_token`、`gateway_token`、`st` 的角色。
- 确认课表接口真正使用的是 `gateway_token`（而非 `st`），并在文档中明确标注。

---

## 3. 安全注意事项

### 3.1 不持久化明文密码 ⚡

**FasterLZU 的做法：** 加密后缓存密码（`AESCrypto.encrypt(jsonEncode(postData))`）用于 `cachedLogin()`。

**Kairos 约束：**
- **Rust 后端绝不持久化明文密码。**
- 如果必须支持"保持登录"功能，应持久化 `refresh_token` 或 `login_token`（加密存储），而非密码。
- 即使 `cachedLogin` 功能需要，也应使用独立的长期 token 代替。

### 3.2 Token 不进入前端持久化状态

**原则：** 前端（Tauri WebView）不持有任何 token。

**实现方式：**
- 所有 API 请求通过 Tauri command → Rust 后端发起。
- Token 仅存在于 Rust 后端进程内存中（或加密后存于后端本地数据库）。
- 前端 JS 侧不出现 `login_token`、`gateway_token`、`st`、`EtToken` 的值。
- 前端 session 状态仅保存"是否已登录"这类布尔信息。

### 3.3 日志脱敏要求

| 禁止日志输出 | 原因 |
|---|---|
| 明文密码 | 凭证泄露 |
| `login_token` / `gateway_token` / `st` / `EtToken` | 会话劫持 |
| 完整 `Authorization` header | token 泄露 |
| 完整 AES 加密 payload | 已知 IV+Key 的情况下可解密 |
| 完整 MD5 签名请求 | 可能泄露签名构造方式 |

**允许：** `"POST /api/xxx (status: 200)"`, `"response time: 234ms"` 等无敏感信息的日志。

### 3.4 不要通过修改全局 HTTP client header 来切换加密行为

同 2.3。使用**每个请求独立的 header** 控制 `Transfer-Encrypt`。

### 3.5 EasyTong 的 HTTP 风险

EasyTong 使用 `http://app.lzu.edu.cn:8080`（HTTP 而非 HTTPS）。

**Kairos 建议：**
- MVP 阶段不实现 EasyTong，规避此风险。
- 后续实现时需要考虑中间人攻击风险。
- 如果必须使用，确保请求不包含密码等敏感信息（EtToken 本身已经通过 AppService 的 HTTPS 链路获得）。

---

## 4. Kairos 实现建议

### 4.1 Rust 后端模块划分

```
kairos-backend/
├── src/
│   ├── lzu/                    # LZU API 集成模块
│   │   ├── mod.rs
│   │   ├── client.rs           # HTTP client 封装（reqwest + 中间件）
│   │   ├── aes_crypto.rs       # AES-CBC hex 加解密（\0 padding）
│   │   ├── md5_sign.rs         # MD5 签名（固定字段顺序）
│   │   ├── auth.rs             # 登录 / token 管理
│   │   ├── schedule.rs         # 课表查询
│   │   └── models.rs           # 数据模型 + 字段映射
│   ├── commands/               # Tauri commands
│   │   ├── lzu_auth.rs
│   │   └── lzu_schedule.rs
│   └── ...
```

### 4.2 优先级路线

**Phase 1 (MVP)：** 仅 AppService 课表链路
1. `lzu_auth.rs` — 登录、token 管理（内存缓存）
2. `lzu_schedule.rs` — 获取学期 + 课表
3. `models.rs` — 字段映射到 Kairos 内模型

**Phase 2（后续）：** 课表写入、用户信息
**Phase 3（后续）：** EasyTong 一卡通
**Phase 4（后续）：** 二维码、订单

### 4.3 课表取数策略

- 首次同步时遍历所有周次（1-`XlxxData.zzx`），获取完整学期课表。
- 按周次缓存，增量更新。
- 使用 `XlxxData.ksrq` 将周次映射到具体日期。

### 4.4 错误处理策略

| 错误类型 | 处理方式 |
|---|---|
| 网络超时 | 重试 2 次（指数退避），最终返回可读错误 |
| 接口返回 `code != 1` | 记录 message，返回给前端 |
| token 过期 (401) | 尝试 `cachedLogin` 刷新，失败则要求用户重新登录 |
| 字段缺失 | Rust 端使用 `Option<T>`，不 panic |
| XML 解析失败 | 返回 `ParseError`，不 panic |

---

## 5. 后续任务边界说明

### 5.1 `lzu-auth-client`

| 范围 | 说明 |
|---|---|
| 实现内容 | 登录 API 调用、token 管理（login_token + gateway_token）、st 刷新、登出 |
| 不包含 | 课表、用户信息、EasyTong |
| 关键接口 | `POST login`, `GET getSt` |
| 安全红线 | 不持久化密码；token 仅存后端内存；日志脱敏 |
| 输入 | 学号 + 密码（来自用户输入，由 Tauri command 传递） |
| 输出 | 登录成功/失败状态 + token 缓存 |
| 参考文档 | `research/lzu-api-contract.md §2.1, §2.5, §4` |

### 5.2 `lzu-schedule-import`

| 范围 | 说明 |
|---|---|
| 实现内容 | 学期信息获取、按周次获取课表、字段映射到 Kairos 模型 |
| 不包含 | 自定义课表增删改、EasyTong |
| 关键接口 | `POST getXlxx`, `GET getZdyCourse` |
| 前置依赖 | `lzu-auth-client` 提供的 token 管理 |
| 输入 | 周次参数（或全量同步策略） |
| 输出 | 映射后的 Kairos 内部课表数据 |
| 字段映射 | `kcmc→name`, `skxql→day_of_week`, `skjsl→location`, `jsxm→teacher`, `week/week_fb→week_pattern`, `jc→start_time/end_time`, `xn/xqm→semester` |
| 参考文档 | `research/lzu-api-contract.md §2.3, §2.4, §7, §8` |

### 5.3 `lzu-import-ui`

| 范围 | 说明 |
|---|---|
| 实现内容 | 用户触发导入、显示导入进度、处理冲突、确认替换 |
| 不包含 | 课表编辑、显示（已有功能） |
| 前置依赖 | `lzu-schedule-import` 提供的数据 |
| 输入 | 用户确认导入操作 |
| 输出 | 已导入到 Kairos 数据库的课表 |

---

## 6. 风险检查清单

- [x] 不持久化明文密码 — **已确认约束**
- [x] token 不进入前端持久化状态 — **已确认原则**
- [x] 日志脱敏 — **已记录禁止项**
- [x] 不使用全局 header 切换加密 — **识别为风险点**
- [x] MD5 签名固定字段顺序 — **已记录隐患和修复方案**
- [x] XML 解析缺字段 panic — **已记录缓解方式**
- [x] QR code / 支付不进入 MVP — **已确认范畴**

---

## 7. 参考文件索引

| FasterLZU 源文件 | 内容 | 本文引用处 |
|---|---|---|
| `lib/app_config.dart` | 全局配置：URL、API 路径、headers、节次时间 | §1.4, 附录 |
| `lib/core/api/appservice_client.dart` | AppService Dio 实例 + EncryptInterceptor | §1.1, §2.3 |
| `lib/core/api/app_client.dart` | EasyTong Dio 实例（无加密中间件） | §3.5 |
| `lib/core/encryption/aes_crypto.dart` | AES-CBC hex 加解密实现 | §2.1, §2.2 |
| `lib/core/encryption/md5_crypto.dart` | MD5 签名实现 | §2.4 |
| `lib/core/auth/repositories/auth_repository.dart` | 登录/登出/token 管理 | §4 |
| `lib/core/schedule/repositories/schedule_repository.dart` | 课表/学期查询 | §2.6 |
| `lib/core/easytong/repositories/easytong_repository.dart` | EasyTong API 调用 | §2.4 |
| `lib/core/auth/models/auth_model.dart` | 登录请求/响应模型 | §4 |
| `lib/core/schedule/models/schedule_model.dart` | XlxxData, CourseInfo 模型 | §2.5 |
| `lib/core/easytong/models/easytong_model.dart` | EasyTong 模型 + XML 解析 | §2.5 |
