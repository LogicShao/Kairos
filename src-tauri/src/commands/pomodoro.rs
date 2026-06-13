use std::sync::{Arc, Mutex};

use rusqlite::Connection;
use tauri::State;

use crate::db::models::{PomodoroConfig, UpdatePomodoroConfigRequest};
use crate::timer::{PomodoroEngine, PomodoroState};

#[tauri::command]
pub fn start_pomodoro(
    engine: State<'_, Arc<Mutex<PomodoroEngine>>>,
) -> Result<(), String> {
    let mut eng = engine.lock().map_err(|e| e.to_string())?;
    eng.start();
    Ok(())
}

#[tauri::command]
pub fn pause_pomodoro(
    engine: State<'_, Arc<Mutex<PomodoroEngine>>>,
) -> Result<(), String> {
    let mut eng = engine.lock().map_err(|e| e.to_string())?;
    eng.pause();
    Ok(())
}

#[tauri::command]
pub fn reset_pomodoro(
    engine: State<'_, Arc<Mutex<PomodoroEngine>>>,
) -> Result<(), String> {
    let mut eng = engine.lock().map_err(|e| e.to_string())?;
    eng.reset();
    Ok(())
}

#[tauri::command]
pub fn get_pomodoro_state(
    engine: State<'_, Arc<Mutex<PomodoroEngine>>>,
) -> Result<PomodoroState, String> {
    let eng = engine.lock().map_err(|e| e.to_string())?;
    Ok(eng.get_state())
}

#[tauri::command]
pub fn update_pomodoro_config(
    engine: State<'_, Arc<Mutex<PomodoroEngine>>>,
    db: State<'_, Arc<Mutex<Connection>>>,
    config: PomodoroConfig,
) -> Result<(), String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let req = UpdatePomodoroConfigRequest {
        work_seconds: config.work_seconds,
        short_break_seconds: config.short_break_seconds,
        long_break_seconds: config.long_break_seconds,
        sessions_before_long_break: config.sessions_before_long_break,
    };
    crate::db::pomodoro::update_config(&conn, &req).map_err(|e| e.to_string())?;
    drop(conn);

    let mut eng = engine.lock().map_err(|e| e.to_string())?;
    eng.update_config(config);
    Ok(())
}
