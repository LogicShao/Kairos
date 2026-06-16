# Quality Guidelines

> 代码质量标准、测试要求和代码审查规范

---

## Overview

Kairos 后端采用测试驱动的质量保证策略。每个数据库 CRUD 函数必须包含完整的单元测试，覆盖成功路径、错误路径和边界情况。项目使用 Rust 的内置测试框架 (`#[test]`) 和 `#[cfg(test)]` 模块隔离测试代码。

核心质量标准：
- **100% CRUD 测试覆盖**：所有数据库函数必须有对应测试
- **内存数据库测试**：使用 `Connection::open_in_memory()` 隔离测试环境
- **迁移幂等性验证**：迁移必须可重复执行
- **无 `unwrap` / `expect` 在命令层**：所有错误转为 `Result`

---

## Testing Requirements

### 1. 数据库层测试（强制要求）

每个 CRUD 函数必须包含至少以下测试：

#### 成功路径测试

**示例**：`src-tauri/src/db/tasks.rs:147-159`

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

#### 错误路径测试

**示例**：`src-tauri/src/db/tasks.rs:162-166`

```rust
#[test]
fn test_get_nonexistent_task() {
    let conn = setup_db();
    let result = get_task(&conn, 999);
    assert!(result.is_err());
}
```

#### 更新操作测试

**示例**：`src-tauri/src/db/tasks.rs:169-190`

```rust
#[test]
fn test_update_task() {
    let conn = setup_db();

    let req = sample_task("original");
    let id = create_task(&conn, &req).expect("Failed to create task");

    let update = UpdateTaskRequest {
        title: String::from("Updated Title"),
        description: String::from("Updated description"),
        status: String::from("in_progress"),
        priority: String::from("high"),
        due_date: Some(String::from("2024-12-31")),
        tags: String::from("[\"urgent\"]"),
    };
    update_task(&conn, id, &update).expect("Failed to update task");

    let task = get_task(&conn, id).expect("Failed to get updated task");
    assert_eq!(task.title, "Updated Title");
    assert_eq!(task.status, "in_progress");
    assert_eq!(task.priority, "high");
    assert_eq!(task.due_date.as_deref(), Some("2024-12-31"));
}
```

#### 删除操作测试

**示例**：`src-tauri/src/db/tasks.rs:193-203`

```rust
#[test]
fn test_delete_task() {
    let conn = setup_db();

    let req = sample_task("del");
    let id = create_task(&conn, &req).expect("Failed to create task");

    delete_task(&conn, id).expect("Failed to delete task");

    let result = get_task(&conn, id);
    assert!(result.is_err());
}
```

#### 查询过滤测试

**示例**：`src-tauri/src/db/tasks.rs:206-251`

```rust
#[test]
fn test_get_all_tasks_with_filters() {
    let conn = setup_db();

    let t1 = CreateTaskRequest {
        title: String::from("High priority task"),
        description: String::new(),
        status: String::from("todo"),
        priority: String::from("high"),
        due_date: None,
        tags: String::from("[]"),
    };
    let t2 = CreateTaskRequest {
        title: String::from("Low priority task"),
        description: String::new(),
        status: String::from("todo"),
        priority: String::from("low"),
        due_date: None,
        tags: String::from("[]"),
    };
    let t3 = CreateTaskRequest {
        title: String::from("Done task"),
        description: String::new(),
        status: String::from("done"),
        priority: String::from("medium"),
        due_date: None,
        tags: String::from("[]"),
    };

    create_task(&conn, &t1).expect("Failed to create t1");
    create_task(&conn, &t2).expect("Failed to create t2");
    create_task(&conn, &t3).expect("Failed to create t3");

    let all = get_all_tasks(&conn, None, None, "created_at", "DESC")
        .expect("Failed to get all tasks");
    assert_eq!(all.len(), 3);

    let high_priority = get_all_tasks(&conn, None, Some("high"), "created_at", "DESC")
        .expect("Failed to filter by priority");
    assert_eq!(high_priority.len(), 1);
    assert_eq!(high_priority[0].title, "High priority task");

    let done_tasks = get_all_tasks(&conn, Some("done"), None, "created_at", "DESC")
        .expect("Failed to filter by status");
    assert_eq!(done_tasks.len(), 1);
    assert_eq!(done_tasks[0].title, "Done task");
}
```

#### 边界情况测试

**示例**：`src-tauri/src/db/tasks.rs:254-259`

```rust
#[test]
fn test_get_all_tasks_empty() {
    let conn = setup_db();
    let tasks = get_all_tasks(&conn, None, None, "created_at", "DESC")
        .expect("Failed to get tasks");
    assert!(tasks.is_empty());
}
```

#### 排序测试

