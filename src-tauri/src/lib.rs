pub mod commands;
pub mod db;
pub mod importers;
pub mod lzu;
pub mod notifications;
pub mod schedule;
pub mod sync;
pub mod timer;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use sync::AutoSyncState;
use tauri::{Emitter, Manager};
use timer::{PomodoroEngine, RestoredPomodoroState, TimerPhase};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            app.handle()
                .plugin(tauri_plugin_clipboard_manager::init())?;
            let notifications_available = app
                .handle()
                .plugin(tauri_plugin_notification::init())
                .is_ok();
            if notifications_available {
                notifications::mark_available();
            } else {
                log::warn!("notification plugin init failed — notifications disabled");
            }
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("failed to resolve app data dir");
            std::fs::create_dir_all(&app_data_dir).expect("failed to create app data dir");

            let db_path = app_data_dir.join("kairos.db");
            let db_path_str = db_path.to_str().expect("invalid db path").to_string();
            let conn =
                db::get_connection(&db_path_str).expect("failed to open database connection");

            let config = db::pomodoro::get_config(&conn).expect("failed to load pomodoro config");

            // ─── 恢复番茄钟运行态 ───
            let engine = {
                let runtime_state = db::pomodoro::get_or_create_runtime_state(&conn)
                    .expect("failed to load pomodoro runtime state");

                let today_key = db::pomodoro::today_local_date();
                let completed = db::pomodoro::count_completed_work_sessions_for_date(&conn)
                    .expect("failed to count today's sessions");

                // 如果 date_key 不是今天，状态被跨天视为过期
                if runtime_state.date_key != today_key {
                    Arc::new(Mutex::new(PomodoroEngine::new(config)))
                } else {
                    // 查询孤儿 session（crash 后可能未保存到 runtime_state）
                    let active_session_id = if runtime_state.active_session_id.is_some() {
                        runtime_state.active_session_id
                    } else {
                        db::pomodoro::find_latest_open_work_session(&conn)
                            .expect("failed to find open session")
                    };

                    let phase = match runtime_state.phase.as_str() {
                        "work" => TimerPhase::Work,
                        "short_break" => TimerPhase::ShortBreak,
                        "long_break" => TimerPhase::LongBreak,
                        _ => TimerPhase::Work,
                    };

                    let remaining = runtime_state.remaining_seconds.max(0) as u32;
                    let total = runtime_state.total_seconds.max(0) as u32;

                    Arc::new(Mutex::new(PomodoroEngine::restore(
                        config,
                        RestoredPomodoroState {
                            phase,
                            remaining_seconds: remaining,
                            total_seconds: total,
                            was_running: runtime_state.is_running,
                            completed_sessions: completed as u32,
                            active_session_id,
                            last_seen_at: Some(runtime_state.last_seen_at.clone()),
                        },
                    )))
                }
            };
            let db_conn = Arc::new(Mutex::new(conn));

            let tick_engine = engine.clone();
            let tick_db = db_conn.clone();
            let handle = app.handle().clone();

            std::thread::spawn(move || loop {
                std::thread::sleep(Duration::from_secs(1));

                let phase_change;
                let state;
                {
                    let Ok(mut eng) = tick_engine.lock() else {
                        log::error!("failed to lock pomodoro engine; stopping tick worker");
                        break;
                    };
                    phase_change = eng.tick();
                    state = eng.get_state();
                }

                let _ = handle.emit("pomodoro-tick", &state);

                if let Ok(conn) = tick_db.lock() {
                    if let Some(transition) = phase_change {
                        if transition.ended_phase == TimerPhase::Work {
                            if let Some(session_id) = transition.ended_session_id {
                                let now = db::chrono_now();
                                if let Err(e) =
                                    db::pomodoro::update_session_end(&conn, session_id, &now)
                                {
                                    log::error!("failed to end pomodoro session: {e}");
                                }
                            }
                        }

                        if transition.next_phase == TimerPhase::Work && state.is_running {
                            match tick_engine.lock() {
                                Ok(mut eng) => {
                                    if eng.active_session_id.is_none() {
                                        let req = db::models::CreatePomodoroSessionRequest {
                                            started_at: db::chrono_now(),
                                            session_type: "work".to_string(),
                                            task_id: None,
                                        };
                                        match db::pomodoro::create_session(&conn, &req) {
                                            Ok(id) => {
                                                eng.active_session_id = Some(id);
                                            }
                                            Err(e) => {
                                                log::error!(
                                                    "failed to create pomodoro session: {e}"
                                                );
                                            }
                                        }
                                    }
                                }
                                Err(e) => log::error!("failed to lock pomodoro engine: {e}"),
                            }
                        }
                    }

                    let persisted = match tick_engine.lock() {
                        Ok(eng) => Some((eng.active_session_id, eng.interrupted)),
                        Err(e) => {
                            log::error!("failed to lock pomodoro engine: {e}");
                            None
                        }
                    };
                    if let Some((active_session_id, interrupted)) = persisted {
                        if let Err(e) = db::pomodoro::update_runtime_state(
                            &conn,
                            &state.phase,
                            state.remaining_seconds as i64,
                            state.total_seconds as i64,
                            state.is_running,
                            active_session_id,
                            interrupted,
                        ) {
                            log::error!("failed to persist pomodoro runtime state: {e}");
                        }
                    }
                }

                if let Some(transition) = phase_change {
                    let _ = handle.emit("pomodoro-phase-change", &state.phase);

                    if notifications_available {
                        // Cancel the notification for the just-ended phase
                        notifications::pomodoro_scheduler::cancel_pomodoro_notification();

                        // Schedule a notification for the new phase ending
                        notifications::pomodoro_scheduler::schedule_pomodoro_notification(
                            &handle,
                            &state.phase,
                            state.remaining_seconds as u64,
                        );

                        // Send an immediate notification about the phase change
                        let (title, body) = match transition.ended_phase {
                            TimerPhase::Work => ("番茄钟", "专注时间结束！休息一下吧"),
                            TimerPhase::ShortBreak => ("番茄钟", "短休息结束！开始专注吧"),
                            TimerPhase::LongBreak => ("番茄钟", "长休息结束！开始专注吧"),
                        };
                        notifications::pomodoro_scheduler::send_immediate_notification(
                            &handle, title, body,
                        );
                    }
                }
            });

            app.manage(db_conn.clone());
            app.manage(engine);

            if let Err(e) = commands::widget::restore_widget_on_startup(app.handle(), &db_conn) {
                log::error!("failed to restore widget window on startup: {e}");
            }

            // ─── 考试通知调度 ───
            if notifications_available {
                match db_conn.lock() {
                    Ok(c) => {
                        let handle = app.handle().clone();
                        if let Err(e) =
                            notifications::exam_scheduler::schedule_exam_notifications(&c, &handle)
                        {
                            log::error!("failed to schedule exam notifications on startup: {e}");
                        }
                    }
                    Err(e) => log::error!("failed to lock database for exam notifications: {e}"),
                }
            }

            // ─── 自动同步状态初始化 ───
            let auto_sync_state = AutoSyncState::new();
            {
                match db_conn.lock() {
                    Ok(c) => {
                        if let Ok(cfg) = db::sync::get_sync_config(&c) {
                            if cfg.auto_sync && !cfg.server_url.is_empty() {
                                let path = db_path_str.clone();
                                let handle = app.handle().clone();
                                sync::spawn_auto_sync_worker(path, &auto_sync_state, handle);
                            }
                        }
                    }
                    Err(e) => log::error!("failed to lock database for auto sync startup: {e}"),
                }
            }
            app.manage(Arc::new(Mutex::new(auto_sync_state)));

            // ─── LZU 认证状态初始化 ───
            let lzu_auth =
                crate::lzu::auth::create_shared_auth().expect("failed to create LZU auth manager");
            app.manage(lzu_auth);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::pomodoro::start_pomodoro,
            commands::pomodoro::pause_pomodoro,
            commands::pomodoro::reset_pomodoro,
            commands::pomodoro::get_pomodoro_state,
            commands::pomodoro::get_pomodoro_config,
            commands::pomodoro::update_pomodoro_config,
            commands::pomodoro::resolve_pomodoro_interruption,
            commands::tasks::get_all_tasks,
            commands::tasks::create_task,
            commands::tasks::update_task,
            commands::tasks::delete_task,
            commands::courses::get_all_courses,
            commands::courses::create_course,
            commands::courses::update_course,
            commands::courses::delete_course,
            commands::courses::reset_all_semester_start_dates,
            commands::courses::import_courses_from_text,
            commands::exams::get_all_exams,
            commands::exams::create_exam,
            commands::exams::update_exam,
            commands::exams::delete_exam,
            commands::exams::import_exams_from_text,
            commands::schedule::get_week_schedule,
            commands::schedule::get_calendar_week,
            commands::sync::get_sync_config,
            commands::sync::update_sync_config,
            commands::sync::test_sync_connection,
            commands::sync::sync_now,
            commands::notifications::get_notification_config,
            commands::notifications::update_notification_config,
            commands::notifications::request_notification_permission,
            commands::widget::get_widget_config,
            commands::widget::update_widget_config,
            commands::widget::show_widget,
            commands::widget::hide_widget,
            commands::widget::save_widget_position,
            commands::widget::open_main_window,
            commands::lzu::lzu_login,
            commands::lzu::lzu_logout,
            commands::lzu::lzu_get_auth_status,
            commands::lzu::lzu_refresh_profile,
            commands::lzu::lzu_refresh_st,
            commands::lzu::lzu_get_campus_card_overview,
            commands::lzu::lzu_get_service_directory,
            commands::lzu::import_lzu_courses,
            commands::briefing::get_today_briefing,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
