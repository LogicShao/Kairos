pub mod commands;
pub mod db;

use tauri::Manager;

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
      std::fs::create_dir_all(&app_data_dir).expect("failed to create app data dir");
      let db_path = app_data_dir.join("kairos.db");
      let _conn = db::get_connection(
        db_path.to_str().expect("invalid db path"),
      )
      .expect("failed to open database connection");
      Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
