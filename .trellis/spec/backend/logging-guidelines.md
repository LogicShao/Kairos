# Logging Guidelines

> 日志记录的使用规范和最佳实践

---

## Overview

Kairos 后端使用 **`log`** crate 作为日志门面，生产环境通过 **`tauri-plugin-log`** 实现日志后端。项目采用轻量级日志策略：仅记录关键业务事件和异常情况，常规的 CRUD 操作不记录日志（通过错误返回值处理）。

当前配置：
- **调试构建**：启用日志，级别 `Info`
- **发布构建**：不启用日志插件（减少开销）
- **日志输出**：控制台（开发时）/ 文件（未配置）

---

## Log Configuration

### 日志初始化

仅在调试构建时启用日志。

**示例**：`src-tauri/src/lib.rs:16-22`

```rust
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            // ...
        })
}
```

**配置说明**：
- `cfg!(debug_assertions)` - 仅在 `cargo build`（非 `--release`）时启用
- `.level(log::LevelFilter::Info)` - 记录 `info!` 及以上级别（`warn!`、`error!`）
- `debug!` 和 `trace!` 不会输出（除非手动调整级别）

### 生产环境日志

当前生产构建 **不包含** 日志系统（无性能开销）。如需在发布版本启用日志，移除 `if cfg!(debug_assertions)` 条件：

```rust
// 始终启用日志
app.handle().plugin(
    tauri_plugin_log::Builder::default()
        .level(log::LevelFilter::Warn)  // 生产环境仅记录警告和错误
        .target(tauri_plugin_log::Target::LogDir { file_name: None })  // 写入文件
        .build(),
)?;
```

---

## Log Levels

### 级别定义

| 级别    | 宏调用       | 使用场景                                   |
|---------|--------------|------------------------------------------|
| Error   | `error!()`   | 不可恢复的错误，需要人工介入               |
| Warn    | `warn!()`    | 可恢复的异常情况，可能影响功能               |
| Info    | `info!()`    | 重要的业务事件（当前主要使用的级别）         |
| Debug   | `debug!()`   | 开发时的详细追踪（当前未使用）               |
| Trace   | `trace!()`   | 极详细的追踪信息（当前未使用）               |

### 当前使用情况

项目中仅有 **1 处日志调用**：

`src-tauri/src/commands/sync.rs:72`

```rust
let remote = match webdav_client.download() {
    Ok(data) => data,
    Err(e) if e.contains("404") => {
        log::info!("No remote data found, will upload local only");
        // 首次同步时远程无数据是正常情况，记录以便调试
        local.clone()
    }
    Err(e) => return Err(e),
};
```

**设计原因**：
- 这是一个 **正常的业务场景**（首次同步），不是错误
- 用户不需要看到提示，但开发者需要知道执行了哪个分支
- 使用 `info!` 而非 `debug!`，因为这是关键的同步流程决策点

---

## What to Log

### 应该记录的事件

基于项目现状，以下场景 **推荐** 记录日志（但当前大部分未实现）：

#### 1. 应用生命周期事件

```rust
// 应用启动
log::info!("Kairos started, database at: {}", db_path);

// 迁移执行
log::info!("Running database migration: v{} - {}", version, name);

// 应用关闭（如果需要）
log::info!("Shutting down Pomodoro engine");
```

#### 2. 关键业务操作

```rust
// 同步操作
log::info!("WebDAV sync started");
log::info!("Sync completed: {} tasks, {} courses uploaded", tasks_count, courses_count);

// 批量操作
log::info!("Deleted {} expired sessions", count);
```

#### 3. 配置变更

```rust
log::info!("Pomodoro config updated: work={}s, break={}s", work_seconds, break_seconds);
log::info!("WebDAV server changed to: {}", server_url);
```

#### 4. 异常但可恢复的情况

```rust
// 外部服务不可用时降级
log::warn!("WebDAV connection failed, continuing in offline mode");

// 数据修复
log::warn!("Found orphaned session (task_id={} not found), setting to NULL", task_id);
```

#### 5. 不可恢复的错误

