use std::sync::{Arc, Mutex};

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::db::models::{PomodoroConfig, UpdatePomodoroConfigRequest};
use crate::timer::{PomodoroEngine, PomodoroState};

/// 按固定顺序（db → engine）加锁，避免死锁；返回两个 guard。
fn lock_db_engine<'a>(
    db: &'a Arc<Mutex<Connection>>,
    engine: &'a Arc<Mutex<PomodoroEngine>>,
) -> Result<
    (
        std::sync::MutexGuard<'a, Connection>,
        std::sync::MutexGuard<'a, PomodoroEngine>,
    ),
    String,
> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let eng = engine.lock().map_err(|e| e.to_string())?;
    Ok((conn, eng))
}

/// Config shape without database `id`, for clean frontend interaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PomodoroConfigData {
    pub work_seconds: i64,
    pub short_break_seconds: i64,
    pub long_break_seconds: i64,
    pub sessions_before_long_break: i64,
    /// 阶段结束后是否自动开始下一阶段计时；false = 暂停等待用户按开始（默认）。
    pub auto_start_next_phase: bool,
}

/// 中断处理请求入参。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvePomodoroInterruptionRequest {
    /// "continue" | "discard" | "complete"
    pub action: String,
}

fn persist_engine_state(conn: &Connection, eng: &PomodoroEngine) -> Result<(), String> {
    let state = eng.get_state();
    crate::db::pomodoro::update_runtime_state(
        conn,
        &state.phase,
        state.remaining_seconds as i64,
        state.total_seconds as i64,
        state.is_running,
        eng.active_session_id,
        eng.interrupted,
    )
    .map_err(|e| e.to_string())
}

/// 从 session 历史重算派生轮数，避免同步导入后内存运行态滞后。
fn refresh_completed_sessions_from_history(
    conn: &Connection,
    eng: &mut PomodoroEngine,
) -> Result<(), String> {
    let completed_sessions = crate::db::pomodoro::count_completed_work_sessions_for_date(conn)
        .map_err(|e| e.to_string())? as u32;
    eng.completed_sessions = completed_sessions;
    Ok(())
}

