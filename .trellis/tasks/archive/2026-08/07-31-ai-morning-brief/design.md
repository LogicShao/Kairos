# Design: AI 每日摘要生成器（AI Morning Brief）

> 本文档通过 4 维度并行设计 + 对抗评审（5 个子代理、33.7 万 token）收敛而来。评审发现各维度在每处共享契约上互相冲突，本设计按「一个契约只有一个赢家」逐项定稿。

## 1. Scope

本任务实现：

- `AIService` trait + `DeepseekProvider`（OpenAI 兼容非流式）+ `RuleBasedService`（本地降级）
- `ai_config` 单例表 + `ai_morning_brief` 按日缓存表（迁移 v11）
- API key AES-CBC 加密存储（本地随机 key 文件，零新依赖）
- 4 个 Tauri 命令（get/update config、get/generate brief）
- Today 页 `AiBriefCard` + `AiSettings` 设置页 + 轻量 Markdown 子集渲染器
- 7:00 定时触发 + 系统通知 + `ai-brief-generated` 事件

不实现：

- SSE 流式、任务建议、番茄反思、多 Provider、测试连接按钮、MCP

## 2. Architecture Decision

### 2.1 同步命令 + blocking reqwest（采纳 verify #9）

所有命令用同步 `#[tauri::command]` + `reqwest::blocking`。Tauri v2 命令在线程池上运行，阻塞 HTTP 不卡主线程，与现有 `sync/webdav.rs` 命令先例一致（Android 已验证）。

**窄锁纪律**：HTTP 期间**绝不持有** `Connection` 锁。流程 = 锁内快照今日数据 + 读配置 → drop 锁 → 发网络请求 → 重锁 double-check upsert。避免 15s 网络调用阻塞 pomodoro tick worker（每 1s 抢锁）。

### 2.2 一个模块、一套命令、一个输出结构（采纳 verify #3）

统一 `commands/ai.rs`：

| 命令 | 签名 | 说明 |
|------|------|------|
| `get_ai_config` | `(db) -> AiConfigView` | 含 `api_key_configured` 布尔，**永不解密回传** |
| `update_ai_config` | `(db, app_handle, req) -> AiConfigView` | 合并 Option 字段；成功后触发 scheduler reschedule |
| `get_ai_morning_brief` | `(db) -> Option<AiMorningBrief>` | 读今日缓存，无则 None |
| `generate_ai_morning_brief` | `(db, engine, app_handle, force) -> AiMorningBrief` | 生成并持久化；force=true 重生成 |

`AiConfigView` 与 `AiMorningBrief` 统一镜像到 `src/types/ai.ts`（type-safety.md 契约唯一来源）。

### 2.3 base_url 不含 /v1（采纳 verify #1）

`ai_config.base_url` 默认 `https://api.deepseek.com`（**不含** `/v1`）。代码统一 `format!("{}/v1/chat/completions", base_url.trim_end_matches('/'))`。迁移种子与 update 一律存不含 `/v1` 的 host，杜绝 `/v1/v1`。

### 2.4 加密：AES-128-CBC + 随机 IV + 本地 key 文件（采纳 verify #2）

新建 `src-tauri/src/ai/crypto.rs`，**不复用** `lzu/crypto.rs`（其 IV=Key 弱配置，注释明确禁止复用）。

- 密钥：16 随机字节写入 `app_data_dir/.ai_encryption_key`（首启 `ensure_key_file` 生成；熵源 `Uuid::new_v4().into_bytes()`，零新依赖；Unix `chmod 600`）。
- 加密：AES-128-CBC + **每次随机 16 字节 IV** + PKCS7 padding（`aes`/`cbc` 已是依赖）。
- 存储格式：`hex(iv) || ":" || hex(cipher)`。
- key 文件与 `kairos.db` 同处 app_data_dir → 防护目标是**远端/WebDAV/静态单文件泄露**，不防本地 root（威胁模型如实写入 spec）。

