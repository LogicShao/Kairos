# FasterLZU API 文档

## 项目概述

FasterLZU 是一个为兰州大学开发的第三方移动应用，使用 Flutter 框架构建。项目采用 Clean Architecture 架构，使用 Riverpod 进行状态管理，Dio 作为 HTTP 客户端。

## API 服务概览

项目使用两个主要的 API 服务：

1. **应用服务 API** (`https://appservice.lzu.edu.cn`)
   - 负责用户认证、课表、用户信息等核心功能
   - 使用 AES 加密传输敏感数据
   - 通过 `Transfer-Encrypt: 'true'` header 标识加密请求

2. **基础应用 API** (`http://app.lzu.edu.cn:8080`)
   - 负责校园卡（EasyTong）相关功能
   - 使用 MD5 签名验证
   - 返回 XML 格式数据

## 通用响应格式

### 标准 JSON 响应格式
```json
{
  "code": 1,          // 状态码：1=成功，其他=失败
  "message": "成功",   // 响应消息
  "data": {}          // 响应数据，类型根据接口不同而变化
}
```

### XML 响应格式（EasyTong 接口）
```xml
<EasyTong>
  <Code>1</Code>
  <Msg>成功</Msg>
  <!-- 其他数据字段 -->
</EasyTong>
```

## 应用服务 API 接口

### 1. 用户认证接口

#### 1.1 用户登录
- **端点**: `POST /api/eusp-unify-terminal/app-user/login`
- **描述**: 用户登录获取认证令牌
- **请求头**:
  - `Transfer-Encrypt: 'true'` (数据加密)
  - `Authorization: ''` (空值)
- **请求体**:
  ```json
  {
    "app_os": 2,        // 应用操作系统：2=Android
    "name": "学号",      // 用户名（学号）
    "pwd": "密码"       // 密码
  }
  ```
- **响应体** (`LoginResponse`):
  ```json
  {
    "code": 1,
    "message": "成功",
    "data": {
      "login_token": "登录令牌",
      "gateway_token": "网关令牌",
      "reset_pwd_token": "重置密码令牌",
      "need_conf_security": "是否需要安全确认"
    }
  }
  ```
- **相关文件**: `lib/core/auth/repositories/auth_repository.dart:40`

#### 1.2 缓存登录
- **端点**: `POST /api/eusp-unify-terminal/app-user/login`
- **描述**: 使用缓存的登录数据进行自动登录
- **请求头**: 同登录接口
- **请求体**: 加密的缓存登录数据
- **响应体**: 同登录接口
- **相关文件**: `lib/core/auth/repositories/auth_repository.dart:63`

#### 1.3 用户登出
- **端点**: `POST /api/eusp-unify-terminal/app-user/logout`
- **描述**: 用户退出登录
- **请求头**:
  - `Authorization: gateway_token`
  - `Content-Type: application/x-www-form-urlencoded`
- **请求体**: 加密的 `loginToken=${login_token}`
- **响应体** (`StringDataResponse`):
  ```json
  {
    "code": 1,
    "message": "成功",
    "data": "成功消息"
  }
  ```
- **相关文件**: `lib/core/auth/repositories/auth_repository.dart:87`

#### 1.4 获取用户头像
- **端点**: `GET /api/eusp-unify-terminal/app-user/userImg`
- **描述**: 获取用户头像图片信息
- **请求头**: `Authorization: gateway_token`
- **查询参数** (`UserImageRequest`):
  ```json
  {
    "loginToken": "登录令牌"
  }
  ```
- **响应体** (`UserImageResponse`):
  ```json
  {
    "code": 1,
    "message": "成功",
    "data": {
      "zp": "头像图片URL",
      "xykh": "校园卡号",
      "rylb": "人员类别",
      "zpbbh": "照片版本号",
      "rn": "人员编号"
    }
  }
  ```
- **相关文件**: `lib/core/auth/repositories/auth_repository.dart:105`

#### 1.5 获取用户信息
- **端点**: `GET /api/eusp-unify-terminal/app-user/userInfo`
- **描述**: 获取详细的用户个人信息
- **请求头**: `Authorization: gateway_token`
- **查询参数** (`UserInfoRequest`):
  ```json
  {
    "loginToken": "登录令牌"
  }
  ```
