use std::sync::{Arc, Mutex};

use rusqlite::Connection;
use serde::Deserialize;
use tauri::State;

#[derive(Debug, Deserialize)]
pub struct WeekScheduleCmd {
    pub semester: String,
    pub week_index: i64,
    #[serde(default)]
    pub semester_start_date: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CalendarWeekCmd {
    pub semester: String,
    pub week_index: i64,
    #[serde(default)]
    pub semester_start_date: Option<String>,
    #[serde(default)]
    pub week_start_date: Option<String>,
}

#[tauri::command]
pub fn get_week_schedule(
    db: State<'_, Arc<Mutex<Connection>>>,
    cmd: WeekScheduleCmd,
) -> Result<crate::schedule::WeekScheduleResponse, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let courses = crate::db::courses::get_all_courses(&conn, Some(&cmd.semester))
        .map_err(|e| e.to_string())?;
    let exams = crate::db::exams::get_all_exams(&conn).map_err(|e| e.to_string())?;

    crate::schedule::build_week_schedule(
        &courses,
        &exams,
        &cmd.semester,
        cmd.week_index,
        cmd.semester_start_date.as_deref(),
    )
}

#[tauri::command]
pub fn get_calendar_week(
    db: State<'_, Arc<Mutex<Connection>>>,
    cmd: CalendarWeekCmd,
) -> Result<crate::schedule::CalendarWeekResponse, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let courses = crate::db::courses::get_all_courses(&conn, Some(&cmd.semester))
        .map_err(|e| e.to_string())?;
    let exams = crate::db::exams::get_all_exams(&conn).map_err(|e| e.to_string())?;
    let tasks = crate::db::tasks::get_all_tasks(&conn, None, None, "due_date", "ASC")
        .map_err(|e| e.to_string())?;

    crate::schedule::build_calendar_week(
        &courses,
        &exams,
        &tasks,
        &cmd.semester,
        cmd.week_index,
        cmd.semester_start_date.as_deref(),
        cmd.week_start_date.as_deref(),
    )
}
