# Error Handling

> 错误处理模式、错误转换和传播约定

---

## Overview

Kairos 后端使用 Rust 的 `Result<T, E>` 类型进行错误处理。错误在不同层级有不同的表示形式：
- **数据库层**：返回 `rusqlite::Result<T>`（即 `Result<T, rusqlite::Error>`）
- **命令层**：转换为 `Result<T, String>`（Tauri 要求可序列化的错误类型）
- **网络层**：使用自定义错误映射函数将 `reqwest::Error` 转为用户友好消息

核心原则：
- **错误就地传播**：用 `?` 操作符向上传递，不过早处理
- **边界转换**：仅在层边界（如命令层 -> 前端）转换错误类型
- **保留上下文**：错误消息包含足够的调试信息（操作类型、参数、HTTP 状态码等）

---

## Error Types

### 1. 数据库错误 (`rusqlite::Error`)

数据库层函数直接返回 `rusqlite::Result<T>`，不进行包装。

**示例**：`src-tauri/src/db/tasks.rs:5`

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

**错误示例**：
- 主键冲突：`rusqlite::Error::SqliteFailure` (UNIQUE constraint failed)
- 外键约束：`rusqlite::Error::SqliteFailure` (FOREIGN KEY constraint failed)
- 查询不到记录：`rusqlite::Error::QueryReturnedNoRows`

### 2. 命令层错误 (`String`)

Tauri 命令必须返回可序列化的错误类型，项目统一使用 `String`。

**转换模式**：`src-tauri/src/commands/tasks.rs:52-68`

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

**两处转换点**：
1. **锁获取失败**：`db.lock().map_err(|e| e.to_string())?`
2. **数据库操作失败**：`.map_err(|e| e.to_string())`

**前端收到的错误消息示例**：
- `"no such table: tasks"` - 迁移未运行
- `"UNIQUE constraint failed: tasks.id"` - 主键冲突
- `"QueryReturnedNoRows"` - 记录不存在

### 3. 网络错误（自定义映射）

WebDAV 客户端将 `reqwest::Error` 转换为用户友好的错误消息。

**错误映射函数**：`src-tauri/src/sync/webdav.rs:125-135`

```rust
fn map_reqwest_error(err: reqwest::Error, _url: &str) -> String {
    if err.is_timeout() {
        "Connection timed out (10s)".to_string()
    } else if err.is_connect() {
        format!("Cannot connect to server: {}", err)
    } else if err.is_request() {
        format!("Request failed: {}", err)
    } else {
        format!("Network error: {}", err)
    }
}
```

**使用示例**：`src-tauri/src/sync/webdav.rs:81-87`

```rust
let response = self
    .client
    .get(&url)
    .headers(headers)
    .send()
    .map_err(|e| map_reqwest_error(e, &url))?;
```

**HTTP 错误处理**：`src-tauri/src/sync/webdav.rs:89-106`

```rust
let status = response.status();
if status.is_success() {
    let body = response
        .text()
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    serde_json::from_str::<SyncData>(&body)
        .map_err(|e| format!("Failed to parse sync data: {}", e))
} else if status.as_u16() == 404 {
    Err("No remote sync data found (404)".to_string())
} else {
    Err(format!(
        "Download failed: HTTP {} — {}",
        status.as_u16(),
        response.text().unwrap_or_default()
    ))
}
```

**错误消息设计原则**：
- 包含 HTTP 状态码（`HTTP 401`、`HTTP 404`）
- 区分网络层错误（超时、连接失败）和应用层错误（404、401）
- 对特殊状态码（404）提供明确的业务语义

---

## Error Handling Patterns

### 1. 就地传播（推荐）