- **响应体** (`UserInfoResponse`):
  ```json
  {
    "code": 1,
    "message": "成功",
    "data": {
      "xtid": "系统ID",
      "rylbm": "人员类别码",
      "rylb": "人员类别",
      "xykh": "校园卡号",
      "rybh": "人员编号",
      "xm": "姓名",
      "xmpy": "姓名拼音",
      "dwh": "单位号",
      "dwmc": "单位名称",
      "xbm": "性别码",
      "dqztm": "当前状态码",
      "dqzt": "当前状态",
      "sfzjlxm": "身份证件类型码",
      "sfzjh": "身份证件号",
      "csrq": "出生日期",
      "mzm": "民族码",
      "mz": "民族",
      "zzmmm": "政治面貌码",
      "jxrq": "入学日期",
      "gjdqm": "国家地区码",
      "csdm": "出生地码",
      "jg": "籍贯",
      "yddh": "移动电话",
      "dzxx": "电子信箱",
      "sfzx": "是否在校",
      "sfxj": "是否休学",
      "bz": "备注",
      "zhgxsj": "最后更新时间",
      "xb": "性别",
      "etong_acc_no": "校园卡账号",
      "wxh": "微信号",
      "nc": "昵称",
      "gxqm": "个性签名"
    }
  }
  ```
- **相关文件**: `lib/core/auth/repositories/auth_repository.dart:120`

#### 1.6 获取 ST 令牌
- **端点**: `GET /api/eusp-unify-terminal/app-user/getSt`
- **描述**: 获取服务令牌（Service Token），用于访问其他服务
- **查询参数**:
  ```json
  {
    "loginToken": "登录令牌",
    "serviceId": "服务ID",
    "service": ""
  }
  ```
- **响应体** (`StringDataResponse`):
  ```json
  {
    "code": 1,
    "message": "成功",
    "data": "ST令牌字符串"
  }
  ```
- **相关文件**: `lib/core/auth/repositories/auth_repository.dart:134`

### 2. 课表相关接口

#### 2.1 获取课表
- **端点**: `GET /api/lzu-teaching-research/kcb/getZdyCourse`
- **描述**: 获取指定周次的课表信息
- **请求头**:
  - `Authorization: gateway_token`
  - `Content-Type: text/plain`
- **查询参数** (`ScheduleRequest`):
  ```json
  {
    "zc": 1,    // 周次（1-20）
    "qsbz": 0   // 起始标志：0=正常
  }
  ```
- **响应体** (`ScheduleResponse`):
  ```json
  {
    "code": 1,
    "message": "成功",
    "data": [
      {
        "kch": "课程号",
        "kcmc": "课程名称",
        "jsxm": "教师姓名",
        "jc": "节次编码（14位二进制）",
        "skjsl": "上课教室",
        "skxql": "上课星期几（1-7）",
        "week": "周次列表（逗号分隔）",
        "bs": "班号",
        "xykh": "校园卡号",
        "xn": "学年",
        "xqm": "学期码",
        "status": "状态",
        "color": "颜色代码",
        "skjc": "上课节次描述",
        "xf": "学分",
        "week_fb": "周次分布",
        "kcrq": "开课日期",
        "create_time": "创建时间",
        "create_user_id": "创建用户ID"
      }
    ]
  }
  ```
- **相关文件**: `lib/core/schedule/repositories/schedule_repository.dart:33`

#### 2.2 获取学期信息
- **端点**: `POST /api/lzu-teaching-research/kcb/getXlxx`
- **描述**: 获取当前学期信息（学年、学期、当前周次等）
- **请求头**:
  - `Authorization: gateway_token`
  - `Content-Type: application/x-www-form-urlencoded`
- **响应体** (`XlxxResponse`):
  ```json
  {
    "code": 1,
    "message": "成功",
    "data": {
      "dqrqszzc": "当前日期所在周次",
      "zzx": "总周数",
      "ksrq": "开始日期",
      "xqm": "星期几码",
      "xn": "学年",
      "xq": "学期"
    }
  }
  ```
- **相关文件**: `lib/core/schedule/repositories/schedule_repository.dart:47`

#### 2.3 添加自定义课程
- **端点**: `POST /api/lzu-teaching-research/kcb/addorUpdateZdyKc`
- **描述**: 添加或更新自定义课程
- **注意**: 此接口不使用加密传输（临时移除 `Transfer-Encrypt` header）
- **请求头**:
  - `Authorization: gateway_token`
  - `Content-Type: application/json;charset=UTF-8`