```rust
pub fn ensure_key_file(app_data_dir: &Path) -> Result<[u8; 16], String>
pub fn encrypt_api_key(app_data_dir: &Path, plaintext: &str) -> Result<String, String>
pub fn decrypt_api_key(app_data_dir: &Path, encrypted: &str) -> Result<String, String>
```

`update` 时 `api_key: Option<String>` 语义：`Some(非空)` = 加密写入新 key；`Some("")` = 清空；`None` = 保留原密文不重加密（随机 IV 下重加密纯属无意义 churn）。

### 2.5 按日缓存 + in-flight 护栏（采纳 verify #4）

`ai_morning_brief.date UNIQUE`（+08:00 中国日期）。`generate_today_brief(force)` 为 7:00 自动与手动**唯一入口**：

1. `force=false` 时先查今日缓存，命中直接返回（零成本）。
2. 抢 in-flight 护栏（模块级 `OnceLock<Arc<AtomicBool>>`，仿 `SyncGuard`）：抢不到则重查缓存，仍无则返回「生成中」错误。
3. 锁内快照数据+读配置 → 释放锁 → resolve_service → 生成 → 释放 in-flight。
4. 重锁 double-check：今日已有记录则丢弃本次结果，否则 upsert。

双保险杜绝「手动 + 7:00 并发双计费」与「重复打开 Today 重复计费」。

### 2.6 降级输出同构 Markdown（采纳 verify #6）

AI 版与规则版**共用同一 5 分节骨架**，仅 footer 区分来源：

```
# {date} {weekday_label} 晨间摘要
## 今日重点
## 课程
## 待办
## 考试
## 专注
---
*AI生成，请核实*   // AI 版
*本地生成*         // 规则版
```

- `source` 枚举 `BriefSource` serde `rename_all="lowercase"` → `"ai" | "rule"`，DB CHECK 与前端字面量三处同值（verify #7）。
- 结构校验：AI 响应必含 5 个节标题且以 `*AI生成，请核实*` 结尾，否则降级规则引擎，**不发起第二次 API 调用**。
- user 消息裁剪 `id` 字段减 token，**绝不中途截断 JSON**（会破坏事实）；`MAX_CONTEXT_CHARS=4000` 仅作兜底护栏。

### 2.7 单一模型名 + 单一价目表（采纳 verify #5）

`prompt.rs` 定义唯一 `MODEL_DEFAULT = "deepseek-chat"`（配置页可改）。成本预算按 deepseek-chat 官方价估算：system ~600 token + user ~450 token + 输出 max_tokens=400 ≈ $0.00045 < $0.05 美分。价格会漂移，实现时按当日官价复核预算表并写入 spec。

## 3. Data Contract

### 3.1 迁移 v11（`db/migrations.rs`）

```sql
CREATE TABLE IF NOT EXISTS ai_config (
    id INTEGER PRIMARY KEY DEFAULT 1,
    enabled INTEGER NOT NULL DEFAULT 0,
    base_url TEXT NOT NULL DEFAULT 'https://api.deepseek.com',
    model TEXT NOT NULL DEFAULT 'deepseek-chat',
    api_key_encrypted TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
INSERT OR IGNORE INTO ai_config
    (id, enabled, base_url, model, api_key_encrypted, created_at, updated_at)
    VALUES (1, 0, 'https://api.deepseek.com', 'deepseek-chat', '', datetime('now'), datetime('now'));

CREATE TABLE IF NOT EXISTS ai_morning_brief (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    date TEXT NOT NULL UNIQUE,                          -- YYYY-MM-DD（+08:00 中国日期）
    markdown TEXT NOT NULL,                             -- 5 分节 markdown 子集
    source TEXT NOT NULL DEFAULT 'ai' CHECK(source IN ('ai','rule')),
    model TEXT NOT NULL DEFAULT '',
    generated_at TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
```

