use std::sync::{Arc, Mutex};

use rusqlite::Connection;
use serde::Deserialize;
use tauri::{AppHandle, State};

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
    #[serde(default)]
    is_daily: Option<bool>,
    #[serde(default)]
    reminder_time: Option<String>,
    /// 普通任务一次性提醒时间（YYYY-MM-DD HH:MM）；null = 不提醒。
    #[serde(default)]
    remind_at: Option<String>,
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
    #[serde(default)]
    is_daily: Option<bool>,
    #[serde(default)]
    reminder_time: Option<String>,
    /// 普通任务一次性提醒时间（YYYY-MM-DD HH:MM）；null = 不提醒。
    #[serde(default)]
    remind_at: Option<String>,
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
    app_handle: AppHandle,
    cmd: CreateTaskCmd,
) -> Result<i64, String> {
    let id = {
        let conn = db.lock().map_err(|e| e.to_string())?;
        let req = CreateTaskRequest {
            title: cmd.title,
            description: cmd.description.unwrap_or_default(),
            status: cmd.status.unwrap_or_else(|| String::from("todo")),
            priority: cmd.priority.unwrap_or_else(|| String::from("medium")),
            due_date: cmd.due_date,
            tags: cmd.tags.unwrap_or_else(|| String::from("[]")),
            is_daily: cmd.is_daily.unwrap_or(false),
            reminder_time: cmd.reminder_time,
            remind_at: cmd.remind_at,
        };
        crate::db::tasks::create_task(&conn, &req).map_err(|e| e.to_string())?
    };
    // 任务/提醒配置变化后重建提醒线程（先释放 db 锁，reschedule 会再次加锁）。
    crate::notifications::daily_reminder::reschedule(db.inner().clone(), &app_handle);
    Ok(id)
}

#[tauri::command]
pub fn update_task(
    db: State<'_, Arc<Mutex<Connection>>>,
    app_handle: AppHandle,
    id: i64,
    cmd: UpdateTaskCmd,
) -> Result<(), String> {
    {
        let conn = db.lock().map_err(|e| e.to_string())?;
        let existing = crate::db::tasks::get_task(&conn, id).map_err(|e| e.to_string())?;

        let new_status = cmd.status.unwrap_or(existing.status.clone());
        let mut req = UpdateTaskRequest {
            title: cmd.title.unwrap_or(existing.title),
            description: cmd.description.unwrap_or(existing.description),
            status: new_status,
            priority: cmd.priority.unwrap_or(existing.priority),
            due_date: cmd.due_date.or(existing.due_date),
            tags: cmd.tags.unwrap_or(existing.tags),
            is_daily: cmd.is_daily.unwrap_or(existing.is_daily),
            // 完成日期由专用 complete/uncomplete 命令维护，编辑任务不直接改动。
            last_completed_date: existing.last_completed_date,
            reminder_time: cmd.reminder_time.or(existing.reminder_time),
            remind_at: cmd.remind_at.or(existing.remind_at),
        };
        // 普通任务完成（status → done）：一次性提醒随完成自动消失，避免已完成的任务继续触发提醒。
        if existing.status != "done" && req.status == "done" {
            req.remind_at = None;
        }
        crate::db::tasks::update_task(&conn, id, &req).map_err(|e| e.to_string())?;
    }
    crate::notifications::daily_reminder::reschedule(db.inner().clone(), &app_handle);
    Ok(())
}

#[tauri::command]
pub fn delete_task(
    db: State<'_, Arc<Mutex<Connection>>>,
    app_handle: AppHandle,
    id: i64,
) -> Result<(), String> {
    {
        let conn = db.lock().map_err(|e| e.to_string())?;
        crate::db::tasks::delete_task(&conn, id).map_err(|e| e.to_string())?;
    }
    crate::notifications::daily_reminder::reschedule(db.inner().clone(), &app_handle);
    Ok(())
}

/// 完成/取消完成每日任务。`completed=true` 标记今日已完成，`false` 回到 todo。
fn set_daily_completed(
    db: &State<'_, Arc<Mutex<Connection>>>,
    app_handle: &AppHandle,
    id: i64,
    completed: bool,
) -> Result<(), String> {
    {
        let conn = db.lock().map_err(|e| e.to_string())?;
        let existing = crate::db::tasks::get_task(&conn, id).map_err(|e| e.to_string())?;
        if !existing.is_daily {
            return Err("该任务不是每日任务".to_string());
        }
        let req = UpdateTaskRequest {
            status: if completed { "done" } else { "todo" }.to_string(),
            last_completed_date: if completed {
                Some(crate::ai::morning_brief::today_china())
            } else {
                None
            },
            ..task_to_update(&existing)
        };
        crate::db::tasks::update_task(&conn, id, &req).map_err(|e| e.to_string())?;
    }
    crate::notifications::daily_reminder::reschedule(db.inner().clone(), app_handle);
    Ok(())
}

/// 完成每日任务：标记今日已完成（status=done + last_completed_date=今日）。
#[tauri::command]
pub fn complete_daily_task(
    db: State<'_, Arc<Mutex<Connection>>>,
    app_handle: AppHandle,
    id: i64,
) -> Result<(), String> {
    set_daily_completed(&db, &app_handle, id, true)
}

/// 取消今日完成：status 回到 todo，last_completed_date 清空。
#[tauri::command]
pub fn uncomplete_daily_task(
    db: State<'_, Arc<Mutex<Connection>>>,
    app_handle: AppHandle,
    id: i64,
) -> Result<(), String> {
    set_daily_completed(&db, &app_handle, id, false)
}

/// 把 Task 转成全字段 UpdateTaskRequest（保留其余字段，仅覆盖指定字段）。
fn task_to_update(existing: &Task) -> UpdateTaskRequest {
    UpdateTaskRequest {
        title: existing.title.clone(),
        description: existing.description.clone(),
        status: existing.status.clone(),
        priority: existing.priority.clone(),
        due_date: existing.due_date.clone(),
        tags: existing.tags.clone(),
        is_daily: existing.is_daily,
        last_completed_date: existing.last_completed_date.clone(),
        reminder_time: existing.reminder_time.clone(),
        remind_at: existing.remind_at.clone(),
    }
}