**示例**：`src-tauri/src/db/tasks.rs:262-293`

```rust
#[test]
fn test_get_all_tasks_sorting() {
    let conn = setup_db();

    let t1 = CreateTaskRequest {
        title: String::from("A"),
        description: String::new(),
        status: String::from("todo"),
        priority: String::from("medium"),
        due_date: None,
        tags: String::from("[]"),
    };
    let t2 = CreateTaskRequest {
        title: String::from("Z"),
        description: String::new(),
        status: String::from("todo"),
        priority: String::from("medium"),
        due_date: None,
        tags: String::from("[]"),
    };
    create_task(&conn, &t1).expect("Failed to create t1");
    create_task(&conn, &t2).expect("Failed to create t2");

    let asc = get_all_tasks(&conn, None, None, "title", "ASC")
        .expect("Failed to sort ASC");
    assert_eq!(asc[0].title, "A");
    assert_eq!(asc[1].title, "Z");

    let desc = get_all_tasks(&conn, None, None, "title", "DESC")
        .expect("Failed to sort DESC");
    assert_eq!(desc[0].title, "Z");
    assert_eq!(desc[1].title, "A");
}
```

### 2. 测试辅助函数

使用辅助函数减少重复代码。

**数据库设置**：`src-tauri/src/db/tasks.rs:128-134`

```rust
fn setup_db() -> Connection {
    let conn = Connection::open_in_memory().expect("Failed to open in-memory DB");
    conn.pragma_update(None, "foreign_keys", "ON")
        .expect("Failed to enable foreign keys");
    migrations::run_migrations(&conn).expect("Migrations failed");
    conn
}
```

**测试数据生成**：`src-tauri/src/db/tasks.rs:136-145`

```rust
fn sample_task(extra: &str) -> CreateTaskRequest {
    CreateTaskRequest {
        title: format!("Test Task {}", extra),
        description: String::from("A test task"),
        status: String::from("todo"),
        priority: String::from("medium"),
        due_date: None,
        tags: String::from("[]"),
    }
}
```

### 3. 迁移测试（强制要求）

迁移系统必须包含以下测试：

#### Schema 创建验证

**示例**：`src-tauri/src/db/migrations.rs:118-144`

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

#### 幂等性测试

**示例**：`src-tauri/src/db/migrations.rs:173-187`

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

#### 约束验证测试

**示例**：`src-tauri/src/db/migrations.rs:155-169`

```rust
// Verify CHECK constraints exist by inserting valid/invalid data
conn.execute(
    "INSERT INTO tasks (title, description, status, priority, tags, created_at, updated_at)
     VALUES ('test', '', 'todo', 'medium', '[]', '2024-01-01', '2024-01-01')",
    [],
)
.expect("Valid task insert failed");

// Invalid status should fail
let result = conn.execute(
    "INSERT INTO tasks (title, description, status, priority, tags, created_at, updated_at)
     VALUES ('test2', '', 'invalid_status', 'medium', '[]', '2024-01-01', '2024-01-01')",
    [],
);
assert!(result.is_err(), "Invalid status should be rejected");
```

### 4. 业务逻辑测试

对于不涉及数据库的纯逻辑（如工具函数），提供独立的单元测试。

**示例**：`src-tauri/src/sync/webdav.rs:206-212`

```rust
#[test]
fn test_base64_encode() {
    assert_eq!(base64_encode(""), "");
    assert_eq!(base64_encode("f"), "Zg==");
    assert_eq!(base64_encode("fo"), "Zm8=");
    assert_eq!(base64_encode("foo"), "Zm9v");
    assert_eq!(base64_encode("foo:bar"), "Zm9vOmJhcg==");
}
```

**URL 处理测试**：`src-tauri/src/sync/webdav.rs:172-203`

```rust
#[test]
fn test_sync_file_url_strips_trailing_slash() {
    let client = WebDavClient {
        server_url: "https://webdav.example.com/".to_string(),
        username: String::new(),
        password: String::new(),
        client: Client::builder()
            .timeout(Duration::from_secs(1))
            .build()
            .expect("Failed to create client"),
    };
    assert_eq!(
        client.sync_file_url(),
        "https://webdav.example.com/kairos-sync.json"
    );
}

#[test]
fn test_sync_file_url_no_trailing_slash() {
    let client = WebDavClient {
        server_url: "https://webdav.example.com/dav".to_string(),
        username: String::new(),
        password: String::new(),
        client: Client::builder()
            .timeout(Duration::from_secs(1))
            .build()
            .expect("Failed to create client"),
    };
    assert_eq!(
        client.sync_file_url(),
        "https://webdav.example.com/dav/kairos-sync.json"
    );
}
```

---

## Forbidden Patterns

