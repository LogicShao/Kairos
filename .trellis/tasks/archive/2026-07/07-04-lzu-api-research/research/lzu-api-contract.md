# LZU API 协议合同

> 基于 `_TEMP/fasterlzu` 源码 reverse-engineered 协议事实。
> 研究日期：2026-07-04 | 目标 Rust 后端接入参考。

---

## 1. 总览

| 系统 | Base URL | 加密方式 | 用途 | MVP 必需？ |
|---|---|---|---|---|
| **AppService** | `https://appservice.lzu.edu.cn` | AES-CBC-128 hex（payload 级别） | 登录、课表、学期信息、用户信息 | **是** |
| **EasyTong** | `http://app.lzu.edu.cn:8080` | MD5 签名（参数级别） | 一卡通余额、二维码、订单 | 否（后续扩展） |

---

## 2. AppService API Endpoint Matrix

**全局默认 headers:**
```json
{
  "User-Agent": "Mozilla/5.0 (Linux; Android 12; SM-S7110 Build/V417IR; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/101.0.4951.61 Mobile Safari/537.36 lzdx_ua JHZF_LZDXAPP",
  "Transfer-Encrypt": "true",
  "Host": "appservice.lzu.edu.cn",
  "Connection": "Keep-Alive",
  "Content-Type": "application/json;charset=UTF-8",
  "Accept-Encoding": "gzip"
}
```

**注意：** 个别请求会临时覆盖 `Transfer-Encrypt`、`Content-Type`、`Authorization`。详情见各 endpoint 备注。

**Base URL:** `https://appservice.lzu.edu.cn`

### 2.1 登录

| 属性 | 值 |
|---|---|
| **Path** | `/api/eusp-unify-terminal/app-user/login` |
| **Method** | `POST` |
| **Headers** | 默认 headers + `"Authorization": ""`（显式传空字符串） |
| **加密？** | **是** — body 被 AES-CBC hex 加密（因 `Transfer-Encrypt: true`，由 `EncryptInterceptor` 自动处理） |
| **Request body (明文原型)** | `{"app_os": 2, "name": "<username>", "pwd": "<password>"}` |
| **依赖 token？** | 否（登录接口本身不需要 token） |
| **Response 格式** | JSON（需先尝试直接 JSON 解析，失败则 AES 解密再 JSON 解析） |
| **Response 关键字段** | `code` (int), `message` (String), `data.login_token` (String?), `data.gateway_token` (String?), `data.reset_pwd_token` (String?), `data.need_conf_security` (String?) |
| **成功标志** | `code == 1` |
| **源码引用** | `lib/core/auth/repositories/auth_repository.dart:40-61`; `lib/app_config.dart:18` |

### 2.2 登出

| 属性 | 值 |
|---|---|
| **Path** | `/api/eusp-unify-terminal/app-user/logout` |
| **Method** | `POST` |
| **Headers** | 覆盖：`"Authorization": "<gateway_token>"`, `"Content-Type": "application/x-www-form-urlencoded"` |
| **加密？** | **是** — body 被 AES 加密 |
| **Request body (明文)** | `loginToken=<login_token>`（urlencoded 格式字符串） |
| **依赖 token？** | 是 — `login_token` 和 `gateway_token` |
| **Response 格式** | JSON（外层 `{code, message, data}`，data 为 String?） |
| **源码引用** | `lib/core/auth/repositories/auth_repository.dart:87-103` |

### 2.3 获取学期信息 (Xlxx)

| 属性 | 值 |
|---|---|
| **Path** | `/api/lzu-teaching-research/kcb/getXlxx` |
| **Method** | `POST` |
| **Headers** | 覆盖：`"Content-Type": "application/x-www-form-urlencoded"`, `"Authorization": "<gateway_token>"` |
| **加密？** | **否**（`Transfer-Encrypt` 不在自定义 headers 中） |
| **Request body** | 空（无 body） |
| **依赖 token？** | 是 — `gateway_token` |
| **Response 格式** | JSON `{code, message, data: XlxxData}` |
| **成功标志** | `code == 1` |
| **源码引用** | `lib/core/schedule/repositories/schedule_repository.dart:47-60` |

### 2.4 获取课表

