use std::sync::{Arc, Mutex};

use rusqlite::Connection;
use tauri::{AppHandle, State};

use crate::db::models::{
    CreatePomodoroProfileRequest, CreateTermPhaseRequest, PomodoroProfile, SemesterContext,
    TermPhase, UpdatePomodoroProfileRequest, UpdateTermPhaseRequest,
};
use crate::term_phase::CurrentPhaseStatus;

#[tauri::command]
pub fn get_current_phase_status(
    db: State<'_, Arc<Mutex<Connection>>>,
    source: Option<String>,
) -> Result<CurrentPhaseStatus, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    crate::term_phase::get_current_phase_status(
        &conn,
        source
            .as_deref()
            .unwrap_or(crate::db::semester::LZU_SEMESTER_CONTEXT_SOURCE),
    )
}

#[tauri::command]
pub fn get_term_phases(
    db: State<'_, Arc<Mutex<Connection>>>,
    term_label: String,
) -> Result<Vec<TermPhase>, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    crate::db::term_phases::get_term_phases(&conn, term_label.trim()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_semester_contexts(
    db: State<'_, Arc<Mutex<Connection>>>,
) -> Result<Vec<SemesterContext>, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    crate::db::semester::get_all_semester_contexts(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_term_phase(
    app_handle: AppHandle,
    db: State<'_, Arc<Mutex<Connection>>>,
    req: CreateTermPhaseRequest,
) -> Result<i64, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let id = crate::db::term_phases::create_term_phase(&conn, &req).map_err(|e| e.to_string())?;
    reschedule_exam_notifications(&conn, &app_handle)?;
    Ok(id)
}

#[tauri::command]
pub fn update_term_phase(
    app_handle: AppHandle,
    db: State<'_, Arc<Mutex<Connection>>>,
    id: i64,
    req: UpdateTermPhaseRequest,
) -> Result<(), String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    crate::db::term_phases::update_term_phase(&conn, id, &req).map_err(|e| e.to_string())?;
    reschedule_exam_notifications(&conn, &app_handle)
}

#[tauri::command]
pub fn delete_term_phase(
    app_handle: AppHandle,
    db: State<'_, Arc<Mutex<Connection>>>,
    id: i64,
) -> Result<(), String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    crate::db::term_phases::soft_delete_term_phase(&conn, id).map_err(|e| e.to_string())?;
    reschedule_exam_notifications(&conn, &app_handle)
}

#[tauri::command]
pub fn get_pomodoro_profiles(
    db: State<'_, Arc<Mutex<Connection>>>,
) -> Result<Vec<PomodoroProfile>, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    crate::db::pomodoro_profiles::get_all_pomodoro_profiles(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_pomodoro_profile(
    db: State<'_, Arc<Mutex<Connection>>>,
    req: CreatePomodoroProfileRequest,
) -> Result<i64, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    crate::db::pomodoro_profiles::create_pomodoro_profile(&conn, &req).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_pomodoro_profile(
    db: State<'_, Arc<Mutex<Connection>>>,
    id: i64,
    req: UpdatePomodoroProfileRequest,
) -> Result<(), String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    crate::db::pomodoro_profiles::update_pomodoro_profile(&conn, id, &req)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_pomodoro_profile(
    db: State<'_, Arc<Mutex<Connection>>>,
    id: i64,
) -> Result<(), String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    crate::db::pomodoro_profiles::delete_pomodoro_profile(&conn, id).map_err(|e| e.to_string())
}

fn reschedule_exam_notifications(conn: &Connection, app_handle: &AppHandle) -> Result<(), String> {
    crate::notifications::exam_scheduler::schedule_exam_notifications(conn, app_handle)
}
