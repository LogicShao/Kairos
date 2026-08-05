//! 应用级命令：退出应用。
//!
//! 仅 Android 端返回键在无上级页面（导航栈为空）时调用；
//! 桌面端不注册监听，此命令不会被调用（注册也无副作用）。

use tauri::AppHandle;

/// 退出应用。
///
/// 触发 [`RunEvent::ExitRequested`] / [`RunEvent::Exit`] 完成退出；
/// 事件投递失败时 tauri 内部回退 `std::process::exit`，保证必然退出。
#[tauri::command]
pub fn exit_app(app: AppHandle) {
    app.exit(0);
}