**同步更新硬编码断言**：`test_migration_creates_tables` table_count 13→15；`test_migration_idempotent` / `test_migration_v3_tolerates_preexisting_columns` 记录数 10→11（共 4 处）。

**不同步证明**：`exporter.rs::SyncData` 白名单只含 5 类实体，`ai_*` 表不参与导出/导入，无需改同步代码。补回归断言：导出 JSON 不含 `"ai_config"` / `"ai_morning_brief"`。

### 3.2 Rust 模型（`db/models.rs`）

```rust
/// AI 摘要配置（单例，id=1）。api_key 仅存密文。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    pub id: i64,
    pub enabled: bool,
    /// OpenAI 兼容 base URL，不含 /v1（代码统一追加）。
    pub base_url: String,
    pub model: String,
    /// API key 密文（AES-CBC + 本地 key 文件）。空 = 未配置。
    pub api_key_encrypted: String,
    pub created_at: String,
    pub updated_at: String,
}

/// 前端可见配置视图。key 永不过桥。
#[derive(Debug, Clone, Serialize)]
pub struct AiConfigView {
    pub id: i64,
    pub enabled: bool,
    pub base_url: String,
    pub model: String,
    /// 是否已配置 key（不回传明文/掩码）。
    pub api_key_configured: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// update 合并请求。api_key: Some(非空)=更新；Some("")=清空；None=保留。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct UpdateAiConfigRequest {
    pub enabled: Option<bool>,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub api_key: Option<String>,
}

/// 今日摘要（date UNIQUE，+08:00 中国日期）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiMorningBrief {
    pub id: i64,
    pub date: String,
    pub markdown: String,
    /// "ai" | "rule"
    pub source: String,
    /// 仅 source="ai" 时有值，如 "deepseek-chat"。
    pub model: String,
    pub generated_at: String,
    pub created_at: String,
    pub updated_at: String,
}
```

### 3.3 TS 镜像（`src/types/ai.ts`）

```ts
/** 与 commands::ai::AiConfigView 对齐。api_key 永不过桥，只有布尔标记。 */
export interface AiConfig {
  id: number
  enabled: boolean
  /** base_url 不含 /v1，后端统一追加 /v1/chat/completions。 */
  base_url: string
  model: string
  api_key_configured: boolean
  created_at: string
  updated_at: string
}

export interface UpdateAiConfigRequest {
  enabled?: boolean
  base_url?: string
  model?: string
  /** 有值=更新密钥；空字符串=清空；省略=保留。 */
  api_key?: string
}

export type BriefSource = "ai" | "rule"

/** 与 commands::ai::AiMorningBrief 对齐。 */
export interface AiMorningBrief {
  id: number
  /** YYYY-MM-DD（+08:00 中国日期）。 */
  date: string
  markdown: string
  source: BriefSource
  /** 仅 source === "ai" 时有值。 */
  model: string
  generated_at: string
  created_at: string
  updated_at: string
}
```

## 4. Backend Design

### 4.1 模块布局

```
src-tauri/src/ai/
├── mod.rs          // AIService trait、BriefSource、AiError、resolve_service
├── deepseek.rs     // DeepseekProvider + OpenAI 兼容 DTO + reqwest 错误映射
├── rule.rs         // RuleBasedService → render_rule_briefing（同构 5 分节）
├── prompt.rs       // SYSTEM_PROMPT 常量、MAX_TOKENS、build_user_message（裁剪 id）
├── crypto.rs       // AES-CBC 加解密 + .ai_encryption_key 文件管理
├── scheduler.rs    // 7:00 定时（单 cancel token）
└── morning_brief.rs // generate_today_brief 编排（缓存→护栏→生成→double-check upsert）

src-tauri/src/db/ai.rs       // ai_config / ai_morning_brief CRUD
src-tauri/src/commands/ai.rs // 4 个 Tauri 命令
```