| 属性 | 值 |
|---|---|
| **Path** | `/api/lzu-teaching-research/kcb/getZdyCourse` |
| **Method** | `GET` |
| **Headers** | 覆盖：`"Authorization": "<gateway_token>"`, `"Content-Type": "text/plain"` |
| **加密？** | **否**（无 `Transfer-Encrypt`） |
| **Query 参数** | `zc` (int, 周次), `qsbz` (int, 未知标志，FasterLZU 固定传 0) |
| **依赖 token？** | 是 — `gateway_token` |
| **Response 格式** | JSON `{code, message, data: [CourseInfo]}` |
| **成功标志** | `code == 1` |
| **源码引用** | `lib/core/schedule/repositories/schedule_repository.dart:33-45` |

### 2.5 获取 ST (Service Ticket)

| 属性 | 值 |
|---|---|
| **Path** | `/api/eusp-unify-terminal/app-user/getSt` |
| **Method** | `GET` |
| **Headers** | 默认 headers（不带额外 Authorization） |
| **加密？** | **否**（GET 请求，无 body 加密） |
| **Query 参数** | `loginToken=<login_token>&serviceId=<service_id>&service=` |
| **依赖 token？** | 是 — `login_token` |
| **Response 格式** | JSON `{code, message, data: String?}`（data 字段即为 st 值） |
| **成功标志** | `code == 1` |
| **源码引用** | `lib/core/auth/repositories/auth_repository.dart:134-149`; `lib/app_config.dart:22` |

### 2.6 获取用户信息

| 属性 | 值 |
|---|---|
| **Path** | `/api/eusp-unify-terminal/app-user/userInfo` |
| **Method** | `GET` |
| **Headers** | 默认 headers + `"Authorization": "<gateway_token>"` |
| **加密？** | **否** |
| **Query 参数** | `loginToken=<login_token>` |
| **依赖 token？** | 是 — `login_token` 和 `gateway_token` |
| **Response 格式** | JSON `{code, message, data: UserInfoData}` |
| **源码引用** | `lib/core/auth/repositories/auth_repository.dart:120-132` |

### 2.7 获取用户头像

| 属性 | 值 |
|---|---|
| **Path** | `/api/eusp-unify-terminal/app-user/userImg` |
| **Method** | `GET` |
| **Headers** | 默认 headers + `"Authorization": "<gateway_token>"` |
| **加密？** | **否** |
| **Query 参数** | `loginToken=<login_token>` |
| **依赖 token？** | 是 |
| **Response 格式** | JSON `{code, message, data: {zp, xykh, rylb, zpbbh, rn}}` |
| **源码引用** | `lib/core/auth/repositories/auth_repository.dart:105-118` |

### 2.8 自定义课表 — 添加课程 (AddSchedule)

| 属性 | 值 |
|---|---|
| **Path** | `/api/lzu-teaching-research/kcb/addorUpdateZdyKc` |
| **Method** | `POST` |
| **Headers** | 覆盖：`"Authorization": "<gateway_token>"`, `"Content-Type": "application/json;charset=UTF-8"`；**显式移除** `Transfer-Encrypt` |
| **加密？** | **否**（因移除了 `Transfer-Encrypt`） |
| **Request body** | `{"kclsit": [<AddScheduleData>]}` |
| **依赖 token？** | 是 |
| **Response 格式** | JSON |
| **源码引用** | `lib/core/schedule/repositories/schedule_repository.dart:62-80` |

### 2.9 自定义课表 — 删除课程 (DelSchedule)

| 属性 | 值 |
|---|---|
| **Path** | `/api/lzu-teaching-research/kcb/deleZdyKc` |
| **Method** | `POST` |
| **Headers** | 覆盖：`"Authorization": "<gateway_token>"`；**显式移除** `Transfer-Encrypt` |
| **加密？** | **否** |
| **Query 参数** | `kch=<课程号>` |
| **Request body** | `{}` |
| **依赖 token？** | 是 |
| **源码引用** | `lib/core/schedule/repositories/schedule_repository.dart:82-96` |

---

## 3. EasyTong API Endpoint Matrix

