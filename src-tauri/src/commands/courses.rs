use std::sync::{Arc, Mutex};

use rusqlite::Connection;
use serde::Deserialize;
use tauri::State;

use crate::db::models::{Course, CreateCourseRequest, UpdateCourseRequest};

#[derive(Debug, Deserialize)]
pub struct CreateCourseCmd {
    pub name: String,
    pub day_of_week: i64,
    pub start_time: String,
    pub end_time: String,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub teacher: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub semester: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCourseCmd {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub day_of_week: Option<i64>,
    #[serde(default)]
    pub start_time: Option<String>,
    #[serde(default)]
    pub end_time: Option<String>,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub teacher: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub semester: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CourseFilterParams {
    #[serde(default)]
    pub semester: Option<String>,
}

#[tauri::command]
pub fn get_all_courses(
    db: State<'_, Arc<Mutex<Connection>>>,
    filters: CourseFilterParams,
) -> Result<Vec<Course>, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    crate::db::courses::get_all_courses(&conn, filters.semester.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_course(
    db: State<'_, Arc<Mutex<Connection>>>,
    cmd: CreateCourseCmd,
) -> Result<i64, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let req = CreateCourseRequest {
        name: cmd.name,
        day_of_week: cmd.day_of_week,
        start_time: cmd.start_time,
        end_time: cmd.end_time,
        location: cmd.location.unwrap_or_default(),
        teacher: cmd.teacher.unwrap_or_default(),
        color: cmd.color.unwrap_or_else(|| String::from("#7C8CC0")),
        semester: cmd.semester.unwrap_or_default(),
    };
    crate::db::courses::create_course(&conn, &req).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_course(
    db: State<'_, Arc<Mutex<Connection>>>,
    id: i64,
    cmd: UpdateCourseCmd,
) -> Result<(), String> {
    let conn = db.lock().map_err(|e| e.to_string())?;

    let existing = crate::db::courses::get_course(&conn, id).map_err(|e| e.to_string())?;

    let req = UpdateCourseRequest {
        name: cmd.name.unwrap_or(existing.name),
        day_of_week: cmd.day_of_week.unwrap_or(existing.day_of_week),
        start_time: cmd.start_time.unwrap_or(existing.start_time),
        end_time: cmd.end_time.unwrap_or(existing.end_time),
        location: cmd.location.unwrap_or(existing.location),
        teacher: cmd.teacher.unwrap_or(existing.teacher),
        color: cmd.color.unwrap_or(existing.color),
        semester: cmd.semester.unwrap_or(existing.semester),
    };
    crate::db::courses::update_course(&conn, id, &req).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_course(
    db: State<'_, Arc<Mutex<Connection>>>,
    id: i64,
) -> Result<(), String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    crate::db::courses::delete_course(&conn, id).map_err(|e| e.to_string())
}
