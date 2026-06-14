pub mod commands;
pub mod db;
pub mod timer;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::{Emitter, Manager};
use timer::PomodoroEngine;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
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
            std::fs::create_dir_all(&app_data_dir)
                .expect("failed to create app data dir");

            let db_path = app_data_dir.join("kairos.db");
            let conn = db::get_connection(
                db_path.to_str().expect("invalid db path"),
            )
            .expect("failed to open database connection");

            let config = db::pomodoro::get_config(&conn)
                .expect("failed to load pomodoro config");

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

                if phase_change.is_some() {
                    let _ = handle.emit("pomodoro-phase-change", &state.phase);
                }
            });

            app.manage(db_conn);
            app.manage(engine);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::pomodoro::start_pomodoro,
            commands::pomodoro::pause_pomodoro,
            commands::pomodoro::reset_pomodoro,
            commands::pomodoro::get_pomodoro_state,
            commands::pomodoro::update_pomodoro_config,
            commands::tasks::get_all_tasks,
            commands::tasks::create_task,
            commands::tasks::update_task,
            commands::tasks::delete_task,
            commands::courses::get_all_courses,
            commands::courses::create_course,
            commands::courses::update_course,
            commands::courses::delete_course,
            commands::exams::get_all_exams,
            commands::exams::create_exam,
            commands::exams::update_exam,
            commands::exams::delete_exam,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