**全局默认 headers:**
```json
{
  "Host": "app.lzu.edu.cn:8080",
  "User-Agent": "Mozilla/5.0 (Linux; Android 12; ..."
}
```

**Base URL:** `http://app.lzu.edu.cn:8080`

> ⚠️ EasyTong 使用 **HTTP**（非 HTTPS），存在中间人攻击风险。
> ⚠️ EasyTong 在 MVP 阶段**不实现**，仅记录以供后续扩展。

### 3.1 交换 EtToken

| 属性 | 值 |
|---|---|
| **Path** | `/easytong_app/ExchangeEtToken` |
| **Method** | `POST` |
| **Headers** | `"Content-Type": "application/x-www-form-urlencoded"` |
| **加密？** | 否 |
| **Request body** | `Time=<yyyyMMddHHmmss>&St=<st>&ContentType=application%2Fjson` |
| **依赖 token？** | 需要先通过 AppService 获取 `st`（Service Ticket） |
| **Response 格式** | JSON `{code, msg, accNum, token}` |
| **成功标志** | `code == 1` |
| **源码引用** | `lib/core/easytong/repositories/easytong_repository.dart:38-62` |

### 3.2 获取一卡通信息

| 属性 | 值 |
|---|---|
| **Path** | `/easytong_app/GetAccInfo` |
| **Method** | `POST` |
| **Headers** | `"Authorization": "<EtToken>"`, `"Content-Type": "application/x-www-form-urlencoded"` |
| **加密？** | MD5 签名（非加密，是签名） |
| **Request body** | URL-encoded: `AccNum=<accNum>&Time=<yyyyMMddHHmmss>&Sign=<md5>&ContentType=application/json` |
| **依赖 token？** | 是 — `EtToken` |
| **Response 格式** | **XML**（非 JSON）`<EasyTong><Code>1</Code><Msg>...</Msg><CardAccNum>...</CardAccNum><EPID>...</EPID></EasyTong>` |
| **源码引用** | `lib/core/easytong/repositories/easytong_repository.dart:112-146` |

### 3.3 获取钱包余额

| 属性 | 值 |
|---|---|
| **Path** | `/easytong_app/GetWalletMoney` |
| **Method** | `POST` |
| **Headers** | `"Authorization": "<EtToken>"`, `"Content-Type": "application/x-www-form-urlencoded"` |
| **加密？** | MD5 签名 |
| **Request body** | URL-encoded: `AccNum=<accNum>&EPID=<epid>&Time=<time>&Sign=<md5>&ContentType=application%2Fjson` |
| **Response 格式** | **XML** `<EasyTong><Code>1</Code><Msg>...</Msg><Table>...</Table></EasyTong>` |
| **源码引用** | `lib/core/easytong/repositories/easytong_repository.dart:82-110` |

### 3.4 获取二维码

| 属性 | 值 |
|---|---|
| **Path** | `/easytong_app/getH5QRCode` |
| **Method** | `POST` |
| **Headers** | `"Authorization": "<EtToken>"`, `"Content-Type": "application/x-www-form-urlencoded"` |
| **加密？** | MD5 签名 |
| **Request body** | URL-encoded: `AccNum=<accNum>&Time=<time>&Sign=<md5>&ContentType=application/json` |
| **Response 格式** | JSON |
| **源码引用** | `lib/core/easytong/repositories/easytong_repository.dart:148-182` |

### 3.5 订单查询

| 属性 | 值 |
|---|---|
| **Path** | `/easytong_app/GetOrderByCode` |
| **Method** | `POST` |
| **Headers** | `"Authorization": "<EtToken>"`, `"Content-Type": "application/x-www-form-urlencoded"` |
| **加密？** | MD5 签名 |
| **Request body** | URL-encoded: `AccNum=<accNum>&AuthCode=<authCode>&CardAccNum=<cardAccNum>&Time=<time>&Sign=<md5>&ContentType=application/json` |
| **Response 格式** | **XML** |
| **源码引用** | `lib/core/easytong/repositories/easytong_repository.dart:184-214` |

---

## 4. Token 生命周期