### 4.2 AIService trait 与分发

```rust
/// 摘要生成来源。DB CHECK、serde、前端三处同值。
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BriefSource { Ai, Rule }

pub trait AIService: Send + Sync {
    fn generate_daily_brief(
        &self,
        briefing: &TodayBriefingResponse,
    ) -> Result<AiBriefResult, AiError>;
}

/// 分发器：未启用或无 key → None（命令层走规则引擎）。
pub fn resolve_service(config: &AiConfig, key: &[u8; 16]) -> Option<Box<dyn AIService>> {
    if config.enabled && !config.api_key_encrypted.is_empty() {
        DeepseekProvider::new(config, key).ok()
    } else {
        None
    }
}
```

`AiError` 变体：`Timeout` / `Network(String)` / `Http { status, body }` / `Parse(String)`（**无** `NotConfigured`，该路径不可达——verify #13）。命令层将 401 映射「API key 无效」、429 映射「请求过于频繁」。

### 4.3 DeepseekProvider

- `reqwest::blocking::Client::builder().timeout(15s).build()`。
- `POST {base_url}/v1/chat/completions`，`Authorization: Bearer {key}`，body：`model / messages[system+user] / temperature=0.7 / max_tokens=400 / stream=false`。
- 解析 `choices[0].message.content`。

### 4.4 generate_today_brief 编排（`morning_brief.rs`）

```rust
pub fn generate_today_brief(
    db: &Arc<Mutex<Connection>>,
    engine: &Arc<Mutex<PomodoroEngine>>,
    app_handle: &AppHandle,
    force: bool,
) -> Result<AiMorningBrief, String>
```

关键步骤：

1. `today_china()` → +08:00 YYYY-MM-DD。
2. 若 `!force`：锁内查今日缓存，命中即返回。
3. 抢 in-flight：失败 → 重查缓存 → 仍无 → `Err("正在生成，请稍候")`。
4. 锁内：`collect_today_briefing(&conn, &eng)` 拿 `TodayBriefingResponse` + 读 `ai_config` + 读 key 文件明文 → **释放锁**。
5. `resolve_service`：Some → 调 AI；`Err(e)` → `log::warn!` 降级规则；None → 规则。AI 结果过结构校验，不过则降级规则。
6. 释放 in-flight。
7. 重锁 double-check upsert → 返回。
8. `emit("ai-brief-generated", &brief)` + 若来源 AI 调用方是调度器则 `show_system_notification`。

`get_today_briefing` 命令体抽取 `pub(crate) fn collect_today_briefing(conn: &Connection, engine: &PomodoroEngine) -> Result<TodayBriefingResponse, String>` 供复用（命令契约不变，briefing.rs 既有测试兜底）。

### 4.5 7:00 调度（`scheduler.rs`，仿 pomodoro_scheduler 单 token）

```rust
fn cancel_token() -> &'static Arc<Mutex<Option<Arc<AtomicBool>>>>
pub fn ensure_scheduled(db, engine, app_handle)  // 仅 enabled && key 时启动
pub fn reschedule(db, engine, app_handle) -> Result<(), String>  // cancel + ensure
fn next_seven_am_utc(now: DateTime<Utc>) -> DateTime<Utc>        // +08:00 下一 07:00
```

线程循环：`sleep(next_7am - now)` → token 检查 → `generate_today_brief(force=false)` → `show_system_notification("今日 AI 摘要已生成", 摘要前 60 字)` → 循环次日。`update_ai_config` 成功后调 `reschedule`。

### 4.6 lib.rs 接线

- 顶层 `pub mod ai;`（`commands`/`db` 之后）。
- setup：`create_dir_all` 后 `ai::crypto::ensure_key_file(&app_data_dir)`（失败仅 `log::warn`，不阻断启动）；`app.manage(Arc::new(key))` 或命令内自取 app_data_dir（两者取一，推荐命令内自取避免新 State）。考试通知块后：`if notifications_available { ai::scheduler::ensure_scheduled(&db_conn, &engine, app.handle()); }`。
- invoke_handler 追加 4 个命令。
- 通知 ID：`ids::stable_id("ai:morning-brief")`。

