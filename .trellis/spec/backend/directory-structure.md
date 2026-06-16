# Directory Structure

> 后端代码的组织方式和模块布局规范

---

## Overview

Kairos 后端采用 Rust + Tauri v2 架构，代码位于 `src-tauri/src/` 目录。采用按功能分层的模块组织方式：**命令层 (commands)** 处理前端 IPC 调用，**数据库层 (db)** 负责数据持久化，**业务逻辑层** 封装核心功能（如 `timer.rs`、`sync/`）。

所有模块通过 `mod.rs` 导出公共接口，内部实现细节保持私有。数据模型统一定义在 `db/models.rs`，供所有层共享。

---

## Directory Layout

```
src-tauri/src/
├── lib.rs              # 应用入口，Tauri Builder 配置，状态管理初始化
├── main.rs             # 二进制入口点（调用 lib.rs）
├── commands/           # Tauri 命令层（IPC 接口）
│   ├── mod.rs          # 导出所有命令模块
│   ├── tasks.rs        # 任务管理命令
│   ├── pomodoro.rs     # 番茄钟命令
│   ├── courses.rs      # 课程表命令
│   ├── exams.rs        # 考试计划命令
│   └── sync.rs         # 同步命令
├── db/                 # 数据库层（CRUD 操作）
│   ├── mod.rs          # 数据库连接、迁移入口、工具函数
│   ├── models.rs       # 所有数据模型（Domain Model + Request DTO）
│   ├── migrations.rs   # 版本化数据库迁移
│   ├── tasks.rs        # 任务表 CRUD
│   ├── pomodoro.rs     # 番茄钟配置/会话 CRUD
│   ├── courses.rs      # 课程表 CRUD
│   ├── exams.rs        # 考试 CRUD
│   └── sync.rs         # 同步配置 CRUD
├── sync/               # 同步业务逻辑
│   ├── mod.rs          # 导出同步模块
│   ├── exporter.rs     # 数据导入/导出逻辑
│   └── webdav.rs       # WebDAV 客户端实现
└── timer.rs            # 番茄钟引擎（状态机 + 计时逻辑）
```

---

## Module Organization

### 1. 命令层 (`commands/`)

**职责**：
- 使用 `#[tauri::command]` 宏定义前端可调用的函数
- 通过 `State<Arc<Mutex<Connection>>>` 访问数据库连接
- 将数据库层的 `Result<T>` 转换为 `Result<T, String>`（Tauri 要求）
- 处理前端传入的可选参数，提供默认值

**示例**：`src-tauri/src/commands/tasks.rs:52-68`

```rust
#[tauri::command]
pub fn get_all_tasks(
    db: State<'_, Arc<Mutex<Connection>>>,
    filters: TaskFilterParams,
) -> Result<Vec<Task>, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let sort_by = filters.sort_by.as_deref().unwrap_or("created_at");
    let sort_order = filters.sort_order.as_deref().unwrap_or("DESC");
    crate::db::tasks::get_all_tasks(
        &conn,
        filters.status_filter.as_deref(),
        filters.priority_filter.as_deref(),
        sort_by,
        sort_order,
    )
    .map_err(|e| e.to_string())
}
```

**要点**：
- 命令参数用自定义 `Cmd` 或 `Params` 结构体封装（用 `#[serde(default)]` 支持可选字段）
- 锁获取失败立即转为 String 错误
- 调用 `db::` 层函数执行实际逻辑
- 所有错误用 `.map_err(|e| e.to_string())` 转换

### 2. 数据库层 (`db/`)

**职责**：
- 提供纯函数式 CRUD 接口，接受 `&Connection` 参数
- 返回 `rusqlite::Result<T>`，不处理错误转换
- 使用 `params!` 宏构建参数化查询（防止 SQL 注入）
- 包含完整的单元测试（使用内存数据库）

**示例**：`src-tauri/src/db/tasks.rs:5-20`

```rust
pub fn create_task(conn: &Connection, req: &CreateTaskRequest) -> Result<i64> {
    conn.execute(
        "INSERT INTO tasks (title, description, status, priority, due_date, tags, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
        params![
            req.title,
            req.description,
            req.status,
            req.priority,
            req.due_date,
            req.tags,
            super::chrono_now(),
        ],
    )?;
    Ok(conn.last_insert_rowid())
}
```

**模块入口** (`db/mod.rs`)：
- `get_connection(db_path)` - 打开连接，启用 WAL 模式和外键约束，运行迁移
- `chrono_now()` - 生成 ISO 8601 UTC 时间戳（供所有 CRUD 函数复用）

**示例**：`src-tauri/src/db/mod.rs:12-18`

```rust
pub fn get_connection(db_path: &str) -> Result<Connection> {
    let conn = Connection::open(db_path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrations::run_migrations(&conn)?;
    Ok(conn)
}
```

