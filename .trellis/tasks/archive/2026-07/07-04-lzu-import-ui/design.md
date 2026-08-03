# 课程导入 UI 增加 LZU API 来源 - 设计

## UI Placement

优先在现有 `ImportModal` 中扩展导入来源，而不是创建新的顶级页面。建议结构：

```text
ImportModal
  - Clipboard tab
  - LZU API tab
```

如现有 modal 复杂度过高，可以拆分子组件：

```text
src/components/courses/import/
  ClipboardImportPanel.tsx
  LzuImportPanel.tsx
```

## State Shape

前端只保存 UI 必需状态：

- username input
- password input
- auth status summary
- loading flags
- import result
- display error

token、endpoint、headers、AES 不进入前端状态。

## Command Boundary

预期调用后端命令：

- `lzu_login`
- `lzu_logout`
- `lzu_get_auth_status`
- `import_lzu_courses`

命令返回稳定类型后，前端只消费 typed result。

## Interaction Flow

1. 用户打开导入弹窗。
2. 切换到 LZU API 来源。
3. 未登录时输入账号密码并登录。
4. 登录成功后点击导入。
5. 导入完成后展示统计并刷新课程列表。
6. 用户关闭弹窗或继续使用剪贴板导入。

## Accessibility and UX

- 登录按钮和导入按钮要有 disabled 状态。
- 表单字段需要 label。
- 错误靠近操作区域展示。
- 不使用大段说明文本塞满 modal。
- 移动端 modal 内布局不能溢出。

## Compatibility

不要改变现有课程导入 command 入参。LZU 导入作为新增命令和新增 UI 面板接入。
