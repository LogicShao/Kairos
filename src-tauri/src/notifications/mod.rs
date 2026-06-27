pub mod exam_scheduler;
pub mod ids;
pub mod pomodoro_scheduler;

use std::sync::atomic::{AtomicBool, Ordering};

/// Set to `true` after `tauri_plugin_notification::init()` succeeds.
/// All notification functions check this flag before attempting to show
/// notifications, preventing crashes when the plugin init failed.
static NOTIFICATIONS_AVAILABLE: AtomicBool = AtomicBool::new(false);

pub fn mark_available() {
    NOTIFICATIONS_AVAILABLE.store(true, Ordering::Release);
}

pub fn is_available() -> bool {
    NOTIFICATIONS_AVAILABLE.load(Ordering::Acquire)
}