- **请求体** (`AddScheduleRequest`):
  ```json
  {
    "kclsit": [
      {
        "kcmc": "课程名称",
        "jsxm": "教师姓名",
        "xf": "学分",
        "color": "颜色代码（如：rgb(148,255,166,0.74)）",
        "skjsl": "上课教室",
        "skxql": "上课星期几（1-7）",
        "week": "周次列表（逗号分隔）",
        "week_fb": "周次分布（同week）",
        "jc": "节次编码（14位二进制，如：00000011000000）",
        "bs": 1,
        "skjc": "上课节次描述（如：周一，第5节 - 第6节）"
      }
    ]
  }
  ```
- **响应体** (`AddScheduleResponse`):
  ```json
  {
    "code": 1,
    "message": "成功",
    "data": "成功消息"
  }
  ```
- **相关文件**: `lib/core/schedule/repositories/schedule_repository.dart:62`

#### 2.4 删除自定义课程
- **端点**: `POST /api/lzu-teaching-research/kcb/deleZdyKc`
- **描述**: 删除自定义课程
- **注意**: 此接口不使用加密传输（临时移除 `Transfer-Encrypt` header）
- **请求头**: `Authorization: gateway_token`
- **查询参数**: `kch=课程号`
- **请求体**: `{}` (空对象)
- **响应体** (`DelScheduleResponse`):
  ```json
  {
    "code": 1,
    "message": "成功",
    "data": "成功消息"
  }
  ```
- **相关文件**: `lib/core/schedule/repositories/schedule_repository.dart:82`

### 3. 应用服务接口

#### 3.1 获取服务详情
- **端点**: `POST /api/eusp-terminal-management/api/v2/getServiceInfoDetailByTerminalRole`
- **描述**: 获取终端角色可访问的服务详细信息
- **请求头**: `Content-Type: application/x-www-form-urlencoded`
- **请求体**: 加密的 `terminalId=1&loginToken=${login_token}`
- **响应体** (`DetailedAppResponse`):
  ```json
  {
    "code": 1,
    "message": "成功",
    "data": [
      {
        "service_type_id": "服务类型ID",
        "service_type_name": "服务类型名称",
        "service_type_icon": "服务类型图标",
        "service_type_icon_url": "服务类型图标URL",
        "service_type_sort": 1,
        "status": 1,
        "create_time": "创建时间",
        "create_user_id": "创建用户ID",
        "service_infos": [
          {
            "service_info_id": "服务信息ID",
            "service_name": "服务名称",
            "app_icon": "应用图标",
            "pc_icon": "PC图标",
            "unint_id": "单位ID",
            "app_type": 1,
            "service_type": "服务类型",
            "service_type_str": "服务类型字符串",
            "android_main": "Android主入口",
            "ios_main": "iOS主入口",
            "h5_url": "H5 URL",
            "h5_pcurl": "H5 PC URL",
            "version_id": "版本ID",
            "is_recommend": 0,
            "start_time": "开始时间",
            "end_time": "结束时间",
            "service_sort": 1,
            "status": 1,
            "create_time": "创建时间",
            "create_user_id": "创建用户ID",
            "is_login": 1,
            "is_new": 0,
            "is_top": 0,
            "is_hot": 0,
            "is_pay": 0,
            "isfull_screen": 0,
            "is_ignore_login": 0,
            "cp_code": "CP代码",
            "sign_key": "签名密钥",
            "app_id": "应用ID",
            "lzu_sign_key": "LZU签名密钥",
            "h5_service_url": "H5服务URL",
            "app_icon_url": "应用图标URL",
            "pc_icon_url": "PC图标URL",
            "unint_name": "单位名称",
            "role_str": "角色字符串",
            "terminal_str": "终端字符串",
            "terminal_ids": "终端ID列表",
            "start_time_str": "开始时间字符串",
            "end_time_str": "结束时间字符串",
            "roles": "角色",
            "role_ids": "角色ID",
            "terminals": "终端",
            "key_word": "关键词",
            "use_system": "使用系统",
            "first_letter": "首字母",
            "introduce": "介绍",
            "condition": "条件",
            "need_attention": "注意事项",
            "contact_phone": "联系电话",
            "oh_service_id": "OH服务ID",
            "object_ids": "对象ID",
            "object_ids_str": "对象ID字符串",
            "pc_show_type": "PC显示类型",
            "has_collected": "是否收藏",
            "fee_scale": "费用比例",
            "fee_scale_str": "费用比例字符串",
            "handle_method": "处理方法",
            "handle_method_str": "处理方法字符串",
            "co_organizer": "协办单位",
            "co_organizer_str": "协办单位字符串",
            "expected_period": "预期周期",
            "expected_period_str": "预期周期字符串",
            "is_show_detail": 0,
            "process_img_source": 0,
            "process_img_url": "流程图片URL",
            "pxyj": "培训要求",
            "cjsj": "创建时间",
            "use_systems": ["使用系统列表"],
            "oh_appid": "OH应用ID",
            "can_consult": "可咨询",
            "can_evaluate": "可评价",
            "monitor_roleid": "监控角色ID",
            "maintainer_roleid": "维护角色ID"
          }
        ]
      }
    ]
  }
  ```
