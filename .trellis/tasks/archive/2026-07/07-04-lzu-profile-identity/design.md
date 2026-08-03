# LZU 用户资料与登录身份展示 - 设计

## Boundary

后端负责调用 LZU AppService、解析原始响应、筛选低敏字段并脱敏。前端只消费身份摘要，不处理原始用户资料字段。

```text
React LZU import panel
  -> invoke lzu_get_profile_summary / lzu_get_auth_status
  -> Rust lzu::appservice user_info
  -> ProfileSummary DTO
```

## Data Contract

建议后端返回结构：

```text
LzuProfileSummary {
  display_name: Option<String>,
  person_no: Option<String>,
  department: Option<String>,
  role: Option<String>,
  campus_card_tail: Option<String>,
}
```

如果选择并入 `LzuAuthStatus`，字段应保持可选，避免未登录或资料接口失败时破坏现有 UI。

## Privacy

允许进入前端的字段必须是白名单。不要把 FasterLZU 的 `UserInfoData` 原样镜像到 TypeScript。

禁止字段：

- `sfzjh` 身份证号
- `yddh` 手机号
- `dzxx` 邮箱
- 完整 `xykh`
- 原始响应 JSON

## Error Handling

资料接口失败时返回业务错误或 `profile: null`。登录状态仍以 auth token 是否存在为准，不因资料失败自动 logout。

## UI Placement

身份摘要放在现有 LZU API 导入面板的 logged-in 状态区，作为账号确认信息，不新增独立页面。