```rust
// 数据库损坏
log::error!("Database corruption detected: {}", err);

// 迁移失败（应用无法启动）
log::error!("Migration v{} failed: {}", version, err);
```

### 不应该记录的事件

以下场景 **不需要** 记录日志（通过返回值处理）：

#### 1. 正常的 CRUD 操作

```rust
// ❌ 不要记录每次查询
pub fn get_task(conn: &Connection, id: i64) -> Result<Task> {
    // log::debug!("Querying task id={}", id);  // 不需要
    conn.query_row(/* ... */)
}
```

**原因**：CRUD 操作是高频操作，记录日志会产生大量噪音。

#### 2. 预期的错误

```rust
// ❌ 不要记录用户输入错误
match crate::db::tasks::get_task(&conn, id) {
    Ok(task) => task,
    Err(rusqlite::Error::QueryReturnedNoRows) => {
        // log::warn!("Task {} not found", id);  // 不需要
        return Err("Task not found".to_string());
    }
    Err(e) => return Err(e.to_string()),
}
```

**原因**：这是前端应该处理的业务逻辑，不是异常情况。

#### 3. 高频的后台任务

```rust
// ❌ 不要在定时器 tick 中记录
std::thread::spawn(move || loop {
    std::thread::sleep(Duration::from_secs(1));
    let mut eng = tick_engine.lock().unwrap();
    // log::trace!("Pomodoro tick: {}s remaining", eng.remaining_seconds());  // 不需要
    eng.tick();
});
```

**原因**：每秒触发一次，日志会迅速膨胀。

---

## Structured Logging

### 当前格式

使用 `log` crate 的默认格式：

```
[2024-01-15 08:30:00] INFO  kairos::commands::sync: No remote data found, will upload local only
```

### 未来扩展（如需结构化日志）

可以使用 `serde_json` 输出结构化日志：

```rust
use serde_json::json;

log::info!("{}", json!({
    "event": "sync_completed",
    "duration_ms": 1234,
    "uploaded": { "tasks": 5, "courses": 3 },
    "downloaded": { "tasks": 2, "courses": 1 },
}));
```

**输出**：

```json
{"event":"sync_completed","duration_ms":1234,"uploaded":{"tasks":5,"courses":3},"downloaded":{"tasks":2,"courses":1}}
```

（当前项目未使用此模式）

---

## What NOT to Log

### 1. 敏感数据

```rust
// ❌ 永远不要记录密码
log::info!("Connecting to WebDAV: user={}, pass={}", username, password);

// ✅ 仅记录非敏感信息
log::info!("Connecting to WebDAV: user={}, server={}", username, server_url);
```

**敏感字段**：
- `sync_config.password` - WebDAV 密码
- 用户输入的任何认证信息

### 2. 大型数据结构

```rust
// ❌ 不要记录完整的数据快照
log::debug!("Sync data: {:?}", sync_data);  // SyncData 可能包含数百条记录

// ✅ 仅记录摘要
log::info!("Uploading {} tasks, {} courses", sync_data.tasks.len(), sync_data.courses.len());
```

### 3. 用户隐私数据

```rust
// ❌ 不要记录任务标题/描述
log::info!("Created task: {}", task.title);

// ✅ 仅记录 ID
log::info!("Created task: id={}", task.id);
```

**例外**：调试特定用户问题时，可在本地临时启用详细日志。

---

## Logging in Tests

### 测试中的日志输出

默认情况下，`cargo test` 会捕获日志输出。要查看日志，使用：

```bash
cargo test -- --nocapture
```

### 测试专用日志初始化

当前测试不初始化日志（因为使用内存数据库，无需追踪）。如需在测试中启用日志：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn init_test_logging() {
        let _ = env_logger::builder()
            .is_test(true)
            .try_init();
    }

    #[test]
    fn test_sync_operation() {
        init_test_logging();
        log::info!("Starting sync test");
        // ...
    }
}
```

---

## Performance Considerations

### 日志对性能的影响

1. **字符串格式化开销**：即使日志级别被过滤，`format!()` 仍会执行

   ```rust
   // ❌ 总是格式化（即使 debug! 被过滤）
   log::debug!("Processing task: {}", expensive_debug_format(&task));

   // ✅ 延迟格式化
   if log::log_enabled!(log::Level::Debug) {
       log::debug!("Processing task: {}", expensive_debug_format(&task));
   }
   ```

2. **当前项目的策略**：因日志使用极少，无需优化

---

## Common Mistakes

### ❌ 日志级别选择不当

```rust
// 错误：将正常的业务事件标记为 error
log::error!("User created a task");