### ❌ 在命令层使用 `unwrap()` / `expect()`

```rust
// 错误：锁失败导致 panic
#[tauri::command]
pub fn get_tasks(db: State<Arc<Mutex<Connection>>>) -> Result<Vec<Task>, String> {
    let conn = db.lock().unwrap();  // 不要这样做！
    // ...
}
```

**正确**：使用 `?` 传播错误

```rust
#[tauri::command]
pub fn get_tasks(db: State<Arc<Mutex<Connection>>>) -> Result<Vec<Task>, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    // ...
}
```

**例外**：应用启动时的初始化代码可以使用 `expect()`（参见 `src-tauri/src/lib.rs:27-35`）

### ❌ SQL 字符串拼接

```rust
// 错误：SQL 注入风险
let sql = format!("SELECT * FROM tasks WHERE status = '{}'", status);
conn.query_row(&sql, [], |row| { /* ... */ })
```

**正确**：使用参数化查询（参见 `database-guidelines.md`）

### ❌ 没有测试的数据库函数

```rust
// 错误：创建函数后没有配套测试
pub fn get_task_count(conn: &Connection) -> Result<i64> {
    // ...
}

// 缺少：
// #[test]
// fn test_get_task_count() { /* ... */ }
```

### ❌ 在生产代码中使用 `println!`

```rust
// 错误：不受控制的输出
pub fn create_task(conn: &Connection, req: &CreateTaskRequest) -> Result<i64> {
    println!("Creating task: {}", req.title);  // 使用 log::info!
    // ...
}
```

### ❌ 忽略 `Result` 返回值

```rust
// 错误：静默失败
conn.execute("DELETE FROM tasks WHERE id = ?1", params![id]);

// 正确：传播错误
conn.execute("DELETE FROM tasks WHERE id = ?1", params![id])?;
```

---

## Required Patterns

### ✅ 所有错误使用 `Result` 返回

```rust
// 数据库层
pub fn create_task(conn: &Connection, req: &CreateTaskRequest) -> Result<i64> { /* ... */ }

// 命令层
#[tauri::command]
pub fn create_task(db: State<Arc<Mutex<Connection>>>, cmd: CreateTaskCmd) -> Result<i64, String> { /* ... */ }
```

### ✅ 命令层转换错误为 String

```rust
crate::db::tasks::get_task(&conn, id).map_err(|e| e.to_string())
```

### ✅ 使用 `#[cfg(test)]` 隔离测试代码

`src-tauri/src/db/tasks.rs:122-294`

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations;
    use rusqlite::Connection;

    fn setup_db() -> Connection { /* ... */ }

    #[test]
    fn test_create_and_get_task() { /* ... */ }

    #[test]
    fn test_get_nonexistent_task() { /* ... */ }

    // 更多测试...
}
```

### ✅ 测试使用内存数据库

```rust
let conn = Connection::open_in_memory().expect("Failed to open in-memory DB");
```

### ✅ 测试启用外键约束

```rust
conn.pragma_update(None, "foreign_keys", "ON")
    .expect("Failed to enable foreign keys");
```

### ✅ 参数化查询使用 `params!` 宏

```rust
conn.execute(
    "INSERT INTO tasks (title, status) VALUES (?1, ?2)",
    params![req.title, req.status],
)
```

---

## Code Review Checklist

提交代码前，确保满足以下要求：

### 功能实现

- [ ] 所有 Tauri 命令已在 `lib.rs` 的 `invoke_handler!` 中注册
- [ ] 数据库函数返回 `rusqlite::Result<T>`
- [ ] 命令函数返回 `Result<T, String>`
- [ ] 所有 SQL 使用参数化查询（`params!` 宏）
- [ ] 动态排序字段使用白名单验证
- [ ] 时间戳字段调用 `db::chrono_now()` 生成

### 错误处理

- [ ] 命令层不使用 `unwrap()` / `expect()`（除应用启动代码）
- [ ] 锁获取失败转为 String：`db.lock().map_err(|e| e.to_string())?`
- [ ] 数据库错误转为 String：`.map_err(|e| e.to_string())`
- [ ] 网络错误包含操作上下文（上传/下载/连接测试）

### 测试覆盖

- [ ] 每个 CRUD 函数有对应的测试模块
- [ ] 测试覆盖成功路径（创建 + 读取 + 验证）
- [ ] 测试覆盖错误路径（不存在的 ID、约束违规）
- [ ] 测试覆盖边界情况（空结果、排序、过滤）
- [ ] 迁移包含幂等性测试
- [ ] 迁移包含约束验证测试

### 代码风格

- [ ] 模块使用 `pub mod xxx;` 导出
- [ ] 测试代码在 `#[cfg(test)] mod tests { /* ... */ }` 中
- [ ] 函数命名使用蛇形命名法（`create_task`、`get_all_tasks`）
- [ ] 结构体命名使用驼峰命名法（`Task`、`CreateTaskRequest`）
- [ ] 无未使用的导入（`cargo clippy` 通过）