### 3. 模型层 (`db/models.rs`)

**职责**：
- 定义所有数据结构（Domain Model 和 Request DTO）
- 所有字段使用 `pub` 公开（便于跨层访问）
- 统一使用 `serde::{Serialize, Deserialize}`

**示例**：`src-tauri/src/db/models.rs:37-47`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub status: String,
    pub priority: String,
    pub due_date: Option<String>,
    pub tags: String,
    pub created_at: String,
    pub updated_at: String,
}
```

**约定**：
- Domain Model 包含所有数据库字段（含 `id`、`created_at`、`updated_at`）
- `CreateXxxRequest` 不含自动生成字段
- `UpdateXxxRequest` 包含所有可更新字段（命令层负责合并现有值）

### 4. 业务逻辑层

**示例 1：定时器** (`timer.rs`)：
- 独立的状态机实现，不依赖数据库
- 提供 `tick()`、`start()`、`pause()`、`reset()` 等方法
- 由 `lib.rs` 在后台线程驱动

**示例 2：同步模块** (`sync/`)：
- `exporter.rs` - 导出/导入所有表的快照，实现 Last-Write-Wins 合并
- `webdav.rs` - 封装 HTTP 客户端，处理认证和错误映射

---

## Naming Conventions

### 文件命名

- **蛇形命名法 (snake_case)**：所有 `.rs` 文件用小写 + 下划线，如 `tasks.rs`、`webdav.rs`
- **模块入口**：每个目录包含 `mod.rs`，导出子模块

### 模块结构

- **按功能垂直切分**：每个功能（tasks、courses、exams）在 `commands/`、`db/` 各有一个同名文件
- **不使用嵌套模块**：所有功能模块平铺在各层级目录中

### 函数命名

- CRUD 操作：`create_task`、`get_task`、`get_all_tasks`、`update_task`、`delete_task`
- Tauri 命令：与数据库函数同名（但参数类型不同）

---

## Examples

### 标准功能的文件组织（以 `tasks` 为例）

```
src-tauri/src/
├── commands/tasks.rs       # get_all_tasks(), create_task(), update_task(), delete_task() 命令
├── db/tasks.rs             # 对应的数据库层实现
└── db/models.rs            # Task, CreateTaskRequest, UpdateTaskRequest 定义
```

### 跨层调用链（创建任务）

```
前端 invoke("create_task", {title: "买菜"})
  ↓
commands/tasks.rs:create_task()
  - 解构 CreateTaskCmd，提供默认值
  - 构造 CreateTaskRequest
  ↓
db/tasks.rs:create_task(&conn, &req)
  - 执行 INSERT 语句
  - 返回 last_insert_rowid
  ↓
返回 Result<i64, String> 给前端
```

### 独立业务模块（不走 CRUD）

```
src-tauri/src/timer.rs      # PomodoroEngine 状态机
src-tauri/src/lib.rs:46     # 后台线程每秒调用 engine.tick()
```

**不创建** `commands/timer.rs`，因为定时器状态通过事件推送 (`emit`) 而非命令拉取。

---

## Common Mistakes

### ❌ 在命令层执行数据库操作

```rust
// 错误：绕过数据库层，直接执行 SQL
#[tauri::command]
pub fn get_task(db: State<Arc<Mutex<Connection>>>, id: i64) -> Result<Task, String> {
    let conn = db.lock().unwrap();
    conn.query_row("SELECT * FROM tasks WHERE id = ?1", params![id], |row| {
        // ...
    }).map_err(|e| e.to_string())
}
```

**正确做法**：调用 `crate::db::tasks::get_task(&conn, id)`

### ❌ 在 `db/` 层转换错误为 String

```rust
// 错误：数据库层不应处理 Tauri 的错误格式要求
pub fn create_task(conn: &Connection, req: &CreateTaskRequest) -> Result<i64, String> {
    // ...
}
```

**正确做法**：返回 `rusqlite::Result<i64>`，由命令层转换

### ❌ 模型定义分散在多个文件

**正确做法**：所有 model 集中在 `db/models.rs`，保持单一真相来源

---

## Checklist

新增功能时，检查是否遵循以下结构：

- [ ] 在 `db/models.rs` 定义 Domain Model 和 Request DTO
- [ ] 在 `db/` 创建对应的 CRUD 函数文件
- [ ] 在 `commands/` 创建 Tauri 命令文件
- [ ] 在 `lib.rs` 的 `invoke_handler!` 中注册命令
- [ ] 每个数据库函数包含至少 3 个单元测试（成功、失败、边界情况）
- [ ] 命令层将所有错误转为 String
- [ ] 所有 SQL 使用 `params!` 宏（禁止字符串拼接）
