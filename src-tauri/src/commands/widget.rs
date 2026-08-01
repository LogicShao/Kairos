use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, EventTarget, LogicalSize, Manager, PhysicalPosition, State};

use crate::db::models::{UpdateWidgetConfigRequest, WidgetConfig};

const WIDGET_LABEL: &str = "widget";
const MAIN_LABEL: &str = "main";
const WIDGET_URL: &str = "index.html?view=widget";
const NAV_TARGETS: [&str; 5] = ["today", "pomodoro", "todo", "courses", "exams"];

#[derive(Debug, Clone, Serialize)]
pub struct MainNavigatePayload {
    /// App.tsx 中的主导航 key。
    pub target: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SaveWidgetPositionRequest {
    /// 窗口左上角物理像素 x 坐标。
    pub x: i64,
    /// 窗口左上角物理像素 y 坐标。
    pub y: i64,
    /// 窗口宽度，逻辑像素。
    pub width: i64,
    /// 窗口高度，逻辑像素。
    pub height: i64,
}

#[tauri::command]
pub fn get_widget_config(db: State<'_, Arc<Mutex<Connection>>>) -> Result<WidgetConfig, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    crate::db::widget::get_or_create_widget_config(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_widget_config(
    db: State<'_, Arc<Mutex<Connection>>>,
    app_handle: AppHandle,
    req: UpdateWidgetConfigRequest,
) -> Result<WidgetConfig, String> {
    let target = {
        let conn = db.lock().map_err(|e| e.to_string())?;
        let current =
            crate::db::widget::get_or_create_widget_config(&conn).map_err(|e| e.to_string())?;
        crate::db::widget::merge_widget_config(&current, &req).map_err(|e| e.to_string())?
    };

    if target.enabled {
        ensure_widget_window(&app_handle, &target)?;
    } else {
        destroy_widget_window(&app_handle)?;
    }

    let saved = { persist_widget_update(db.inner(), &req)? };

    emit_widget_config(&app_handle, &saved);
    Ok(saved)
}

#[tauri::command]
pub fn show_widget(
    db: State<'_, Arc<Mutex<Connection>>>,
    app_handle: AppHandle,
) -> Result<WidgetConfig, String> {
    let req = widget_enabled_update(true);
    update_widget_config(db, app_handle, req)
}

#[tauri::command]
pub fn hide_widget(
    db: State<'_, Arc<Mutex<Connection>>>,
    app_handle: AppHandle,
) -> Result<WidgetConfig, String> {
    destroy_widget_window(&app_handle)?;
    let req = widget_enabled_update(false);
    persist_widget_update(db.inner(), &req)
}

#[tauri::command]
pub fn save_widget_position(
    db: State<'_, Arc<Mutex<Connection>>>,
    request: SaveWidgetPositionRequest,
) -> Result<WidgetConfig, String> {
    let req = widget_position_update(request);
    persist_widget_update(db.inner(), &req)
}

#[tauri::command]
pub fn open_main_window(app_handle: AppHandle, target: Option<String>) -> Result<(), String> {
    let main = app_handle
        .get_webview_window(MAIN_LABEL)
        .ok_or_else(|| "主窗口不存在".to_string())?;
    main.show().map_err(|e| e.to_string())?;
    main.set_focus().map_err(|e| e.to_string())?;

    if let Some(target) = target {
        if !NAV_TARGETS.contains(&target.as_str()) {
            return Err(format!("无效的导航目标：{target}"));
        }
        app_handle
            .emit_to(
                EventTarget::webview_window(MAIN_LABEL),
                "main-navigate",
                MainNavigatePayload { target },
            )
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

pub fn restore_widget_on_startup(
    app_handle: &AppHandle,
    db: &Arc<Mutex<Connection>>,
) -> Result<(), String> {
    let config = {
        let conn = db.lock().map_err(|e| e.to_string())?;
        crate::db::widget::get_or_create_widget_config(&conn).map_err(|e| e.to_string())?
    };

    if config.enabled {
        ensure_widget_window(app_handle, &config)?;
    }

    Ok(())
}

fn ensure_widget_window(app_handle: &AppHandle, config: &WidgetConfig) -> Result<(), String> {
    if let Some(window) = app_handle.get_webview_window(WIDGET_LABEL) {
        apply_widget_window_config(&window, config)?;
        window.show().map_err(|e| e.to_string())?;
        return Ok(());
    }

    let builder = tauri::WebviewWindowBuilder::new(
        app_handle,
        WIDGET_LABEL,
        tauri::WebviewUrl::App(PathBuf::from(WIDGET_URL)),
    )
    .title("Kairos 小组件")
    .decorations(false)
    .transparent(true)
    .resizable(false)
    .skip_taskbar(true)
    .always_on_top(config.always_on_top)
    .inner_size(config.width as f64, config.height as f64)
    .focused(false);

    let window = builder.build().map_err(|e| e.to_string())?;
    apply_widget_window_config(&window, config)
}

fn apply_widget_window_config(
    window: &tauri::WebviewWindow,
    config: &WidgetConfig,
) -> Result<(), String> {
    window
        .set_size(LogicalSize::new(config.width as f64, config.height as f64))
        .map_err(|e| e.to_string())?;
    window
        .set_always_on_top(config.always_on_top)
        .map_err(|e| e.to_string())?;
    window.set_skip_taskbar(true).map_err(|e| e.to_string())?;

    if let (Some(x), Some(y)) = (config.x, config.y) {
        window
            .set_position(PhysicalPosition::new(x as i32, y as i32))
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn destroy_widget_window(app_handle: &AppHandle) -> Result<(), String> {
    if let Some(window) = app_handle.get_webview_window(WIDGET_LABEL) {
        window.destroy().map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn emit_widget_config(app_handle: &AppHandle, config: &WidgetConfig) {
    let _ = app_handle.emit_to(
        EventTarget::webview_window(WIDGET_LABEL),
        "widget-config-updated",
        config,
    );
}

fn persist_widget_update(
    db: &Arc<Mutex<Connection>>,
    req: &UpdateWidgetConfigRequest,
) -> Result<WidgetConfig, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let saved = crate::db::widget::update_widget_config(&conn, req).map_err(|e| e.to_string())?;
    Ok(saved)
}

fn widget_enabled_update(enabled: bool) -> UpdateWidgetConfigRequest {
    UpdateWidgetConfigRequest {
        enabled: Some(enabled),
        ..Default::default()
    }
}

fn widget_position_update(request: SaveWidgetPositionRequest) -> UpdateWidgetConfigRequest {
    UpdateWidgetConfigRequest {
        x: Some(request.x),
        y: Some(request.y),
        width: Some(request.width),
        height: Some(request.height),
        ..Default::default()
    }
}
