# Android 返回键返回上一页面（无上级页面时退出）

## 背景

Android 端按系统返回键会**立即退出应用**。根因：Tauri 内置 AppPlugin 的返回键逻辑——当 JS 侧没有注册 `back-button` 监听时，若 webview 无页面历史则直接 `finish()` 退出。Kairos 是 state 导航的 SPA（webview 无 history），因此返回键总是退出。

## 需求

1. 在子页面（课程表、考试、LZU 服务、通知设置、学期阶段、AI 设置、同步设置等）按返回键 → 返回进入该页面前的上一页面。
2. 在任意主页面（今天、专注、待办、日历、设置）按返回键 → 退出应用到桌面。
3. 桌面端行为保持不变（不受影响）。

## 非目标

- 不处理 webview 内部链接跳转的 history 后退（应用内无外链页面）。
- 不做"再按一次退出"的 toast 交互（需求明确为一次返回直接退出）。

## 页面层级

- **主页面**（5 个，底部 Tab）：`today` / `pomodoro` / `todo` / `calendar` / `kairos`——相互切换为平级替换，不进导航栈。
- **子页面**（从 KairosHub 等入口进入）：`courses` / `exams` / `lzu-services` / `notifications` / `semester-phases` / `ai-settings` / `sync`——进入时压栈，返回时出栈。

## 验收标准

- [ ] A1：Android 在 `courses` 页面按返回键 → 回到 `kairos`；连续返回最终回到主页面。
- [ ] A2：Android 在主页面（如 `today`）按返回键 → 应用退出到桌面。
- [ ] A3：Android 从子页面返回后再次按返回键 → 正确回到主页面而非退出。
- [ ] A4：主页面之间 Tab 切换后（如 `todo` → `today`）按返回键 → 退出（Tab 切换重置导航栈）。
- [ ] A5：桌面端（Windows）应用行为与改动前一致（无返回键相关副作用）。
- [ ] A6：`cargo test`、`npm run build` 全部通过。
