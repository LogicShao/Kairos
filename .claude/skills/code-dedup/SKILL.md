---
name: code-dedup
description: 在编写 Rust/TypeScript 代码时自动激活，防止引入重复代码，强制执行三次法则（Rule of Three）
---

# 代码去重规则 (Code Deduplication Rules)

## 触发条件

在生成或修改以下类型代码时自动激活：
- Rust 函数、trait、struct、enum
- TypeScript/React 组件、hooks、工具函数
- 数据库操作层（db 模块）、同步逻辑（sync 模块）

## 核心规则

### 规则 1：三次法则（Rule of Three）— 强制

| 次数 | 行为 |
|------|------|
| **第 1 次** | 直接实现，无需顾虑 |
| **第 2 次** | 可以复制，但**必须在代码旁添加 `// FIXME: dedup — 出现第 3 次时必须抽象`** |
| **第 3 次** | **禁止复制**。必须先创建抽象（trait / 宏 / 泛型函数），再基于抽象实现 |

### 规则 2：薄包装函数检查

如果函数体 ≤ 5 行且满足以下**任一**条件，不允许创建：
- 仅调用另一个函数 + 类型转换 / 参数重排
- 仅封装另一个函数 + 硬编码部分参数
- 与已有函数逻辑完全相同，仅类型不同

**处理方式**：在调用方内联，或使用泛型统一。

### 规则 3：实体组操作模式

当多个实体（如 Task/Course/Exam/PomodoroSession）共享同一 CRUD 操作模式时：

- **Rust**: 使用 `trait` 定义操作接口 + 泛型函数实现公共逻辑
- **TypeScript**: 使用泛型函数或高阶函数
- 例外：如果实体间字段差异过大（>50% 字段不同），允许分别实现，但需在注释中说明理由

### 规则 4：宏优于复制（Rust 特定）

当同一逻辑对多个类型重复时，优先使用 `macro_rules!`：

```rust
// ✅ 正确：用宏消除重复
macro_rules! normalized {
    ($name:ident, $ty:ty, $entity:expr) => {
        fn $name(item: &$ty) -> $ty {
            let mut n = item.clone();
            if n.sync_id.is_empty() {
                n.sync_id = format!("legacy-{}-{}", $entity, n.id);
            }
            n
        }
    };
}
normalized!(normalized_task, Task, "task");
normalized!(normalized_course, Course, "course");
```

### 规则 5：export / merge 模式识别

如果你正在写 `export_*` 或 `merge_*` 函数，并且这是第 2 个同模式的函数：
- **立即检查**已有实现的差异点（SQL 列、字段名、时间戳比较方式）
- 如果差异 ≤ 30%，使用 **trait 参数化**统一为一个实现

### 规则 6：提交前检查

每次修改 Rust 代码后，运行：

```bash
jscpd . --threshold 10 --reporters console --format rust
```

如果新增了 clone（阈值 10% 以上），**不允许提交**，先重构消除重复。

## 🛠 项目工具配置

### 快速检测命令

```bash
# 全项目检测（Rust + TS），阈值 10%
jscpd . --threshold 10 --reporters console --format rust,typescript

# 仅检测 Rust，AST 级别
cargo duplicated --threshold 3 --min-block-size 5

# 生成 HTML 报告
jscpd . -c .jscpd.json --reporters html
```

### 配置文件

- `.jscpd.json` — jscpd 配置（minTokens: 30, minLines: 5）
- `.dups.toml` — cargo-duplicated 配置（阈值 3 次）

## 输出要求

每次生成 Rust/TypeScript 代码后，自检并附加一行：

```
[dedup] ✅ 本次未引入重复代码
```

或：

```
[dedup] ⚠️ 建议：函数 `xxx` 与已有 `yyy` 结构相同，建议在下次修改时用 trait/宏统一（违反规则 1，当前为第 2 次出现）
```

## 已知重复热点（已记录，等待重构）

以下模块存在已知重复，修改这些文件时**不允许增加新的重复**：

| 文件 | 重复模式 | 严重度 |
|------|----------|--------|
| `src-tauri/src/sync/exporter.rs` | export_*/merge_*/normalized_* ×4 | 高 (11.4%) |
| `src-tauri/src/commands/courses.rs` ↔ `exams.rs` | 命令分发模式 ×2 | 中 |
| `src-tauri/src/db/courses.rs` ↔ `exams.rs` ↔ `tasks.rs` | CRUD 查询 ×3 | 中 |
| `src-tauri/src/db/migrations.rs` | 表创建 SQL ×3 | 低（测试数据） |
| `src-tauri/src/schedule.rs` | 课程/考试时间展开逻辑 | 高 |

> 这些已知重复是有意记录的技术债务，会在后续专项重构中处理。新代码不得复制这些模式。