### 数据库变更

- [ ] Schema 变更通过迁移系统实现（不直接修改 SQL）
- [ ] 迁移版本号递增
- [ ] 迁移包含描述性名称
- [ ] 新表包含 `created_at` 和 `updated_at` 字段
- [ ] 枚举字段使用 CHECK 约束
- [ ] 外键定义 `ON DELETE` 行为

---

## Running Tests

### 运行所有测试

```bash
cd src-tauri
cargo test
```

### 运行特定模块测试

```bash
cargo test db::tasks::tests
```

### 查看测试输出

```bash
cargo test -- --nocapture
```

### 测试覆盖率（可选）

```bash
# 安装 tarpaulin
cargo install cargo-tarpaulin

# 生成覆盖率报告
cargo tarpaulin --out Html
```

---

## Linting and Formatting

### Clippy（静态分析）

```bash
cargo clippy
```

**常见警告**：
- 未使用的导入
- 不必要的 `clone()`
- 可简化的 `match` 表达式

### Rustfmt（代码格式化）

```bash
cargo fmt
```

### 提交前检查

```bash
# 格式化 + 检查 + 测试
cargo fmt && cargo clippy && cargo test
```

---

## Testing Anti-Patterns

### ❌ 测试依赖外部服务

```rust
// 错误：依赖真实的 WebDAV 服务器
#[test]
fn test_webdav_upload() {
    let client = WebDavClient::new(
        "https://real-server.com".to_string(),
        "user".to_string(),
        "pass".to_string(),
    ).unwrap();
    client.upload(&data).unwrap();  // 可能失败，不稳定
}
```

**正确**：使用 mock 或仅测试纯函数（如 URL 构建、base64 编码）

### ❌ 测试共享可变状态

```rust
// 错误：多个测试修改同一个文件数据库
static DB_PATH: &str = "test.db";

#[test]
fn test_a() {
    let conn = Connection::open(DB_PATH).unwrap();  // 冲突！
    // ...
}

#[test]
fn test_b() {
    let conn = Connection::open(DB_PATH).unwrap();  // 冲突！
    // ...
}
```

**正确**：使用内存数据库（每个测试独立）

### ❌ 测试断言不精确

```rust
// 错误：仅检查操作不报错，不验证结果
#[test]
fn test_create_task() {
    let conn = setup_db();
    let req = sample_task("test");
    create_task(&conn, &req).unwrap();  // 没有验证 ID > 0
}
```

**正确**：验证返回值

```rust
let id = create_task(&conn, &req).expect("Failed to create task");
assert!(id > 0);

let task = get_task(&conn, id).expect("Failed to get task");
assert_eq!(task.title, "Test Task test");
```

---

## Quality Metrics

### 当前项目质量状态

- ✅ **数据库测试覆盖**：100%（`tasks.rs`、`migrations.rs` 包含完整测试）
- ✅ **错误处理**：所有命令层转换错误为 String
- ✅ **参数化查询**：所有 SQL 使用 `params!`
- ✅ **迁移幂等性**：已验证
- ⚠️ **命令层测试**：无（命令层未单独测试，通过集成测试覆盖）
- ⚠️ **网络层测试**：仅工具函数有测试（未 mock HTTP 请求）

### 未来改进方向

1. **命令层单元测试**：模拟 `State<Arc<Mutex<Connection>>>`，验证错误转换
2. **集成测试**：启动完整的 Tauri 应用，测试前后端交互
3. **性能测试**：批量插入/查询的性能基准
4. **网络层 mock**：使用 `mockito` 或类似库模拟 WebDAV 响应

---

## Checklist

新功能开发完成后，检查以下要点：

### 开发阶段

- [ ] 运行 `cargo fmt` 格式化代码
- [ ] 运行 `cargo clippy` 检查警告
- [ ] 运行 `cargo test` 验证所有测试通过
- [ ] 为新增的数据库函数编写测试
- [ ] 为新增的迁移编写测试

### 代码审查阶段

- [ ] 检查所有 `unwrap()` / `expect()` 是否必要
- [ ] 检查错误是否正确传播（不吞掉错误）
- [ ] 检查 SQL 是否使用参数化查询
- [ ] 检查测试是否覆盖错误路径
- [ ] 检查命令是否在 `lib.rs` 中注册

### 提交前

- [ ] 所有测试通过
- [ ] 无 Clippy 警告
- [ ] 代码已格式化
- [ ] 提交消息描述清晰（使用 Conventional Commits）
