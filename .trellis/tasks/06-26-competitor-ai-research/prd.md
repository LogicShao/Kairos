# 竞品AI架构调研 — Google One/钉钉/企业微信的Agent接入方式学习

## Goal

研究 Google One/Snapshot、钉钉、企业微信的先进架构和 AI/Agent 接入方式，提炼适合 Kairos（个人学业时间管理工具）采纳的具体方案。产出可落地的功能建议和实施路线图。

## Constraints

1. **个人开发优先** — 不学习太重功能，方便使用 Claude Code 及 Codex 完成开发
2. **功能明确有效** — 有切实际作用，不搞花架子
3. **可接入轻量 AI/Agent** — 基于 OpenAI 兼容协议，首选 deepseek-v4-flash（及后续 deepseek 系列），零本地部署
4. **必须跨平台** — Windows/Linux/Android（维持 Tauri v2 架构）
5. **合理拓展** — 基于以上四点延伸（如 MCP Server 生态、插件接口等）

## Branch Strategy

所有 AI/实验性功能在**独立 feature 分支**上开发，`main` 分支保持核心功能稳定可发布。

```
main ──────────────────────────────────────────── (稳定主线)
  │
  ├── feat/today-briefing      ← Phase 1: 今日概览卡片 (无AI)
  ├── feat/ai-morning-brief    ← Phase 2: AI每日摘要 + 任务建议
  ├── feat/ai-pomodoro-reflect ← Phase 2: 番茄钟AI反思
  ├── feat/ai-course-import    ← Phase 3: 课程智能导入
  └── feat/kairos-mcp          ← Phase 4: MCP Server + PluginCard trait
```

**规则：**
- `main` = 始终可构建可运行的稳定版本，不含实验性 AI 功能
- 每个 feature 分支独立可合入、可废弃，互不阻塞
- 分支命名以 `feat/` 前缀区分功能开发
- 合入 `main` 前需功能稳定、通过完整的跨平台验证
- 废弃的分支保留在远端但不合入，作为技术探索的记录

## Research Findings

### 三款产品的共同信号

| 产品 | 定位 | AI 接入方式 | 核心启示 |
|------|------|------------|---------|
| **Google Assistant Snapshot** | 主动式每日摘要 + 卡片流聚合 | Gemini 三层混合（Nano端侧 + Flash中速 + Pro深度） | "打开即看到今天要做什么"——零摩擦入口 |
| **钉钉 Agent OS (木兰)** | AI 原生协作 OS | 钉钉 ONE + MCP Host + Cool App 碎片化 + 多 Agent 仲裁 | "事找人"——主动推送优先，减少用户查询成本 |
| **企业微信 5.0** | 连接微信的协作平台 | 混元大模型 + 智能总结/搜索/机器人 + Webhook 极轻量机器人 | 反幻觉原则 + 极致简单 API + AI 嵌入工作流而非独立聊天页 |

### 三产品共同选择（验证过的模式）

1. **卡片流聚合引擎** — 统一 UI 表面渲染多种数据源，每张卡片按时间上下文排序
2. **AI 摘要生成** — 将结构化数据转为自然语言摘要
3. **薄客户端 / 厚服务端** — 客户端只负责渲染（对 Kairos：Rust 后端承担"服务端"角色）
4. **AI 是 ambient 的，不是 destination 的** — AI 嵌入现有界面，而非独立"AI助手"Tab

## Recommended Features

### Phase 1: 基石（2-3周，无 AI 依赖）

1. **今日概览卡片 (Today Briefing Card)** — [中投入/高影响/无AI]
   - KairosHub 首页聚合当日课程、待办到期、考试倒计时、番茄钟统计
   - 纯本地 Rust 规则引擎 + SQLite，<50ms 响应
   - 按时间上下文排序（早上优先课程和待办，下午优先统计和明日预习）

### Phase 2: AI 增强（3-4周，需 AI）

2. **AI 每日摘要生成器 (AI Morning Brief)** — [中投入/高影响/AI必需]
   - deepseek-v4-flash 读取本地日程生成自然语言摘要（SSE streaming + Markdown渲染）
   - 定时早7:00自动触发或手动一键生成，单次成本 <0.05 美分（deepseek 极低定价）
   - 走 OpenAI 兼容 `/v1/chat/completions` 端点，API key 存入 Tauri secure store
3. **智能任务优先级建议 (AI Task Advisor)** — [低投入/中影响/AI必需]
   - 基于截止日期+课程时间+历史专注数据排序，复用 AIService 基础设施
