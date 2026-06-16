# Database Guidelines

> SQLite 数据库使用规范、迁移管理和查询构建模式

---

## Overview

Kairos 使用 **rusqlite** 作为 SQLite 驱动，采用原生 SQL 查询（不使用 ORM）。数据库连接在应用启动时初始化，通过 `Arc<Mutex<Connection>>` 在 Tauri 状态中共享。迁移系统基于版本号顺序执行，幂等且可回滚验证。

核心约定：
- **WAL 模式** 提升并发性能
- **外键约束** 强制引用完整性
- **参数化查询** 防止 SQL 注入
- **事务保护** 确保迁移原子性

---

## Database Connection

### 连接初始化

使用 `db::get_connection()` 创建连接，自动配置 WAL 模式、外键和迁移。

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

**应用启动时调用**：`src-tauri/src/lib.rs:31-35`

```rust
let db_path = app_data_dir.join("kairos.db");
let conn = db::get_connection(
    db_path.to_str().expect("invalid db path"),
)
.expect("failed to open database connection");
```

### 状态管理

连接在 `lib.rs:setup()` 中注册到 Tauri 状态：

```rust
let db_conn = Arc::new(Mutex::new(conn));
app.manage(db_conn);
```

命令层通过 `State<Arc<Mutex<Connection>>>` 访问：

```rust
#[tauri::command]
pub fn get_all_tasks(
    db: State<'_, Arc<Mutex<Connection>>>,
    // ...
) -> Result<Vec<Task>, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    // ...
}
```

---

## Query Patterns

### 1. 参数化查询（强制使用）

**✅ 正确**：使用 `params!` 宏绑定参数

`src-tauri/src/db/tasks.rs:6-19`

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

**❌ 禁止**：字符串拼接（SQL 注入风险）

```rust
// 永远不要这样做！
let sql = format!("SELECT * FROM tasks WHERE status = '{}'", status);
conn.query_row(&sql, [], |row| { /* ... */ })
```

### 2. 单行查询

使用 `query_row()` 获取单条记录，返回 `Result<T, Error>`（不存在时返回 `Error`）。

**示例**：`src-tauri/src/db/tasks.rs:22-41`

```rust
pub fn get_task(conn: &Connection, id: i64) -> Result<Task> {
    conn.query_row(
        "SELECT id, title, description, status, priority, due_date, tags, created_at, updated_at
         FROM tasks WHERE id = ?1",
        params![id],
        |row| {
            Ok(Task {
                id: row.get(0)?,
                title: row.get(1)?,
                description: row.get(2)?,
                status: row.get(3)?,
                priority: row.get(4)?,
                due_date: row.get(5)?,
                tags: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        },
    )
}
```

**要点**：
- 显式列出所有列名（不用 `SELECT *`）
- 使用位置索引 `row.get(0)?` 按顺序提取（避免按名称查找的开销）
- 在闭包内用 `?` 传播错误

### 3. 多行查询

使用 `prepare()` + `query_map()` 遍历结果集。

**示例**：`src-tauri/src/db/tasks.rs:78-95`

```rust
let mut stmt = conn.prepare(&sql)?;
let rows = stmt.query_map(param_refs.as_slice(), |row| {
    Ok(Task {
        id: row.get(0)?,
        title: row.get(1)?,
        description: row.get(2)?,
        status: row.get(3)?,
        priority: row.get(4)?,
        due_date: row.get(5)?,
        tags: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
})?;

rows.collect()
```

**动态 WHERE 子句**（带可选过滤器）：

`src-tauri/src/db/tasks.rs:63-76`

```rust
let mut sql = String::from(
    "SELECT id, title, description, status, priority, due_date, tags, created_at, updated_at FROM tasks WHERE 1=1",
);
let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

if let Some(status_val) = status_filter {
    sql.push_str(" AND status = ?");
    params_vec.push(Box::new(status_val.to_string()));
}
if let Some(priority_val) = priority_filter {
    params_vec.push(Box::new(priority_val.to_string()));
    sql.push_str(" AND priority = ?");
}

sql.push_str(&format!(" ORDER BY {} {}", sort_column, order));
```

**SQL 注入防护**：动态排序字段需白名单验证

`src-tauri/src/db/tasks.rs:50-55`

```rust
let allowed_sort_columns = ["title", "status", "priority", "due_date", "created_at", "updated_at"];
let sort_column = if allowed_sort_columns.contains(&sort_by) {
    sort_by
} else {
    "created_at"
};
```

### 4. 更新和删除

使用 `execute()` 执行非查询语句。

**示例**：`src-tauri/src/db/tasks.rs:98-115`

```rust
pub fn update_task(conn: &Connection, id: i64, req: &UpdateTaskRequest) -> Result<()> {
    conn.execute(
        "UPDATE tasks
         SET title = ?1, description = ?2, status = ?3, priority = ?4, due_date = ?5, tags = ?6, updated_at = ?7
         WHERE id = ?8",
        params![
            req.title,
            req.description,
            req.status,
            req.priority,
            req.due_date,
            req.tags,
            super::chrono_now(),
            id,
        ],
    )?;
    Ok(())
}
```

