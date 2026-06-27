pub mod commands;
pub mod db;
pub mod importers;
pub mod notifications;
pub mod schedule;
pub mod sync;
pub mod timer;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use sync::AutoSyncState;
use tauri::{Emitter, Manager};
use timer::{PomodoroEngine, TimerPhase};

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

            let engine = Arc::new(Mutex::new(PomodoroEngine::new(config)));
            let db_conn = Arc::new(Mutex::new(conn));

            let tick_engine = engine.clone();
            let handle = app.handle().clone();

            std::thread::spawn(move || loop {
                std::thread::sleep(Duration::from_secs(1));

                let phase_change;
                let state;
                {
                    let mut eng = tick_engine.lock().unwrap();
                    phase_change = eng.tick();
                    state = eng.get_state();
                }

                let _ = handle.emit("pomodoro-tick", &state);

                if let Some(ended_phase) = phase_change {
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
                        let (title, body) = match ended_phase {
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

            // ─── 考试通知调度 ───
            if notifications_available {
                let c = db_conn.lock().unwrap();
                let handle = app.handle().clone();
                if let Err(e) =
                    notifications::exam_scheduler::schedule_exam_notifications(&c, &handle)
                {
                    log::error!("failed to schedule exam notifications on startup: {e}");
                }
            }

            // ─── 自动同步状态初始化 ───
            let auto_sync_state = AutoSyncState::new();
            {
                let c = db_conn.lock().unwrap();
                if let Ok(cfg) = db::sync::get_sync_config(&c) {
                    if cfg.auto_sync && !cfg.server_url.is_empty() {
                        let path = db_path_str.clone();
                        let handle = app.handle().clone();
                        sync::spawn_auto_sync_worker(path, &auto_sync_state, handle);
                    }
                }
            }
            app.manage(Arc::new(Mutex::new(auto_sync_state)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::pomodoro::start_pomodoro,
            commands::pomodoro::pause_pomodoro,
            commands::pomodoro::reset_pomodoro,
            commands::pomodoro::get_pomodoro_state,
            commands::pomodoro::get_pomodoro_config,
            commands::pomodoro::update_pomodoro_config,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
