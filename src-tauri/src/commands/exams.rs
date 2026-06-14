use std::sync::{Arc, Mutex};

use rusqlite::Connection;
use serde::Deserialize;
use tauri::State;

use crate::db::models::{CreateExamRequest, Exam, UpdateExamRequest};

#[derive(Debug, Deserialize)]
pub struct CreateExamCmd {
    pub course_name: String,
    pub exam_datetime: String,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub course_id: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateExamCmd {
    #[serde(default)]
    pub course_name: Option<String>,
    #[serde(default)]
    pub exam_datetime: Option<String>,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub course_id: Option<i64>,
}

#[tauri::command]
pub fn get_all_exams(
    db: State<'_, Arc<Mutex<Connection>>>,
) -> Result<Vec<Exam>, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    crate::db::exams::get_all_exams(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_exam(
    db: State<'_, Arc<Mutex<Connection>>>,
    cmd: CreateExamCmd,
) -> Result<i64, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let req = CreateExamRequest {
        course_name: cmd.course_name,
        exam_datetime: cmd.exam_datetime,
        location: cmd.location.unwrap_or_default(),
        notes: cmd.notes.unwrap_or_default(),
        course_id: cmd.course_id,
    };
    crate::db::exams::create_exam(&conn, &req).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_exam(
    db: State<'_, Arc<Mutex<Connection>>>,
    id: i64,
    cmd: UpdateExamCmd,
) -> Result<(), String> {
    let conn = db.lock().map_err(|e| e.to_string())?;

    let existing = crate::db::exams::get_exam(&conn, id).map_err(|e| e.to_string())?;

    let req = UpdateExamRequest {
        course_name: cmd.course_name.unwrap_or(existing.course_name),
        exam_datetime: cmd.exam_datetime.unwrap_or(existing.exam_datetime),
        location: cmd.location.unwrap_or(existing.location),
        notes: cmd.notes.unwrap_or(existing.notes),
        course_id: match cmd.course_id {
            Some(val) => Some(val),
            None => existing.course_id,
        },
    };
    crate::db::exams::update_exam(&conn, id, &req).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_exam(
    db: State<'_, Arc<Mutex<Connection>>>,
    id: i64,
) -> Result<(), String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    crate::db::exams::delete_exam(&conn, id).map_err(|e| e.to_string())
}