- **相关文件**: `lib/core/app/repositories/app_repository.dart:35`

## 基础应用 API 接口（EasyTong 校园卡）

### 4. 校园卡接口

#### 4.1 交换 ET 令牌
- **端点**: `POST /easytong_app/ExchangeEtToken`
- **描述**: 使用 ST 令牌交换 EasyTong 令牌
- **请求头**: `Content-Type: application/x-www-form-urlencoded`
- **请求体**: `Time=20250101120000&St=ST令牌&ContentType=application%2Fjson`
- **响应体** (`EtTokenResponse`):
  ```json
  {
    "msg": "成功消息",
    "accNum": "账户号",
    "code": 1,
    "token": "ET令牌"
  }
  ```
- **相关文件**: `lib/core/easytong/repositories/easytong_repository.dart:38`

#### 4.2 获取账户信息
- **端点**: `POST /easytong_app/GetAccInfo`
- **描述**: 获取校园卡账户详细信息
- **请求头**:
  - `Authorization: ET令牌`
  - `Content-Type: application/x-www-form-urlencoded`
- **请求体**: URL 编码的查询字符串，包含 MD5 签名
  ```
  AccNum=账户号&Time=时间戳&Sign=MD5签名&ContentType=application%2Fjson
  ```
- **签名算法**: `MD5(AccNum + Time + MD5密钥)`
- **响应体** (XML):
  ```xml
  <EasyTong>
    <Code>1</Code>
    <Msg>成功</Msg>
    <CardAccNum>卡账户号</CardAccNum>
    <EPID>EPID</EPID>
  </EasyTong>
  ```
- **相关文件**: `lib/core/easytong/repositories/easytong_repository.dart:112`

#### 4.3 获取钱包余额
- **端点**: `POST /easytong_app/GetWalletMoney`
- **描述**: 获取校园卡各钱包余额
- **请求头**:
  - `Authorization: ET令牌`
  - `Content-Type: application/x-www-form-urlencoded`
- **请求体**: URL 编码的查询字符串，包含 MD5 签名
  ```
  AccNum=账户号&EPID=EPID&Time=时间戳&Sign=MD5签名&ContentType=application%2Fjson
  ```
- **签名算法**: `MD5(AccNum + EPID + Time + MD5密钥)`
- **响应体** (XML):
  ```xml
  <EasyTong>
    <Code>1</Code>
    <Msg>成功</Msg>
    <Table>
      <CardName>卡名称</CardName>
      <CardAccNum>卡账户号</CardAccNum>
      <Unit>单位</Unit>
      <WalletMoney>钱包余额</WalletMoney>
      <WalletName>钱包名称</WalletName>
      <IsWithdraw>是否可提现</IsWithdraw>
      <MoneyMax>最大金额</MoneyMax>
      <MonTemp>临时金额</MonTemp>
      <MonCard>卡金额</MonCard>
      <WalletNum>钱包编号</WalletNum>
    </Table>
    <!-- 可能有多个 Table 元素 -->
  </EasyTong>
  ```
- **相关文件**: `lib/core/easytong/repositories/easytong_repository.dart:82`

#### 4.4 获取 H5 二维码
- **端点**: `POST /easytong_app/getH5QRCode`
- **描述**: 获取用于支付的 H5 页面二维码
- **请求头**:
  - `Authorization: ET令牌`
  - `Content-Type: application/x-www-form-urlencoded`
- **请求体**: URL 编码的查询字符串，包含 MD5 签名
  ```
  AccNum=账户号&Time=时间戳&Sign=MD5签名&ContentType=application%2Fjson
  ```
- **签名算法**: `MD5(AccNum + Time + MD5密钥)`
- **响应体** (JSON):
  ```json
  {
    "authNum": "授权号",
    "cardAccNum": "卡账户号",
    "qRCode": "二维码数据",
    "code": 1,
    "msg": "成功"
  }
  ```
- **相关文件**: `lib/core/easytong/repositories/easytong_repository.dart:148`

#### 4.5 通过授权码获取订单
- **端点**: `POST /easytong_app/GetOrderByCode`
- **描述**: 通过授权码查询订单详情
- **请求头**:
  - `Authorization: ET令牌`
  - `Content-Type: application/x-www-form-urlencoded`