**注意**：`updated_at` 在更新时自动设置为当前时间。

---

## Migrations

### 迁移系统设计

**核心原则**：
- 版本号递增（从 1 开始）
- 每个迁移包含 `(version, name, sql)`
- 使用 `_migrations` 表追踪已应用的版本
- 事务保护：迁移 SQL + 版本记录一起提交或回滚

### 迁移入口

`src-tauri/src/db/migrations.rs:3-12`

```rust
pub fn run_migrations(conn: &Connection) -> Result<()> {
    // Create the migrations tracking table if it doesn't exist
    conn.execute(
        "CREATE TABLE IF NOT EXISTS _migrations (
            version INTEGER PRIMARY KEY,
            name    TEXT NOT NULL,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        [],
    )?;
    // ...
}
```

### 迁移定义

所有迁移定义在 `run_migrations()` 函数内：

`src-tauri/src/db/migrations.rs:14-90`

```rust
let migrations: Vec<(i32, &str, &str)> = vec![
    (
        1,
        "initial_schema",
        "
        CREATE TABLE IF NOT EXISTS pomodoro_config (
            id INTEGER PRIMARY KEY DEFAULT 1,
            work_seconds INTEGER NOT NULL DEFAULT 1500,
            short_break_seconds INTEGER NOT NULL DEFAULT 300,
            long_break_seconds INTEGER NOT NULL DEFAULT 900,
            sessions_before_long_break INTEGER NOT NULL DEFAULT 4
        );

        CREATE TABLE IF NOT EXISTS tasks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            description TEXT NOT NULL DEFAULT '',
            status TEXT NOT NULL DEFAULT 'todo' CHECK(status IN ('todo', 'in_progress', 'done')),
            priority TEXT NOT NULL DEFAULT 'medium' CHECK(priority IN ('high', 'medium', 'low')),
            due_date TEXT,
            tags TEXT NOT NULL DEFAULT '[]',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        -- 更多表...
        ",
    ),
    (
        2,
        "sync_config",
        "
        CREATE TABLE IF NOT EXISTS sync_config (
            id INTEGER PRIMARY KEY DEFAULT 1,
            server_url TEXT NOT NULL DEFAULT '',
            username TEXT NOT NULL DEFAULT '',
            password TEXT NOT NULL DEFAULT '',
            auto_sync INTEGER NOT NULL DEFAULT 0,
            last_sync_at TEXT
        );
        ",
    ),
];
```

### 迁移执行逻辑

`src-tauri/src/db/migrations.rs:92-112`

```rust
let current_version: i32 = conn
    .query_row(
        "SELECT COALESCE(MAX(version), 0) FROM _migrations",
        [],
        |row| row.get(0),
    )?;

for (version, name, sql) in migrations {
    if version > current_version {
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(sql)?;
        tx.execute(
            "INSERT INTO _migrations (version, name) VALUES (?1, ?2)",
            rusqlite::params![version, name],
        )?;
        tx.commit()?;
    }
}
```

**关键点**：
- 使用 `execute_batch()` 执行多语句 SQL
- 迁移和版本记录在同一事务中（失败时一起回滚）
- 已应用的迁移会被跳过（幂等性）

### 添加新迁移

```rust
// 在 migrations 向量末尾追加
(
    3,  // 版本号递增
    "add_user_preferences",
    "
    CREATE TABLE IF NOT EXISTS user_preferences (
        key TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );
    ",
),
```

---

## Naming Conventions

### 表名

- **蛇形命名法 (snake_case)**：`pomodoro_config`, `pomodoro_sessions`, `sync_config`
- **复数形式用于实体表**：`tasks`, `courses`, `exams`（表示多个记录）
- **单数形式用于配置表**：`pomodoro_config`, `sync_config`（单例模式）

### 列名

- **蛇形命名法**：`created_at`, `updated_at`, `day_of_week`
- **时间戳列**：统一使用 `xxx_at` 后缀（ISO 8601 TEXT 类型）
- **外键列**：`xxx_id`（如 `task_id`, `course_id`）

### 索引命名

（当前未使用显式索引，主键和外键自动索引）

---

## Schema Conventions

### 主键

```sql
id INTEGER PRIMARY KEY AUTOINCREMENT
```

- 所有实体表使用自增主键
- 配置表（单例）使用 `id INTEGER PRIMARY KEY DEFAULT 1`

### 时间戳

```sql
created_at TEXT NOT NULL,
updated_at TEXT NOT NULL
```

- 存储 ISO 8601 UTC 格式：`2024-01-15T08:30:00Z`
- 在 CRUD 函数中用 `super::chrono_now()` 生成

### CHECK 约束