```
┌─────────────────────────────────────────────────────────┐
│                    AppService (https)                    │
│                                                         │
│  登录 ──→ login_token + gateway_token                   │
│                                                         │
│  login_token ──→ getSt() ──→ st (Service Ticket)       │
│                                                         │
│  gateway_token ──→ Authorization header (大部分接口)    │
│                                                         │
│  st ──→ EasyTong ExchangeEtToken ──→ EtToken           │
└─────────────────────────────────────────────────────────┘
```

### 4.1 login_token

- **来源**: 登录成功后的 `LoginResponse.data.login_token`
- **格式**: 不确定（FasterLZU 直接使用，未记录格式）
- **用途**: 作为参数传给 `userInfo`, `userImg`, `getSt` 等查询接口
- **有效期**: 服务器端定义，过期后需要重新登录
- **存储**: FasterLZU 用 `FlutterSecureStorage` 以 `${username}login_token` 为 key 持久化
- **Kairos 建议**: 仅存于 Rust 后端内存/Redis，不发给前端

### 4.2 gateway_token

- **来源**: 登录成功后的 `LoginResponse.data.gateway_token`
- **格式**: 字符串，作为 `Authorization: Bearer <value>` 使用
- **用途**: 大部分 AppService 接口的 HTTP `Authorization` header
- **有效期**: 与 `login_token` 相同
- **存储**: FasterLZU 用 `FlutterSecureStorage` 以 `${username}gateway_token` 持久化
- **Kairos 建议**: 仅存于 Rust 后端内存，**_不_**用于前端 header

### 4.3 st (Service Ticket)

- **来源**: 通过 `getSt` 接口获取（需要 `login_token`）
- **格式**: 字符串（`StResponse.data` 是 String?）
- **用途**:
  - 课表请求的 `Authorization` header（FasterLZU 中 `schedule_repository.dart:36` 错误地把 gatewayToken 当作 st 使用——实际上是 gateway_token 被用到 `Authorization`）
  - EasyTong 的 `ExchangeEtToken` 参数
- **有效期**: 较短（推测的临时 ticket）
- **Kairos 注意**: FasterLZU 的 schedule 接口使用 `gatewayToken`（AuthRepository 的 getter）而非 `st`。需要澄清哪个 token 真正有效。代码中 `schedule_repository.dart:35` 调用 `_authRepository.gatewayToken`，但 `auth_repository.dart` 中 `get gatewayToken` 对应的是 `gateway_token`。

### 4.4 EtToken

- **来源**: EasyTong `ExchangeEtToken` 接口（需要 `st`）
- **格式**: 字符串（`EtTokenResponse.token`）
- **用途**: EasyTong 系列接口的 `Authorization` header
- **有效期**: 未知（FasterLZU 的 `getEtToken()` 有简单缓存未过期判断）
- **Kairos 建议**: MVP 不涉及

---

## 5. AES-CBC-128 Hex 加解密协议

### 5.1 参数

| 参数 | 值 |
|---|---|
| 算法 | AES-CBC-128 |
| Key | 16 字节（源码中 `AppConfig.aesKey`，占位 `REMOVED_SECRET`） |
| IV | **与 Key 相同**（即 IV = Key = 16 字节 UTF-8 字符串） |
| Padding | **手动 PKCS7**（FasterLZU 未使用标准 PKCS7 填充，而是用 `\0` 补足到 16 倍数） |
| 输出格式 | **hex 小写**（每字节转两位十六进制字符串） |
| 输入编码 | UTF-8 |

### 5.2 加密流程

```text
1. 明文字符串 → UTF-8 bytes
2. 用 \0 补足到 16 的倍数
3. AES-CBC 加密（key=IV=16 字节字符串）
4. 密文字节逐一转为 2 位十六进制字符串
```

**源码引用**: `lib/core/encryption/aes_crypto.dart`

### 5.3 解密流程

```text
1. hex 字符串 → bytes
2. AES-CBC 解密（key=IV）
3. 去除尾部 \0（非标准 PKCS7 unpad，而是从尾部往前找第一个非零字节）
4. 剩余 bytes → UTF-8 字符串
```

### 5.4 不标准之处

1. **IV 与 Key 相同** — 这是安全弱点，但 LZU 后端协议如此，必须遵循。
2. **填充方式** — 用 `\0` 而非标准 PKCS7。解密时从尾部往前移除连续 `\0`。
3. **hex 编码** — 输出/输入均为十六进制字符串，而非 base64。

