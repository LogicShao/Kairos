# LZU API 接入 - 设计

## Architecture

采用本地优先的分层接入方式：

```text
React UI
  -> Tauri commands
    -> lzu service facade
      -> appservice client / easytong client
        -> reqwest
    -> existing course import + SQLite
```

React 只负责触发登录、拉取和导入，不持有 LZU 协议细节。Rust 后端负责网络请求、加密、认证状态、数据归一化和数据库写入。

## Module Boundaries

### Parent Task

父任务只维护任务树、范围和最终集成验收，不直接落业务实现。

### LZU AppService

对应 `https://appservice.lzu.edu.cn`：

- 登录、登出、用户信息。
- 课表、学期信息。
- AES-CBC hex payload 加解密。
- `Authorization` 使用 gateway token。

### LZU App / EasyTong

对应 `http://app.lzu.edu.cn:8080`：

- `st` 换 `EtToken`。
- 一卡通余额、二维码、订单查询。
- MD5 签名。

MVP 只实现 AppService 课表链路，EasyTong 放后续任务。

## Data Flow

1. UI 提交账号密码到 Tauri command。
2. Rust LZU auth service 调用登录接口。
3. 后端保存运行期 token 状态，返回登录状态摘要。
4. UI 触发课表导入。
5. Rust schedule service 拉取学期信息和课表数据。
6. mapper 将 LZU 课程字段转成 Kairos `CreateCourseRequest`。
7. 复用现有课程导入去重逻辑写入 SQLite。
8. 前端刷新课程列表。

## Compatibility

- 保持现有剪贴板导入接口。
- 保持本地课程模型作为唯一渲染来源。
- 后端新增命令应保持小而稳定，便于未来替换 API 细节。

## Risk Areas

- AES key / MD5 key 的来源和存放方式。
- LZU 接口返回明文 JSON 或加密 hex 的混合情况。
- 课表节次 `jc` bitmask 到时间段的映射。
- 周次字符串和 Kairos `matches_week_pattern` 的兼容性。
- 网络超时和校园 API 不稳定导致的用户体验。

## Rollback Shape

每个子任务都应保持可单独回退：

- Auth client 回退不影响本地课程。
- Schedule import 回退不影响剪贴板导入。
- UI 回退后仍可使用原导入弹窗。
- EasyTong 未完成不影响课表 MVP。