使用 `?` 操作符向上传递错误，不在中间层捕获。

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
    )?;  // 错误直接向上传播
    Ok(())
}
```

### 2. 边界转换（命令层）

仅在 Tauri 命令边界转换错误类型。

**示例**：`src-tauri/src/commands/tasks.rs:88-106`

```rust
#[tauri::command]
pub fn update_task(
    db: State<'_, Arc<Mutex<Connection>>>,
    id: i64,
    cmd: UpdateTaskCmd,
) -> Result<(), String> {
    let conn = db.lock().map_err(|e| e.to_string())?;

    // 先查询现有记录（提供更好的错误消息）
    let existing = crate::db::tasks::get_task(&conn, id).map_err(|e| e.to_string())?;

    let req = UpdateTaskRequest {
        title: cmd.title.unwrap_or(existing.title),
        description: cmd.description.unwrap_or(existing.description),
        status: cmd.status.unwrap_or(existing.status),
        priority: cmd.priority.unwrap_or(existing.priority),
        due_date: cmd.due_date.or(existing.due_date),
        tags: cmd.tags.unwrap_or(existing.tags),
    };
    crate::db::tasks::update_task(&conn, id, &req).map_err(|e| e.to_string())
}
```

**模式分解**：
1. 锁获取 → 转为 String
2. 查询现有数据 → 转为 String（如果不存在，前端收到 `"QueryReturnedNoRows"`）
3. 合并补丁数据（仅更新提供的字段）
4. 执行更新 → 转为 String

### 3. 上下文增强

在网络层为错误添加操作上下文。

**示例**：`src-tauri/src/sync/webdav.rs:66-75`

```rust
let status = response.status();
if status.is_success() || status.as_u16() == 201 || status.as_u16() == 204 {
    Ok(())
} else {
    Err(format!(
        "Upload failed: HTTP {} — {}",
        status.as_u16(),
        response.text().unwrap_or_default()
    ))
}
```

**对比无上下文的错误**：
- ❌ `"401 Unauthorized"` - 不知道是上传还是下载失败
- ✅ `"Upload failed: HTTP 401 — Invalid credentials"` - 明确操作类型和原因

### 4. 特殊情况处理

对于业务上可接受的"错误"，返回特定的错误消息而非传播底层错误。

**示例**：`src-tauri/src/commands/sync.rs:72`

```rust
let remote = match webdav_client.download() {
    Ok(data) => data,
    Err(e) if e.contains("404") => {
        log::info!("No remote data found, will upload local only");
        // 首次同步时没有远程数据是正常的
        // 继续执行本地上传逻辑...
    }
    Err(e) => return Err(e),
};
```

---

## Error Propagation

### 数据库层 → 命令层

**数据库函数**（返回 `Result<T, rusqlite::Error>`）：

```rust
pub fn get_task(conn: &Connection, id: i64) -> Result<Task> {
    conn.query_row(/* ... */)
}
```

**命令函数**（转换为 `Result<T, String>`）：

```rust
#[tauri::command]
pub fn get_task_command(
    db: State<Arc<Mutex<Connection>>>,
    id: i64,
) -> Result<Task, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    crate::db::tasks::get_task(&conn, id).map_err(|e| e.to_string())
}
```

### 链式传播

数据库层内部函数可以直接用 `?` 链式调用。

**示例**：`src-tauri/src/db/migrations.rs:92-112`

```rust
let current_version: i32 = conn
    .query_row(
        "SELECT COALESCE(MAX(version), 0) FROM _migrations",
        [],
        |row| row.get(0),
    )?;  // 传播 rusqlite::Error

for (version, name, sql) in migrations {
    if version > current_version {
        let tx = conn.unchecked_transaction()?;  // 传播错误
        tx.execute_batch(sql)?;  // 传播错误
        tx.execute(
            "INSERT INTO _migrations (version, name) VALUES (?1, ?2)",
            rusqlite::params![version, name],
        )?;  // 传播错误
        tx.commit()?;  // 传播错误
    }
}
```

---

## Logging Errors

### 当前使用场景

项目目前仅在特定业务场景记录日志（非错误传播路径）。

**示例**：`src-tauri/src/commands/sync.rs:72`

```rust
log::info!("No remote data found, will upload local only");
```

### 日志级别指南

虽然项目中日志使用较少，但遵循以下约定：

- **`info!`**：正常的业务事件（首次同步、配置加载成功）
- **`warn!`**：可恢复的异常情况（超时后重试、降级到默认值）
- **`error!`**：不可恢复的错误（迁移失败、数据损坏）
- **`debug!`**：开发时的详细追踪（SQL 语句、HTTP 请求体）

**不记录日志的情况**：
- 预期的错误（如查询不到记录）直接返回给前端处理
- 用户输入验证失败（由前端负责验证）

---

## Frontend Error Handling

### 前端接收到的错误格式

Tauri 将 `Result<T, String>` 的 `Err` 包装为 JavaScript 异常。

**前端调用示例**：

```typescript
try {
  const task = await invoke<Task>("get_task_command", { id: 123 });
  console.log(task);
} catch (error) {
  // error 是字符串："QueryReturnedNoRows"
  if (error === "QueryReturnedNoRows") {
    showNotification("任务不存在");
  } else {
    showNotification(`错误：${error}`);
  }
}
```

### 错误消息国际化

后端错误消息使用英文（便于调试和日志检索），前端根据错误字符串映射为用户语言。

**前端错误映射示例**：

```typescript
const ERROR_MESSAGES: Record<string, string> = {
  "QueryReturnedNoRows": "记录不存在",
  "UNIQUE constraint failed": "该记录已存在",
  "Connection timed out (10s)": "连接超时，请检查网络",
};