- **请求体**: URL 编码的查询字符串，包含 MD5 签名
  ```
  AccNum=账户号&AuthCode=授权码&CardAccNum=卡账户号&Time=时间戳&Sign=MD5签名&ContentType=application%2Fjson
  ```
- **签名算法**: `MD5(AccNum + AuthCode + CardAccNum + Time + MD5密钥)`
- **响应体** (XML):
  ```xml
  <EasyTong>
    <Code>1</Code>
    <Msg>成功</Msg>
    <CompareType>比较类型</CompareType>
    <DealTime>交易时间</DealTime>
    <Discount>折扣</Discount>
    <ManagerFee>管理费</ManagerFee>
    <NeedMoney>需要金额</NeedMoney>
    <RealMoney>实际金额</RealMoney>
    <Status>状态</Status>
    <WalletMoney>钱包余额</WalletMoney>
    <WalletName>钱包名称</WalletName>
  </EasyTong>
  ```
- **相关文件**: `lib/core/easytong/repositories/easytong_repository.dart:184`

## 加密机制

### 1. AES 加密（应用服务 API）
- **密钥**: 在 `AppConfig.aesKey` 中配置
- **使用场景**: 所有发送到 `appservice.lzu.edu.cn` 的敏感数据
- **实现**: `lib/core/encryption/aes_crypto.dart`
- **拦截器**: `lib/core/api/appservice_client.dart:25` 中的 `EncryptInterceptor`
- **流程**:
  1. 请求时：如果 `Transfer-Encrypt: 'true'`，自动加密 JSON 数据
  2. 响应时：自动解密返回的数据

### 2. MD5 签名（EasyTong API）
- **密钥**: 在 `AppConfig.md5Key` 中配置
- **使用场景**: 所有 EasyTong 接口的请求签名
- **实现**: `lib/core/encryption/md5_crypto.dart`
- **签名格式**: `MD5(参数1 + 参数2 + ... + MD5密钥)`
- **参数顺序**: 按字母顺序排序的参数值拼接

## 错误处理

### 通用错误码
- `code: 1`: 成功
- `code: 0`: 失败
- `code: -1`: 系统错误
- `code: 401`: 未授权
- `code: 403`: 禁止访问
- `code: 404`: 资源不存在
- `code: 500`: 服务器内部错误

### 特定错误场景
1. **登录失败**: 用户名或密码错误、账户锁定等
2. **令牌过期**: `login_token` 或 `gateway_token` 失效
3. **网络超时**: 连接超时、响应超时
4. **数据解析错误**: 加密数据解密失败、JSON 解析错误

## 安全注意事项

1. **敏感数据加密**: 所有敏感数据（密码、令牌等）都经过 AES 加密传输
2. **令牌管理**: 登录令牌和网关令牌分开管理，存储在安全存储中
3. **请求签名**: EasyTong 接口使用 MD5 签名防止篡改
4. **超时设置**: 所有请求设置 3 秒超时，避免长时间等待
5. **错误日志**: 详细的错误日志记录，便于排查问题

## 相关文件索引

### API 配置
- `lib/app_config.dart`: API 端点配置、请求头配置
- `lib/core/api/appservice_client.dart`: 应用服务 API 客户端（带加密）
- `lib/core/api/app_client.dart`: 基础应用 API 客户端

### Repository 实现
- `lib/core/auth/repositories/auth_repository.dart`: 认证相关 API
- `lib/core/schedule/repositories/schedule_repository.dart`: 课表相关 API
- `lib/core/easytong/repositories/easytong_repository.dart`: 校园卡相关 API
- `lib/core/app/repositories/app_repository.dart`: 应用服务 API

### 数据模型
- `lib/core/auth/models/auth_model.dart`: 认证相关数据模型
- `lib/core/schedule/models/schedule_model.dart`: 课表相关数据模型
- `lib/core/easytong/models/easytong_model.dart`: 校园卡相关数据模型
- `lib/core/app/models/app_model.dart`: 应用服务数据模型

### 加密工具
- `lib/core/encryption/aes_crypto.dart`: AES 加密实现
- `lib/core/encryption/md5_crypto.dart`: MD5 签名实现

## 版本历史

- **v1.0**: 初始版本，包含基本认证、课表、校园卡功能
- **v1.1**: 添加加密传输、错误处理优化
- **v1.2**: 添加应用服务列表、界面优化

---

*文档最后更新: 2025-12-18*
*基于 FasterLZU 项目代码分析生成*