```sql
status TEXT NOT NULL DEFAULT 'todo' CHECK(status IN ('todo', 'in_progress', 'done'))
```

- 枚举类型用 CHECK 约束实现
- 提供默认值保证数据一致性

### 外键

```sql
task_id INTEGER,
FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE SET NULL
```

- 必须在连接初始化时启用：`pragma_update(None, "foreign_keys", "ON")`
- 使用 `ON DELETE SET NULL` 或 `ON DELETE CASCADE` 明确级联行为

---

## Testing Patterns

### 测试数据库设置

`src-tauri/src/db/tasks.rs:128-134`

```rust
fn setup_db() -> Connection {
    let conn = Connection::open_in_memory().expect("Failed to open in-memory DB");
    conn.pragma_update(None, "foreign_keys", "ON")
        .expect("Failed to enable foreign keys");
    migrations::run_migrations(&conn).expect("Migrations failed");
    conn
}
```

**要点**：
- 使用 `open_in_memory()` 创建隔离环境
- 启用外键约束（与生产一致）
- 运行完整迁移（验证 schema 正确性）

### 测试用例结构

`src-tauri/src/db/tasks.rs:147-159`

```rust
#[test]
fn test_create_and_get_task() {
    let conn = setup_db();

    let req = sample_task("1");
    let id = create_task(&conn, &req).expect("Failed to create task");
    assert!(id > 0);

    let task = get_task(&conn, id).expect("Failed to get task");
    assert_eq!(task.title, "Test Task 1");
    assert_eq!(task.status, "todo");
    assert_eq!(task.priority, "medium");
}
```

**覆盖的测试类型**：
- 成功路径：创建、读取、更新、删除
- 错误路径：读取不存在的记录、更新不存在的记录
- 边界情况：空列表、过滤结果为空、排序顺序
- 约束验证：CHECK 约束拒绝非法值

### 迁移测试

`src-tauri/src/db/migrations.rs:118-144`

```rust
#[test]
fn test_migration_creates_tables() {
    let conn = Connection::open_in_memory().expect("Failed to open in-memory DB");
    conn.pragma_update(None, "foreign_keys", "ON")
        .expect("Failed to enable foreign keys");

    run_migrations(&conn).expect("Migrations failed");

    // Verify _migrations table exists and has version 1
    let version: i32 = conn
        .query_row("SELECT version FROM _migrations WHERE version = 1", [], |row| {
            row.get(0)
        })
        .expect("Migration record not found");
    assert_eq!(version, 1);

    // Verify all tables exist
    let table_count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name NOT LIKE '_migrations'",
            [],
            |row| row.get(0),
        )
        .expect("Failed to count tables");
    assert_eq!(table_count, 6);
}
```

**幂等性测试**：`src-tauri/src/db/migrations.rs:173-187`

```rust
#[test]
fn test_migration_idempotent() {
    let conn = Connection::open_in_memory().expect("Failed to open in-memory DB");
    conn.pragma_update(None, "foreign_keys", "ON")
        .expect("Failed to enable foreign keys");

    // Run migrations twice - second should be a no-op
    run_migrations(&conn).expect("First migration failed");
    run_migrations(&conn).expect("Second migration should be idempotent");

    // Should have exactly two migration records (v1 + v2) applied once each
    let count: i32 = conn
        .query_row("SELECT COUNT(*) FROM _migrations", [], |row| row.get(0))
        .expect("Failed to count migrations");
    assert_eq!(count, 2);
}
```

---

## Common Mistakes

### ❌ 忘记启用外键约束

```rust
// 错误：外键不生效
let conn = Connection::open(db_path)?;
migrations::run_migrations(&conn)?;
```

**正确**：必须显式启用

```rust
let conn = Connection::open(db_path)?;
conn.pragma_update(None, "foreign_keys", "ON")?;
migrations::run_migrations(&conn)?;
```

### ❌ 在迁移外修改 schema

**错误**：直接在应用代码中创建表

```rust
conn.execute("CREATE TABLE IF NOT EXISTS temp_data (...)", [])?;
```

**正确**：所有 schema 变更必须通过迁移系统

### ❌ 不使用事务保护迁移

**正确做法**：已在 `migrations.rs:101-108` 实现事务包裹

### ❌ 使用 `SELECT *`

**问题**：列顺序变化导致解析错误

**正确**：显式列出所有列名（见查询模式示例）

---

## Checklist

新增数据库功能时：

- [ ] 通过迁移系统添加表/字段（递增版本号）
- [ ] 在 `db/models.rs` 定义对应的 Rust 结构体
- [ ] CRUD 函数使用 `params!` 宏（禁止字符串拼接）
- [ ] 单行查询用 `query_row()`，多行用 `prepare()` + `query_map()`
- [ ] 时间戳字段调用 `super::chrono_now()` 生成
- [ ] 为每个函数编写至少 3 个测试用例
- [ ] 测试覆盖外键约束和 CHECK 约束
- [ ] 迁移测试验证表创建和幂等性
