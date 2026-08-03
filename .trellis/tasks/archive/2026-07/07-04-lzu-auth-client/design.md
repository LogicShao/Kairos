# LZU 认证与 AppService Client - 设计

## Backend Shape

建议新增模块：

```text
src-tauri/src/lzu/
  mod.rs
  appservice.rs
  auth.rs
  crypto.rs
  error.rs
  models.rs
```

`commands/lzu.rs` 只暴露命令薄层，业务逻辑留在 `lzu` 模块。

## State

登录成功后后端维护运行期状态：

```text
LzuSession {
  username,
  login_token,
  gateway_token,
}
```

第一版可以只做运行期状态。后续如需记住登录状态，再评估系统安全存储，不在本任务中缓存密码。

## Client Contract

AppService client 提供：

- `post_encrypted_json`
- `get_json`
- `post_form_encrypted_or_raw`

不要通过修改全局 header 切换加密行为；每个请求显式声明是否加密。

## Error Model

建议错误分层：

- 网络错误
- 超时
- 加密失败
- 解密失败
- JSON 解析失败
- API 返回失败码
- 未登录

Tauri command 返回字符串或稳定错误对象时，用户文案应避免暴露内部敏感信息。

## Security Notes

- `Authorization` 输出日志时必须脱敏。
- 请求体中包含密码时不记录。
- token 不进入 WebDAV 同步。
- 前端只拿到 `is_logged_in`、`username`、可选 `display_name`。