**Kairos 实现注意**: 在 Rust 中需要使用 `aes` crate（CBC 模式）+ 手动 `\0` padding + hex 编解码。

---

## 6. MD5 签名协议

### 6.1 签名流程

```text
1. 对 Map 的 value 按**迭代顺序**拼接，以 | 分隔：v1|v2|v3|
2. 在末尾 + md5Key
3. 对整个字符串做 MD5 hash
```

**源码引用**: `lib/core/encryption/md5_crypto.dart`

### 6.2 实现注意事项

⚠️ **FasterLZU 的 MD5 签名存在隐患**：
- Dart 的 `Map.values` 迭代顺序受插入顺序影响，**与 HashMap 不同**。
- 当移植到 Rust 时，必须使用 `IndexMap` 或 `BTreeMap` 来**固定字段顺序**，否则签名不匹配。
- `Map<String, String>` 的 `values` 顺序 = 插入顺序，Rust 的 `HashMap` 不保证顺序。

### 6.3 签名参数字段顺序（以 GetWalletMoney 为例）

```text
AccNum → EPID → Time → (拼接后追加 md5Key)
```

字段顺序由插入代码决定（参见 `lib/core/easytong/repositories/easytong_repository.dart:87-91`）。

---

## 7. 字段模型

### 7.1 XlxxData（学期信息）

| 字段 | 含义 | 类型 | 示例 |
|---|---|---|---|
| `dqrqszzc` | 当前日期所在周次 | String? | `"1"` |
| `zzx` | 总周次 | String? | `"16"` |
| `ksrq` | 学期开始日期 | String? | `"2025-03-03"` |
| `xqm` | 学期序号 | String? | `"1"` (秋季) / `"2"` (春季) |
| `xn` | 学年 | String? | `"2025"` |
| `xq` | 学期名称 | String? | `"2024-2025学年 第一学期"` 或类似 |

**源码引用**: `lib/core/schedule/models/schedule_model.dart:7-19`

> ⚠️ 注意：`xqm` 的类型是 `String?`（XlxxData 中），但 `CourseInfo.xqm` 是 `int?`。

### 7.2 CourseInfo（课程信息）

| 字段 | 含义 | 类型 | 示例 |
|---|---|---|---|
| `kch` | 课程号 | String? | `"105404003"` |
| `kcmc` | 课程名称 | String? | `"通信原理"` |
| `jsxm` | 教师姓名 | String? | `"张冠茂"` |
| `jc` | 节次位图 | String? | `"11000000000000"`（14 位，1 表示有课） |
| `skjsl` | 上课教室 | String? | `"秦岭堂B404"` |
| `skxql` | 上课星期几 | String? | `"2"`（1=周一…7=周日） |
| `week` | 周次 | String? | `"1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16"` |
| `week_fb` | 单双周过滤 | String? | `"2,4,6,8"` 或 `null` |
| `bs` | 班号？ | String? | `"2"` |
| `xykh` | 校园卡号 | String? | `"320230941441"` |
| `xn` | 学年 | String? | `"2025"` |
| `xqm` | 学期序号 | int? | `2` |
| `status` | 状态 | int? | `null` |
| `color` | 颜色 | String? | `null` |
| `sksj` | 上课节次描述 | String? | `"上午12节"` |
| `xf` | 学分 | String? | `"3.5"` |
| `kcrq` | 开课日期 | String? | `"2025/03/18"` |
| `create_time` | 创建时间 | String? | `null` |
| `create_user_id` | 创建者 | String? | `null` |

**源码引用**: `lib/core/schedule/models/schedule_model.dart:45-71`

### 7.3 jc（节次位图）编码规则

`jc` 字段是一个 14 字符的字符串，每位代表一个节次段：