## 5. Frontend Design

### 5.1 AiBriefCard（`src/pages/today/AiBriefCard.tsx`）

挂载在 Today 页现有 briefing `AcrylicPanel` 之下同列。

- 状态机：`config-loading` → 若 `!enabled || !api_key_configured` → **渲染 null**（AI 可选，不打扰默认用户）；否则 `idle`（未生成 → 「生成今日摘要」按钮）/ `loading` / `ready`（Markdown + 来源徽标 + model + generated_at + 重新生成）/ `error`。
- 挂载时 `get_ai_config` + `get_ai_morning_brief`；按钮调 `generate_ai_morning_brief({force:false})`，重新生成 `{force:true}`。
- `listenWithCleanup<AiMorningBrief>("ai-brief-generated", ...)` 自动刷新。

### 5.2 MarkdownSubset（`src/lib/markdown-subset.tsx`）

~40 行子集渲染器，React 节点渲染（**无** `dangerouslySetInnerHTML`，天然防 XSS）：

- `# ` / `## ` 标题
- `**加粗**`
- `- ` 无序列表
- 空行分段；`---` 分隔线
- 未知行按 `<p>` 兜底（不报错）

零新依赖（package.json 无 markdown 库；react-markdown 留作后续扩展位）。

### 5.3 AiSettings（`src/components/settings/AiSettings.tsx`）

仿 `SyncSettings`/`NotificationSettings`：`save-on-leave`（latestDraftRef/lastPersistedRef）+ inputClass + api_key password 输入（已配置时 placeholder「已保存密钥，留空不变」）。

- enabled 开关、base_url、model、api_key。
- 说明文案：成本（单次约 <1000 token，每日最多一次）、隐私（密钥仅本机 AES 加密存储、不同步 WebDAV；课表/任务/考试数据会发往第三方 API，可在设置关闭）。

### 5.4 导航接线

- `App.tsx`：`{active === "ai-settings" && <AiSettings onNavigate={setActive} />}`。
- `AppShell.tsx`：`isKairosArea` 数组加 `'ai-settings'`。
- `KairosHub.tsx`：`HUB_ENTRIES` 加 `{ key: "ai-settings", label: "AI 设置", description: "配置每日摘要与 API Key", icon: Sparkles }`。

## 6. Tradeoffs

| 方案 | 决策 | 理由 |
|------|------|------|
| AES-GCM（新依赖）vs AES-CBC（复用） | AES-CBC | 零新依赖；威胁模型不要求认证加密（防护单文件泄露） |
| env key vs key 文件 | key 文件 | Android 无用户级 env；不与 KAIROS_LZU_AES_KEY 通道耦合 |
| react-markdown vs 子集渲染器 | 子集渲染器 | 零依赖、React 节点防 XSS；prompt 约束使子集封闭 |
| async + spawn_blocking vs 同步命令 | 同步命令 | 与 webdav 先例一致；Tauri 命令线程池吸收阻塞 |
| 迁移 TodayBriefingResponse vs 引用 | 引用不迁移 | 避免动稳定命令；仅抽 `collect_today_briefing` |
| 按日缓存 + in-flight vs 无状态每次生成 | 缓存 + 护栏 | 满足日均 <0.05 美分；防重复计费 |

## 7. Rollback

- AI 功能全部默认关闭：`enabled=0` 时不调度、Today 不渲染、命令走规则引擎，可在设置一键关闭。
- 若生成链路出问题，前端「重新生成」或清除今日缓存即恢复。
- 分支 `feat/ai-morning-brief` 独立，不合入 main 前不影响稳定版。