// 正确：使用 info
log::info!("Task created: id={}", task_id);
```

### ❌ 在循环中记录日志

```rust
// 错误：可能产生上千条日志
for task in tasks {
    log::info!("Processing task: {}", task.id);
    process(task);
}

// 正确：仅记录批次摘要
log::info!("Processing {} tasks", tasks.len());
for task in tasks {
    process(task);
}
```

### ❌ 使用 `println!` 代替日志

```rust
// 错误：无法控制级别和输出目标
println!("Sync completed");

// 正确：使用日志系统
log::info!("Sync completed");
```

### ❌ 记录错误后又返回错误

```rust
// 错误：重复记录（调用栈上层可能也会记录）
pub fn create_task(conn: &Connection, req: &CreateTaskRequest) -> Result<i64> {
    conn.execute(/* ... */)
        .map_err(|e| {
            log::error!("Failed to create task: {}", e);  // 不需要
            e
        })
}

// 正确：仅在最终处理点记录
#[tauri::command]
pub fn create_task_command(/* ... */) -> Result<i64, String> {
    crate::db::tasks::create_task(&conn, &req)
        .map_err(|e| {
            log::error!("Task creation failed: {}", e);  // 在这里记录
            e.to_string()
        })
}
```

---

## Examples from Codebase

### 当前唯一的日志使用

`src-tauri/src/commands/sync.rs:72`

```rust
let remote = match webdav_client.download() {
    Ok(data) => data,
    Err(e) if e.contains("404") => {
        log::info!("No remote data found, will upload local only");
        local.clone()
    }
    Err(e) => return Err(e),
};
```

**为什么这样设计**：
1. **业务决策点**：404 是正常场景，但需要记录以便理解同步行为
2. **使用 `info!` 而非 `debug!`**：即使在生产环境，这也是值得记录的关键事件
3. **不记录错误情况**：其他错误直接返回，由调用方处理

### 推荐的日志使用（未实现）

如果未来扩展日志记录，推荐的模式：

```rust
// 应用启动
pub fn run() {
    log::info!("Kairos {} starting", env!("CARGO_PKG_VERSION"));
    tauri::Builder::default()
        .setup(|app| {
            let db_path = app.path().app_data_dir()?;
            log::info!("Database path: {}", db_path.display());
            // ...
        })
}

// 迁移执行
for (version, name, sql) in migrations {
    if version > current_version {
        log::info!("Applying migration v{}: {}", version, name);
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(sql)?;
        tx.commit()?;
    }
}

// 同步操作
pub fn sync_now(/* ... */) -> Result<(), String> {
    log::info!("Starting WebDAV sync to {}", server_url);
    let start = std::time::Instant::now();
    
    // 执行同步...
    
    log::info!(
        "Sync completed in {}ms: uploaded {} items, downloaded {} items",
        start.elapsed().as_millis(),
        upload_count,
        download_count
    );
    Ok(())
}
```

---

## Checklist

实现新功能时的日志检查：

- [ ] 仅在关键业务决策点记录日志（不记录每个 CRUD 操作）
- [ ] 使用正确的日志级别（`info!` 用于正常事件，`warn!` 用于异常，`error!` 用于错误）
- [ ] 不记录敏感数据（密码、用户隐私）
- [ ] 不在高频操作中记录日志（定时器 tick、查询循环）
- [ ] 日志消息包含足够的上下文（操作类型、关键参数）
- [ ] 在测试中不依赖日志输出（用 `assert!` 验证行为）
- [ ] 考虑日志的性能影响（避免昂贵的格式化）
