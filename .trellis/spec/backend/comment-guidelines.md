# Comment Guidelines

> 最小注释规范：确保跨 session 的 AI 对关键结构体和函数有一致理解。

---

## Overview

当多个 AI session（不同模型、不同时间）维护同一代码库时，最容易产生歧义的是**跨模块复用的数据结构**——字段名本身往往不够精确，不同 AI 可能做出不同推断。

本规范定义**三种必须添加注释的场景**，追求极简：只覆盖"不写注释就会产生分歧"的情况。

核心原则：
- **DRY 注释**：注释说明"为什么"和"语义边界"，不重复代码已经表达的信息
- **YAGNI 注释**：只给跨模块复用或语义非显而易见的符号加注释，不给自解释的局部变量加注释
- **KISS 注释**：一句话说清楚，不用多行文档

---

## 规则一：跨模块复用的结构体 — 每个字段必须有 `//` 或 `///` 注释

### 触发条件

结构体同时满足以下条件时，每个字段必须有注释：
1. 定义在 `models.rs`、`mod.rs` 或公共模块中
2. 被两个或以上不同文件 `use` 引用
3. 字段名存在歧义空间（仅靠字段名无法唯一确定语义）

### 示例

✅ **正确**：

```rust
/// 单次同步的变更统计。merged 和 conflicts 的定义见各字段注释。
pub struct SyncStats {
    /// 本次同步成功写入本地数据库的实体数（新增 + 被远端覆盖的更新）。
    /// 不包含仅对齐 sync_id 而未修改数据的操作。
    pub tasks_merged: usize,
    /// courses 表合并数
    pub courses_merged: usize,
    /// exams 表合并数
    pub exams_merged: usize,
    /// pomodoro_sessions 表合并数
    pub sessions_merged: usize,
    /// 被拒绝的远端实体数。计算公式: sum(远端实体总数 - merged)。
    /// 含义：本地版本较新或时间戳相等，保留了本地数据。
    /// 这不是传统"编辑冲突"（两人同时编辑），而是"较旧的远端更新被忽略"。
    pub conflicts: usize,
}
```

❌ **错误**：

```rust
pub struct SyncStats {
    pub tasks_merged: usize,     // 是"合并了"还是"上传了"？
    pub conflicts: usize,        // 是"冲突数"还是"被拒绝数"？
}
```

### 豁免

以下情况可以不写注释：
- 字段名已经充分自解释且无歧义（如 `id: i64`、`title: String`）
- DTO/Request 结构体的字段已在对应的 Create/Update trait 文档中说明

---

## 规则二：非显而易见的纯函数 — 一句话说明核心逻辑

### 触发条件

函数同时满足以下条件时，需要一行注释：
1. 函数包含非平凡算法（不能一眼看出结果）
2. 函数名不完全揭示返回值语义

### 示例

✅ **正确**：

```rust
/// effective_timestamp: 有墓碑取 deleted_at，无墓碑取 updated_at。
/// 这确保删除操作的时间戳可以"赢过"普通的修改时间戳。
fn remote_effective_timestamp<'a>(
    updated_at: &'a str,
    deleted_at: Option<&'a str>,
) -> &'a str {
    deleted_at.unwrap_or(updated_at)
}
```

❌ **错误**：

```rust
// 无注释——函数名没提到"墓碑优先"这个关键设计决策
fn remote_effective_timestamp<'a>(
    updated_at: &'a str,
    deleted_at: Option<&'a str>,
) -> &'a str {
    deleted_at.unwrap_or(updated_at)
}
```

### 豁免

- CRUD 函数（`create_*`、`get_*`、`update_*`、`delete_*`）——命名已充分表达意图
- 标准 trait 实现（`Display`、`From`、`Serialize` 等）

---

## 规则三：协议常量/迁移版本号 — 说明取值原因或约束

### 触发条件

以下类型的常量必须注释：
1. 版本号（schema_version、migration version）
2. 重试次数、超时阈值
3. 格式标记（如 JSON 字段的 magic string）

### 示例

✅ **正确**：

```rust
/// v2 快照格式：引入 sync_id + 墓碑字段，支持跨设备合并和删除传播。
/// v1 兼容：旧格式无 schema_version 字段，默认值为 1，仍可导入。
const CURRENT_SCHEMA_VERSION: i64 = 2;

/// ETag 冲突重试次数。设为 1：一次重试足够覆盖"两个设备交替上传"的正常场景。
/// 超过一次说明存在持续竞争，应返回错误让用户稍后重试。
const ETAG_RETRY_MAX: u8 = 1;
```

❌ **错误**：

```rust
const CURRENT_SCHEMA_VERSION: i64 = 2;  // 为什么是 2？v1 是什么？
const ETAG_RETRY_MAX: u8 = 1;           // 为什么只重试一次？
```

### 豁免

- 数学/物理常量（`PI`、`SECONDS_IN_MINUTE` 等）——语义自明
- 数组长度/缓冲区大小——如果变量名已经说明用途

---

## 规则四：同步/通信协议的关键决策 — 在模块顶部的 `//!` 注释中说明

### 触发条件

模块实现了跨设备/跨进程的通信协议时，模块顶部必须有一行 `//!` 概述设计决策。

### 示例

✅ **正确**：

```rust
//! Sync exporter: v2 快照协议实现。
//!
//! 设计决策:
//! - 合并键: sync_id (v1 兼容回退到 SQLite id)
//! - 胜负判定: LWW (Last-Writer-Wins)，比较 effective_timestamp
//! - 墓碑: deleted_at 非空即已删除，正常查询过滤 WHERE deleted_at IS NULL
//! - ETag: 条件上传防止覆盖更新，冲突时重试一次
```

---

## 检查清单

提交代码前确认：

- [ ] 新增的跨模块结构体（2+ 文件引用）的每个字段有注释
- [ ] 非显而易见的算法函数有一句话注释
- [ ] 协议/版本常量有取值原因说明
- [ ] 新通信协议模块顶部有 `//!` 设计概述

---

## 与其它规范的关系

- 本规范是 [quality-guidelines.md](./quality-guidelines.md) 的补充，注释要求是代码质量标准的一部分
- 结构体契约定义请参考 [schedule-import-guidelines.md](./schedule-import-guidelines.md) 的契约格式
- 注释语言应与代码注释语言保持一致（本项目使用中文）

---

**核心原则**：不写注释的代价是下一个 AI session 的误解。三行注释可以节省三小时的调试。