function getErrorMessage(error: string): string {
  for (const [key, message] of Object.entries(ERROR_MESSAGES)) {
    if (error.includes(key)) {
      return message;
    }
  }
  return `未知错误：${error}`;
}
```

---

## Common Mistakes

### ❌ 过早转换错误类型

```rust
// 错误：数据库层返回 String（失去错误类型信息）
pub fn create_task(conn: &Connection, req: &CreateTaskRequest) -> Result<i64, String> {
    conn.execute(/* ... */)
        .map_err(|e| e.to_string())?;  // 不应该在这里转换
    Ok(conn.last_insert_rowid())
}
```

**正确**：保持 `rusqlite::Result<i64>`，由命令层转换。

### ❌ 吞掉错误

```rust
// 错误：用 unwrap_or_default 隐藏错误
let tasks = crate::db::tasks::get_all_tasks(&conn, None, None, "id", "ASC")
    .unwrap_or_default();
```

**正确**：让错误传播到前端

```rust
let tasks = crate::db::tasks::get_all_tasks(&conn, None, None, "id", "ASC")
    .map_err(|e| e.to_string())?;
```

### ❌ 不提供错误上下文

```rust
// 错误：前端不知道是哪个网络请求失败
.map_err(|e| e.to_string())?
```

**正确**：添加操作描述

```rust
.map_err(|e| format!("Failed to upload sync data: {}", e))?
```

### ❌ 使用 `panic!` 处理可恢复错误

```rust
// 错误：导致整个应用崩溃
let conn = db.lock().expect("Failed to lock database");
```

**正确**：在命令边界返回错误

```rust
let conn = db.lock().map_err(|e| e.to_string())?;
```

**例外**：应用启动时的初始化失败可以 panic（如数据库路径非法、迁移失败）

`src-tauri/src/lib.rs:32-35`

```rust
let conn = db::get_connection(
    db_path.to_str().expect("invalid db path"),
)
.expect("failed to open database connection");
```

---

## Testing Error Handling

### 测试错误路径

**示例**：`src-tauri/src/db/tasks.rs:162-166`

```rust
#[test]
fn test_get_nonexistent_task() {
    let conn = setup_db();
    let result = get_task(&conn, 999);
    assert!(result.is_err());
}
```

### 测试约束违规

**示例**：`src-tauri/src/db/migrations.rs:163-169`

```rust
// Invalid status should fail
let result = conn.execute(
    "INSERT INTO tasks (title, description, status, priority, tags, created_at, updated_at)
     VALUES ('test2', '', 'invalid_status', 'medium', '[]', '2024-01-01', '2024-01-01')",
    [],
);
assert!(result.is_err(), "Invalid status should be rejected");
```

### 测试网络错误

**示例**：`src-tauri/src/sync/webdav.rs:206-213` (测试错误映射逻辑)

```rust
#[test]
fn test_base64_encode() {
    assert_eq!(base64_encode(""), "");
    assert_eq!(base64_encode("foo:bar"), "Zm9vOmJhcg==");
}
```

（注：当前项目的网络测试不模拟失败场景，仅测试正常路径和工具函数）

---

## Checklist

实现新功能时的错误处理检查：

- [ ] 数据库层函数返回 `rusqlite::Result<T>`
- [ ] 命令层使用 `.map_err(|e| e.to_string())` 转换错误
- [ ] 锁获取失败正确处理（不使用 `unwrap`）
- [ ] 网络错误包含操作上下文（上传/下载/连接测试）
- [ ] HTTP 错误包含状态码和响应体
- [ ] 特殊业务状态（如 404 首次同步）有专门的处理逻辑
- [ ] 测试覆盖错误路径（不存在的 ID、约束违规）
- [ ] 前端可以根据错误字符串做出合理响应

---

## Scenario: WebDAV 自动同步运行态契约

### 1. Scope / Trigger

- Trigger: `sync_now`、`update_sync_config`、`sync-finished` 事件和自动同步 worker 形成了新的跨层运行态契约。
- Scope: 手动同步、自动同步、前端 SyncSettings 页面刷新、运行中互斥和停用/重启语义。

### 2. Signatures

- Backend command:
  - `get_sync_config() -> Result<SyncConfig, String>`
  - `update_sync_config(config: SyncConfig) -> Result<(), String>`
  - `sync_now() -> Result<SyncResult, String>`
- Backend runtime:
  - `spawn_auto_sync_worker(db_path, &AutoSyncState, app_handle)`
  - `auto_sync_loop(db_path, enabled, running, worker_epoch, current_epoch, app_handle)`
- Frontend event:
  - `listen<{ last_sync_at: string }>("sync-finished", ...)`

### 3. Contracts

- Request fields:
  - `SyncConfig.auto_sync = true` 且 `server_url != ""` 时，后端允许启动自动同步 worker。
  - `update_sync_config` 必须先持久化数据库，再决定启停 worker。
- Response / event fields:
  - `sync_now` 成功返回 `SyncResult`，失败返回可序列化字符串。
  - `sync-finished` payload 只包含持久化后的 `last_sync_at`，格式为 UTC ISO 8601 字符串。
- Runtime fields:
  - `AutoSyncState.running` 是手动/自动同步共享的全局互斥护栏。
  - `AutoSyncState.worker_epoch` 是 worker 代次。关闭或重新开启自动同步时必须 bump，旧线程据此退出。

### 4. Validation & Error Matrix

- `server_url == ""` -> `sync_now` 返回 `"Server URL not configured"`
- 已有同步进行中 -> `sync_now` 返回 `"Sync already in progress"`
- 自动同步触发时 `running = true` -> 本轮跳过，仅记录日志
- `auto_sync` 从关到开 -> 先写库，再启动新 worker 代次
- `auto_sync` 从开到关 -> 先写库，再停用当前 worker 代次
- WebDAV 下载/上传失败 -> 返回/记录带上下文的错误字符串，不弹前端成功事件
- 只有在 `last_sync_at` 已持久化后，才允许 emit `sync-finished`

### 5. Good / Base / Bad Cases

- Good:
  - 用户修改配置并开启自动同步，应用启动后延迟窗口内只存在一个有效 worker。
  - 页面打开时收到 `sync-finished`，显示数据库中的真实 `last_sync_at`。
- Base:
  - 自动同步失败只打日志，下一周期继续尝试。
  - 页面未打开时自动同步成功，只更新数据库；下次进入页面再读取。
- Bad:
  - 关闭再快速开启自动同步后，旧 worker 继续长期存活并参与轮询。
  - 前端用 `new Date().toISOString()` 猜测同步时间，而不是读取持久化值。
  - 手动同步与自动同步同时上传同一份快照。

### 6. Tests Required

- Unit:
  - `AutoSyncState` 停用/重启后旧 epoch 失效
  - `SyncGuard` 第二次 acquire 失败，drop 后可重新 acquire
- Integration:
  - `sync_now` 在空 `server_url` 下返回明确错误
  - `update_sync_config` 不因快速切换 `auto_sync` 留下长期有效的旧 worker
- Frontend assertion points:
  - `SyncSettings` 监听 `sync-finished` 后刷新 `last_sync_at`
  - 页面卸载前若存在未保存草稿，会触发一次最终持久化

### 7. Wrong vs Correct

#### Wrong

- `update_sync_config` 直接 `spawn` 新线程，但不给旧线程失效信号
- 前端在同步成功后本地猜时间，或只在按钮点击时才保存配置

#### Correct

- worker 启停通过 `worker_epoch` 代次失效保证“旧线程可退出，新线程可独占”
- 前端先保存 `SyncConfig`，再执行测试连接 / 立即同步 / 开关切换
- `sync-finished` 只传递后端已落库的 `last_sync_at`
