use rusqlite::{params, Connection, Result};

use super::models::{CreateTaskRequest, Task, UpdateTaskRequest};

pub fn create_task(conn: &Connection, req: &CreateTaskRequest) -> Result<i64> {
    let now = super::chrono_now();
    conn.execute(
        "INSERT INTO tasks (sync_id, title, description, status, priority, due_date, tags, is_daily, last_completed_date, reminder_time, remind_at, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12)",
        params![
            crate::sync::ids::new_sync_id(),
            req.title,
            req.description,
            req.status,
            req.priority,
            req.due_date,
            req.tags,
            req.is_daily as i64,
            Option::<String>::None, // 创建时从未完成，last_completed_date 恒 null
            req.reminder_time,
            req.remind_at,
            now,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_task(conn: &Connection, id: i64) -> Result<Task> {
    conn.query_row(
        "SELECT id, sync_id, title, description, status, priority, due_date, tags, created_at, updated_at, is_daily, last_completed_date, reminder_time, remind_at, deleted_at
         FROM tasks WHERE id = ?1 AND deleted_at IS NULL",
        params![id],
        |row| {
            Ok(Task {
                id: row.get(0)?,
                sync_id: row.get(1)?,
                title: row.get(2)?,
                description: row.get(3)?,
                status: row.get(4)?,
                priority: row.get(5)?,
                due_date: row.get(6)?,
                tags: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
                is_daily: row.get::<_, i64>(10)? != 0,
                last_completed_date: row.get(11)?,
                reminder_time: row.get(12)?,
                remind_at: row.get(13)?,
                deleted_at: row.get(14)?,
            })
        },
    )
}

pub fn get_all_tasks(
    conn: &Connection,
    status_filter: Option<&str>,
    priority_filter: Option<&str>,
    sort_by: &str,
    sort_order: &str,
) -> Result<Vec<Task>> {
    let allowed_sort_columns = [
        "title",
        "status",
        "priority",
        "due_date",
        "created_at",
        "updated_at",
    ];
    // 白名单防止 SQL 注入：只允许预定义的列名出现在 ORDER BY 子句中。
    let sort_column = if allowed_sort_columns.contains(&sort_by) {
        sort_by
    } else {
        "created_at"
    };

    let order = if sort_order.eq_ignore_ascii_case("ASC") {
        "ASC"
    } else {
        "DESC"
    };

    let mut sql = String::from(
        "SELECT id, sync_id, title, description, status, priority, due_date, tags, created_at, updated_at, is_daily, last_completed_date, reminder_time, remind_at, deleted_at FROM tasks WHERE deleted_at IS NULL",
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

    let param_refs: Vec<&dyn rusqlite::types::ToSql> =
        params_vec.iter().map(|p| p.as_ref()).collect();
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(param_refs.as_slice(), |row| {
        Ok(Task {
            id: row.get(0)?,
            sync_id: row.get(1)?,
            title: row.get(2)?,
            description: row.get(3)?,
            status: row.get(4)?,
            priority: row.get(5)?,
            due_date: row.get(6)?,
            tags: row.get(7)?,
            created_at: row.get(8)?,
            updated_at: row.get(9)?,
            is_daily: row.get::<_, i64>(10)? != 0,
            last_completed_date: row.get(11)?,
            reminder_time: row.get(12)?,
            remind_at: row.get(13)?,
            deleted_at: row.get(14)?,
        })
    })?;

    rows.collect()
}

pub fn update_task(conn: &Connection, id: i64, req: &UpdateTaskRequest) -> Result<()> {
    conn.execute(
        "UPDATE tasks
         SET title = ?1, description = ?2, status = ?3, priority = ?4, due_date = ?5, tags = ?6,
             is_daily = ?7, last_completed_date = ?8, reminder_time = ?9, remind_at = ?10, updated_at = ?11
         WHERE id = ?12",
        params![
            req.title,
            req.description,
            req.status,
            req.priority,
            req.due_date,
            req.tags,
            req.is_daily as i64,
            req.last_completed_date,
            req.reminder_time,
            req.remind_at,
            super::chrono_now(),
            id,
        ],
    )?;
    Ok(())
}

pub fn delete_task(conn: &Connection, id: i64) -> Result<()> {
    let now = super::chrono_now();
    conn.execute(
        "UPDATE tasks SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2",
        params![now, id],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations;
    use rusqlite::Connection;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().expect("Failed to open in-memory DB");
        conn.pragma_update(None, "foreign_keys", "ON")
            .expect("Failed to enable foreign keys");
        migrations::run_migrations(&conn).expect("Migrations failed");
        conn
    }

    fn sample_task(extra: &str) -> CreateTaskRequest {
        CreateTaskRequest {
            title: format!("Test Task {}", extra),
            description: String::from("A test task"),
            status: String::from("todo"),
            priority: String::from("medium"),
            due_date: None,
            tags: String::from("[]"),
            is_daily: false,
            reminder_time: None,
            remind_at: None,
        }
    }

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

    #[test]
    fn test_get_nonexistent_task() {
        let conn = setup_db();
        let result = get_task(&conn, 999);
        assert!(result.is_err());
    }

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
            is_daily: false,
            last_completed_date: None,
            reminder_time: None,
            remind_at: None,
        };
        update_task(&conn, id, &update).expect("Failed to update task");

        let task = get_task(&conn, id).expect("Failed to get updated task");
        assert_eq!(task.title, "Updated Title");
        assert_eq!(task.status, "in_progress");
        assert_eq!(task.priority, "high");
        assert_eq!(task.due_date.as_deref(), Some("2024-12-31"));
    }

    #[test]
    fn test_remind_at_roundtrip() {
        let conn = setup_db();

        // 创建时写入 remind_at。
        let req = CreateTaskRequest {
            title: String::from("Remind me"),
            description: String::new(),
            status: String::from("todo"),
            priority: String::from("medium"),
            due_date: None,
            tags: String::from("[]"),
            is_daily: false,
            reminder_time: None,
            remind_at: Some(String::from("2026-08-05 14:00")),
        };
        let id = create_task(&conn, &req).expect("Failed to create task");
        let task = get_task(&conn, id).expect("Failed to get task");
        assert_eq!(task.remind_at.as_deref(), Some("2026-08-05 14:00"));

        // 更新改 remind_at。
        let update = UpdateTaskRequest {
            title: String::from("Remind me"),
            description: String::new(),
            status: String::from("todo"),
            priority: String::from("medium"),
            due_date: None,
            tags: String::from("[]"),
            is_daily: false,
            last_completed_date: None,
            reminder_time: None,
            remind_at: Some(String::from("2026-08-06 09:30")),
        };
        update_task(&conn, id, &update).expect("Failed to update remind_at");
        let task = get_task(&conn, id).expect("Failed to get updated task");
        assert_eq!(task.remind_at.as_deref(), Some("2026-08-06 09:30"));

        // 清空 remind_at（提醒发送后/过期后置 null）。
        let clear = UpdateTaskRequest {
            remind_at: None,
            ..update
        };
        update_task(&conn, id, &clear).expect("Failed to clear remind_at");
        let task = get_task(&conn, id).expect("Failed to get cleared task");
        assert!(task.remind_at.is_none());
    }

    #[test]
    fn test_delete_task() {
        let conn = setup_db();

        let req = sample_task("del");
        let id = create_task(&conn, &req).expect("Failed to create task");

        delete_task(&conn, id).expect("Failed to delete task");

        let result = get_task(&conn, id);
        assert!(result.is_err());
    }

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
            is_daily: false,
            reminder_time: None,
            remind_at: None,
        };
        let t2 = CreateTaskRequest {
            title: String::from("Low priority task"),
            description: String::new(),
            status: String::from("todo"),
            priority: String::from("low"),
            due_date: None,
            tags: String::from("[]"),
            is_daily: false,
            reminder_time: None,
            remind_at: None,
        };
        let t3 = CreateTaskRequest {
            title: String::from("Done task"),
            description: String::new(),
            status: String::from("done"),
            priority: String::from("medium"),
            due_date: None,
            tags: String::from("[]"),
            is_daily: false,
            reminder_time: None,
            remind_at: None,
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

    #[test]
    fn test_get_all_tasks_empty() {
        let conn = setup_db();
        let tasks =
            get_all_tasks(&conn, None, None, "created_at", "DESC").expect("Failed to get tasks");
        assert!(tasks.is_empty());
    }

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
            is_daily: false,
            reminder_time: None,
            remind_at: None,
        };
        let t2 = CreateTaskRequest {
            title: String::from("Z"),
            description: String::new(),
            status: String::from("todo"),
            priority: String::from("medium"),
            due_date: None,
            tags: String::from("[]"),
            is_daily: false,
            reminder_time: None,
            remind_at: None,
        };
        create_task(&conn, &t1).expect("Failed to create t1");
        create_task(&conn, &t2).expect("Failed to create t2");

        let asc = get_all_tasks(&conn, None, None, "title", "ASC").expect("Failed to sort ASC");
        assert_eq!(asc[0].title, "A");
        assert_eq!(asc[1].title, "Z");

        let desc = get_all_tasks(&conn, None, None, "title", "DESC").expect("Failed to sort DESC");
        assert_eq!(desc[0].title, "Z");
        assert_eq!(desc[1].title, "A");
    }
}
