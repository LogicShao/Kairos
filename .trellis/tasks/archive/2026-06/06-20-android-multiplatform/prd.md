# 三平台适配：Android 初始化 + 响应式 UI 改造

## Goal

使 Kairos 同时支持 Windows、Linux、Android 三平台，一套代码库通过 CSS 响应式 + Tauri 跨平台抽象实现，不创建平台分支，不搞条件编译。

## Requirements

### 1. 开发环境搭建

- 安装 Android Studio（获取 Android SDK + NDK + 模拟器，内置 JDK 无需单独装 Java）
- 安装 Rust Android 编译目标：`aarch64-linux-android`、`armv7-linux-androideabi`
- 开发主力 Windows，Android USB/模拟器验证移动 UI，Linux 交 CI

### 2. Android 平台初始化

- 执行 `npx tauri android init` 生成 `src-tauri/gen/android/`
- 确认 `Cargo.toml` crate-type 已含 `staticlib` + `cdylib`（Android 必需，当前已配）
- `cargo tauri android dev` 在模拟器/真机上跑通空白构建

### 3. 响应式 UI 改造

- **AppShell**：≥768px 侧边栏，<768px 底部 Tab Bar
- **番茄钟**：移动端全屏卡片 + 触摸友好按钮（≥44x44dp）
- **待办列表**：移动端卡片列表 + 底部弹出表单
- **课程表**：移动端单日列 + 水平滑动切换日期
- **考试倒计时**：移动端卡片列表
- **同步设置**：移动端全屏表单

### 4. Makefile 补充

- `android-dev`、`android-build-debug`、`android-build`
- `check-all`：含 aarch64-linux-android target

## Technical Decisions

| 维度 | 决策 |
|------|------|
| 代码分支 | 单分支 `main` |
| UI 适配 | CSS 响应式优先（Tailwind `md:` 断点），JS 平台检测兜底 |
| Rust 后端 | 零平台耦合 |
| 导航 | 768px 断点：侧边栏 ↔ 底部 Tab Bar |

### 不做什么

- 不创建单独 mobile 入口（`#[cfg_attr]` 已够）
- 不做 iOS（初始版本）
- 不引入第三方动画库

### 风险

| 风险 | 应对 |
|------|------|
| `rusqlite` bundled NDK 交叉编译失败 | 降级为不用 `bundled`，用系统 SQLite |
| 番茄钟后台线程被杀 | 后续 `@tauri-apps/plugin-notification` |
| WebView CSS 兼容差异 | 只用标准 CSS |
| 首次 Android 构建极慢 | Gradle 依赖 + NDK，预留 20 分钟 |

## Acceptance Criteria

- [ ] `npx tauri android init` 成功，`gen/android/` 目录存在
- [ ] `cargo tauri android dev` 在模拟器上跑通空白启动
- [ ] AppShell 在 <768px 显示底部 Tab Bar，≥768px 显示侧边栏
- [ ] 5 个功能模块在移动端可用
- [ ] `make check-all` 通过
- [ ] 桌面端布局无回归

## Implementation Order

1. 环境搭建：Android Studio + Rust targets
2. Android 初始化：`tauri android init` + 空白构建验证
3. AppShell 响应式改造
4. 番茄钟移动端适配（第一个完整模块，验证模式）
5. 其余 4 个模块逐一适配
6. Makefile 补充
7. 全平台检查通过