fn soft_delete_active_session(conn: &Connection, eng: &mut PomodoroEngine) -> Result<(), String> {
    if let Some(session_id) = eng.active_session_id.take() {
        crate::db::pomodoro::soft_delete_session(conn, session_id).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn clear_interruption(eng: &mut PomodoroEngine) {
    eng.interrupted = false;
    eng.interrupted_session_id = None;
}

fn persist_and_cancel_notification(
    conn: std::sync::MutexGuard<'_, Connection>,
    eng: std::sync::MutexGuard<'_, PomodoroEngine>,
) -> Result<(), String> {
    persist_engine_state(&conn, &eng)?;
    drop(conn);
    drop(eng);
    crate::notifications::pomodoro_scheduler::cancel_pomodoro_notification();
    Ok(())
}

#[tauri::command]
pub fn start_pomodoro(
    engine: State<'_, Arc<Mutex<PomodoroEngine>>>,
    db: State<'_, Arc<Mutex<Connection>>>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let (conn, mut eng) = lock_db_engine(&db, &engine)?;

    // 清除中断标记（用户主动开始意味着接受当前状态）
    clear_interruption(&mut eng);

    // work 阶段若没有活跃 session，创建一个
    if eng.phase == crate::timer::TimerPhase::Work && eng.active_session_id.is_none() {
        let now = crate::db::chrono_now();
        let req = crate::db::models::CreatePomodoroSessionRequest {
            started_at: now,
            session_type: "work".to_string(),
            task_id: None,
        };
        let id = crate::db::pomodoro::create_session(&conn, &req).map_err(|e| e.to_string())?;
        eng.active_session_id = Some(id);
    }

    eng.start();

    let state = eng.get_state();
    persist_engine_state(&conn, &eng)?;

    drop(conn);
    drop(eng);

    // Schedule a notification for the current phase end
    crate::notifications::pomodoro_scheduler::schedule_pomodoro_notification(
        &app_handle,
        &state.phase,
        state.remaining_seconds as u64,
    );
    Ok(())
}

#[tauri::command]
pub fn pause_pomodoro(
    engine: State<'_, Arc<Mutex<PomodoroEngine>>>,
    db: State<'_, Arc<Mutex<Connection>>>,
) -> Result<(), String> {
    let (conn, mut eng) = lock_db_engine(&db, &engine)?;
    eng.pause();

    persist_and_cancel_notification(conn, eng)
}

#[tauri::command]
pub fn reset_pomodoro(
    engine: State<'_, Arc<Mutex<PomodoroEngine>>>,
    db: State<'_, Arc<Mutex<Connection>>>,
) -> Result<(), String> {
    let (conn, mut eng) = lock_db_engine(&db, &engine)?;

    // 如果有未结束的活跃 session，软删除
    soft_delete_active_session(&conn, &mut eng)?;

    eng.reset();
    clear_interruption(&mut eng);

    persist_and_cancel_notification(conn, eng)
}

#[tauri::command]
pub fn get_pomodoro_state(
    engine: State<'_, Arc<Mutex<PomodoroEngine>>>,
    db: State<'_, Arc<Mutex<Connection>>>,
) -> Result<PomodoroState, String> {
    let (conn, mut eng) = lock_db_engine(&db, &engine)?;
    refresh_completed_sessions_from_history(&conn, &mut eng)?;
    Ok(eng.get_state())
}

#[tauri::command]
pub fn update_pomodoro_config(
    engine: State<'_, Arc<Mutex<PomodoroEngine>>>,
    db: State<'_, Arc<Mutex<Connection>>>,
    config: PomodoroConfigData,
) -> Result<(), String> {
    let (conn, mut eng) = lock_db_engine(&db, &engine)?;

    soft_delete_active_session(&conn, &mut eng)?;

    let req = UpdatePomodoroConfigRequest {
        work_seconds: config.work_seconds,
        short_break_seconds: config.short_break_seconds,
        long_break_seconds: config.long_break_seconds,
        sessions_before_long_break: config.sessions_before_long_break,
        auto_start_next_phase: config.auto_start_next_phase,
    };
    crate::db::pomodoro::update_config(&conn, &req).map_err(|e| e.to_string())?;

    eng.update_config(PomodoroConfig {
        id: 1,
        work_seconds: config.work_seconds,
        short_break_seconds: config.short_break_seconds,
        long_break_seconds: config.long_break_seconds,
        sessions_before_long_break: config.sessions_before_long_break,
        auto_start_next_phase: config.auto_start_next_phase,
    });
    refresh_completed_sessions_from_history(&conn, &mut eng)?;
    persist_engine_state(&conn, &eng)?;
    crate::notifications::pomodoro_scheduler::cancel_pomodoro_notification();
    Ok(())
}

#[tauri::command]
pub fn get_pomodoro_config(
    db: State<'_, Arc<Mutex<Connection>>>,
) -> Result<PomodoroConfigData, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let config = crate::db::pomodoro::get_config(&conn).map_err(|e| e.to_string())?;
    Ok(PomodoroConfigData {
        work_seconds: config.work_seconds,
        short_break_seconds: config.short_break_seconds,
        long_break_seconds: config.long_break_seconds,
        sessions_before_long_break: config.sessions_before_long_break,
        auto_start_next_phase: config.auto_start_next_phase,
    })
}

/// 处理中断：继续 / 丢弃 / 补记完成。
#[tauri::command]
pub fn resolve_pomodoro_interruption(
    engine: State<'_, Arc<Mutex<PomodoroEngine>>>,
    db: State<'_, Arc<Mutex<Connection>>>,
    request: ResolvePomodoroInterruptionRequest,
) -> Result<PomodoroState, String> {
    let (conn, mut eng) = lock_db_engine(&db, &engine)?;

    match request.action.as_str() {
        "continue" => {
            // 清除中断标记，保持 phase 和剩余时间，保持暂停状态
            clear_interruption(&mut eng);
            eng.is_running = false;
            persist_engine_state(&conn, &eng)?;
        }
        "discard" => {
            soft_delete_active_session(&conn, &mut eng)?;
            clear_interruption(&mut eng);
            eng.reset();
            persist_engine_state(&conn, &eng)?;
        }
        "complete" => {
            if eng.active_session_id.is_none() {
                return Err("没有可补记的专注 session".to_string());
            }
            // 补全 session ended_at
            if let Some(session_id) = eng.active_session_id {
                let now = crate::db::chrono_now();
                crate::db::pomodoro::update_session_end(&conn, session_id, &now)
                    .map_err(|e| e.to_string())?;
            }
            eng.active_session_id = None;
            clear_interruption(&mut eng);
            eng.complete_phase_paused();
            persist_engine_state(&conn, &eng)?;
        }
        _ => return Err(format!("未知操作: {}", request.action)),
    }

    Ok(eng.get_state())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refresh_completed_sessions_from_history_uses_database_source() {
        let conn = crate::db::get_connection(":memory:").expect("Failed to open test DB");
        let config = crate::db::pomodoro::get_config(&conn).expect("Failed to get config");
        let mut eng = PomodoroEngine::new(config);

        let now = crate::db::chrono_now();
        let req = crate::db::models::CreatePomodoroSessionRequest {
            started_at: now.clone(),
            session_type: "work".to_string(),
            task_id: None,
        };
        let session_id =
            crate::db::pomodoro::create_session(&conn, &req).expect("Failed to create session");
        crate::db::pomodoro::update_session_end(&conn, session_id, &now)
            .expect("Failed to end session");

        refresh_completed_sessions_from_history(&conn, &mut eng)
            .expect("Failed to refresh completed sessions");

        assert_eq!(eng.completed_sessions, 1);
    }
}