| 索引 | 节次 | 开始时间 | 结束时间 |
|---|---|---|---|
| 0 | 1 | 08:30 | 09:15 |
| 1 | 2 | 09:25 | 10:10 |
| 2 | 3 | 10:30 | 11:15 |
| 3 | 4 | 11:25 | 12:10 |
| 4 | 中 | 12:30 | 13:15 |
| 5 | 中2 | 13:25 | 14:00 |
| 6 | 5 | 14:30 | 15:15 |
| 7 | 6 | 15:25 | 16:10 |
| 8 | 7 | 16:30 | 17:15 |
| 9 | 8 | 17:25 | 18:10 |
| 10 | 9 | 19:00 | 19:45 |
| 11 | 10 | 19:55 | 20:40 |
| 12 | 11 | 21:00 | 21:55 |
| 13 | 12 | 21:55 | 22:40 |

例：`"11000000000000"` 表示第 1-2 节。

**源码引用**: `lib/app_config.dart:54-56`

---

## 8. Kairos 字段映射建议

| 学期/课表 | LZU API 字段 | Kairos 内字段 | 说明 |
|---|---|---|---|
| 课程名称 | `CourseInfo.kcmc` | `name` | 直接映射 |
| 上课日 | `CourseInfo.skxql` | `day_of_week` | 值范围 1-7，Kairos 可能需要 0-indexed |
| 教室 | `CourseInfo.skjsl` | `location` | 直接映射 |
| 教师 | `CourseInfo.jsxm` | `teacher` | 直接映射 |
| 周次模式 | `CourseInfo.week` + `week_fb` | `week_pattern` | 联合编码，`week` 列出所有有效周，`week_fb` 为非 null 时进一步过滤 |
| 节次时间 | `CourseInfo.jc` | `start_time`/`end_time` | 位图 -> 最早节次的开始时间 / 最末节次的结束时间 |
| 学期标识 | `CourseInfo.xn` + `xqm` | `semester` | 拼接为 `"2025-2"` 等形式 |
| 学期开始日 | `XlxxData.ksrq` | `semester_start_date` | 用于周次→具体日期换算 |

### 补充说明

- `jc` 位图解析后，`start_time` 取第一个 '1' 对应的 `classStartTimes`，`end_time` 取最后一个 '1' 对应的 `classEndTimes`。
- `week` 字段以逗号分隔的周次数列表。`week_fb` 为 null 时所有周有效，非 null 时取交集。
- `semester_start_date` 来自 `XlxxData.ksrq`，需要与 `Xlxx` 接口获取。

---

## 9. MVP 范围声明

**MVP 只依赖 `AppService` 课表链路：**

```
登录 → 获取 Xlxx（学期信息） → 按周次获取课表
```

涉及的接口：
1. `POST /api/eusp-unify-terminal/app-user/login` — 登录
2. `POST /api/lzu-teaching-research/kcb/getXlxx` — 学期信息
3. `GET /api/lzu-teaching-research/kcb/getZdyCourse` — 课表

EasyTong 相关全部（EtToken、余额、二维码、订单）**不阻塞 MVP**。

---

## 10. 附录：API 路径索引

| 接口名 | app_config 键 | 路径 |
|---|---|---|
| 登录 | `login` | `/api/eusp-unify-terminal/app-user/login` |
| 登出 | `logout` | `/api/eusp-unify-terminal/app-user/logout` |
| 课表 | `schedule` | `/api/lzu-teaching-research/kcb/getZdyCourse` |
| 学期信息 | `xlxx` | `/api/lzu-teaching-research/kcb/getXlxx` |
| ST | `st` | `/api/eusp-unify-terminal/app-user/getSt` |
| 用户信息 | `userInfo` | `/api/eusp-unify-terminal/app-user/userInfo` |
| 用户头像 | `userImg` | `/api/eusp-unify-terminal/app-user/userImg` |
| 添加自定义课 | `addSchedule` | `/api/lzu-teaching-research/kcb/addorUpdateZdyKc` |
| 删除自定义课 | `delSchedule` | `/api/lzu-teaching-research/kcb/deleZdyKc` |
| EtToken | `etToken` | `/easytong_app/ExchangeEtToken` |
| 一卡通信息 | `GetAccInfo` | `/easytong_app/GetAccInfo` |
| 二维码 | `qrCode` | `/easytong_app/getH5QRCode` |
| 钱包余额 | `GetWalletMoney` | `/easytong_app/GetWalletMoney` |
| 订单查询 | `GetOrderByCode` | `/easytong_app/GetOrderByCode` |