4. **番茄钟 AI 反思提示** — [低投入/中影响/AI必需]
   - 每次番茄 session 结束后微型反思引导 + 每周汇总报告
5. **课程数据智能导入** — [低投入/高影响/AI必需]
   - 粘贴教务课表文本 → deepseek-v4-flash 解析 → 确认后一键导入

### Phase 3: 生态（3-4周，部分需 AI）

6. **Kairos MCP Server** — [中投入/高影响/AI必需]
   - 暴露 create_task/query_calendar/get_today_summary 等 tools
   - Claude Code/Cursor 可直接读写用户日程
7. **轻量卡片插件接口 (PluginCard trait)** — [中投入/中影响/无AI]
   - Rust trait 定义卡片接口，内置4实现，未来开放社区贡献
8. **多模型 Provider 切换** — [低投入/低影响/AI可选]
   - 前端设置页支持切换 deepseek / openai 兼容 endpoint + 自定义 base_url
   - 离线时自动降级为 RuleBasedService（规则引擎）

## What to Avoid

| 避免模式 | 原因 | 替代方案 |
|---------|------|---------|
| 多 Agent 仲裁框架（钉钉 ONE） | Agent 通信协议+冲突解决，solo dev 不可承受 | 单个 well-crafted system prompt |
| 自研跨平台引擎（CEF/Flutter Dutter） | 需专业团队多年投入 | 保持 Tauri v2（已有） |
| 系统级 Smartspace/Widget 集成 | 需 SystemUI 级权限 | Tauri 本地通知 + NotificationCompat |
| 双塔神经网络/端侧 ML 训练 | 需海量数据+GPU | 10行 Rust 规则引擎 |
| AI 独立聊天 Tab | 增加用户操作摩擦 | AI 嵌入现有界面（首页卡/任务列表/导入框） |
| AI 幻觉编造日程数据 | 破坏用户信任，可能造成实际损失 | System Prompt 约束 + "AI生成"标注 + 关键操作二次确认 |
| 微服务/K8s/消息队列 | 与个人桌面 App 定位矛盾 | 单体 Tauri + 本地 SQLite + 直连 AI API |
| 本地部署大模型（Ollama/LM Studio） | 需下载 GB 级模型文件 + GPU 显存，个人开发负担过重 | deepseek-v4-flash 云端 API，OpenAI 兼容协议，零部署零维护 |

## Implementation Roadmap

| Phase | 分支 | 周期 | 可交付物 | AI |
|-------|------|------|---------|-----|
| 1: 基石 | `feat/today-briefing` | 2-3周 | 今日概览卡片 + 规则引擎 + KairosHub 首页改造 | 无 |
| 2a: AI摘要 | `feat/ai-morning-brief` | 2-3周 | AIService trait + deepseek-v4-flash 直连 + 每日摘要 + 任务建议 | 必需 |
| 2b: 番茄反思 | `feat/ai-pomodoro-reflect` | 1周 | 番茄 session 后 AI 反思引导 + 周报告 | 必需 |
| 3: 智能导入 | `feat/ai-course-import` | 1周 | AI 解析课表文本 + ImportModal 智能解析 tab | 必需 |
| 4: MCP生态 | `feat/kairos-mcp` | 2-3周 | Kairos MCP Server + PluginCard trait + Provider 切换设置 | 必需 |
| 5: 打磨 | 各分支合入后 | 持续 | 性能优化 + Android 通知适配 + 文档 | 可选 |

> **AI 服务设计原则**：全部使用 OpenAI 兼容协议（`/v1/chat/completions`），即一个 `reqwest` HTTP 调用即可对接 deepseek-v4-flash 或任意兼容 provider。无需 SDK、无需本地模型、无需额外依赖。

> **分支间关系**：`feat/ai-morning-brief` 依赖 `feat/today-briefing`（共享卡片渲染组件和 AIService 基础设施），其余分支相互独立，可并行开发。

## Acceptance Criteria

- [ ] 所有 AI 功能均为可选（设置中可关闭），离线时降级到规则引擎
- [ ] 反幻觉：AI 生成内容标注"AI生成，请核实"，创建/修改操作需用户二次确认
- [ ] API key 通过 Tauri secure store 加密存储，不同步到 WebDAV
- [ ] AI 调用成本对学生用户友好（deepseek-v4-flash，日均 <0.05 美分）
- [ ] 所有变更保持跨平台兼容（Windows/Linux/Android）
- [ ] `main` 分支始终可构建可运行，不含不稳定的实验性功能
- [ ] 每个 feature 分支独立可合入、可废弃，不阻塞其他分支或 `main`
- [ ] PRD 通过用户审核确认
