use std::sync::{Arc, Mutex};

use rusqlite::Connection;
use serde::Deserialize;
use tauri::State;

use crate::db::models::{CreateTaskRequest, Task, UpdateTaskRequest};

#[derive(Debug, Deserialize)]
pub struct CreateTaskCmd {
    title: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    priority: Option<String>,
    #[serde(default)]
    due_date: Option<String>,
    #[serde(default)]
    tags: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTaskCmd {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    priority: Option<String>,
    #[serde(default)]
    due_date: Option<String>,
    #[serde(default)]
    tags: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TaskFilterParams {
    #[serde(default)]
    status_filter: Option<String>,
    #[serde(default)]
    priority_filter: Option<String>,
    #[serde(default)]
    sort_by: Option<String>,
    #[serde(default)]
    sort_order: Option<String>,
}

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

#[tauri::command]
pub fn create_task(
    db: State<'_, Arc<Mutex<Connection>>>,
    cmd: CreateTaskCmd,
) -> Result<i64, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let req = CreateTaskRequest {
        title: cmd.title,
        description: cmd.description.unwrap_or_default(),
        status: cmd.status.unwrap_or_else(|| String::from("todo")),
        priority: cmd.priority.unwrap_or_else(|| String::from("medium")),
        due_date: cmd.due_date,
        tags: cmd.tags.unwrap_or_else(|| String::from("[]")),
    };
    crate::db::tasks::create_task(&conn, &req).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_task(
    db: State<'_, Arc<Mutex<Connection>>>,
    id: i64,
    cmd: UpdateTaskCmd,
) -> Result<(), String> {
    let conn = db.lock().map_err(|e| e.to_string())?;

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

#[tauri::command]
pub fn delete_task(db: State<'_, Arc<Mutex<Connection>>>, id: i64) -> Result<(), String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    crate::db::tasks::delete_task(&conn, id).map_err(|e| e.to_string())
}
