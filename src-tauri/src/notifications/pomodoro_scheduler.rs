use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

use super::ids;

// ── Global cancellation token ───────────────────────────────────────────────────
//
// Only ONE pomodoro notification is active at a time (for the current phase end).
// The token is replaced when a new phase starts or when cancelled.

fn current_cancel_token() -> &'static Arc<Mutex<Option<Arc<AtomicBool>>>> {
    static TOKEN: OnceLock<Arc<Mutex<Option<Arc<AtomicBool>>>>> = OnceLock::new();
    TOKEN.get_or_init(|| Arc::new(Mutex::new(None)))
}

// ── Helpers ─────────────────────────────────────────────────────────────────────

fn phase_name_cn(phase: &str) -> &str {
    match phase {
        "work" => "专注",
        "short_break" => "短休息",
        "long_break" => "长休息",
        _ => phase,
    }
}

// ── Public API ──────────────────────────────────────────────────────────────────

/// Schedule a notification for the current pomodoro phase end.
///
/// Spawns a background thread that sleeps for `remaining_seconds` then shows a
/// system notification. Only one pomodoro notification is active at a time;
/// calling this replaces any previously scheduled notification.
pub fn schedule_pomodoro_notification(app_handle: &AppHandle, phase: &str, remaining_seconds: u64) {
    if !super::is_available() {
        return;
    }
    if remaining_seconds == 0 {
        log::debug!("pomodoro: skipping notification schedule — 0 remaining seconds");
        return;
    }

    let target_epoch = chrono::Utc::now().timestamp() as u64 + remaining_seconds;
    let id = ids::stable_id(&format!("pomodoro:{phase}:{target_epoch}"));

    let cancel_token = Arc::new(AtomicBool::new(false));

    // Replace any existing token (cancel the previous one first)
    {
        let mut guard = current_cancel_token().lock().unwrap();
        if let Some(ref old) = *guard {
            old.store(true, Ordering::SeqCst);
        }
        *guard = Some(cancel_token.clone());
    }

    let app_handle = app_handle.clone();
    let cn_name = phase_name_cn(phase).to_string();

    thread::spawn(move || {
        thread::sleep(Duration::from_secs(remaining_seconds));

        if cancel_token.load(Ordering::SeqCst) {
            log::debug!("pomodoro notification cancelled before show");
            return;
        }

        let body = format!("番茄钟「{cn_name}」阶段已结束");
        if let Err(e) = app_handle
            .notification()
            .builder()
            .id(id)
            .title("番茄钟")
            .body(body)
            .show()
        {
            log::error!("failed to show pomodoro notification: {e}");
        }

        // Clean up token after showing
        let mut guard = current_cancel_token().lock().unwrap();
        if let Some(ref token) = *guard {
            if Arc::ptr_eq(token, &cancel_token) {
                *guard = None;
            }
        }
    });
}

/// Cancel the currently scheduled pomodoro notification.
///
/// Sets the cancel flag so the background thread will skip showing the
/// notification. Safe to call even if no notification is currently scheduled.
pub fn cancel_pomodoro_notification() {
    let mut guard = current_cancel_token().lock().unwrap();
    if let Some(ref token) = *guard {
        token.store(true, Ordering::SeqCst);
    }
    *guard = None;
}

/// Send an immediate (non-scheduled) system notification.
///
/// Useful for phase-change instant notifications (e.g., "Focus time is over!").
pub fn send_immediate_notification(app_handle: &AppHandle, title: &str, body: &str) {
    if !super::is_available() {
        return;
    }
    let app_handle = app_handle.clone();
    let title = title.to_string();
    let body = body.to_string();
    let id = ids::stable_id(&format!("pomodoro:immediate:{title}:{body}"));

    thread::spawn(move || {
        if let Err(e) = app_handle
            .notification()
            .builder()
            .id(id)
            .title(title)
            .body(body)
            .show()
        {
            log::error!("failed to show immediate notification: {e}");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase_name_cn_known() {
        assert_eq!(phase_name_cn("work"), "专注");
        assert_eq!(phase_name_cn("short_break"), "短休息");
        assert_eq!(phase_name_cn("long_break"), "长休息");
    }

    #[test]
    fn test_phase_name_cn_unknown_fallback() {
        assert_eq!(phase_name_cn("unknown_phase"), "unknown_phase");
    }

    #[test]
    fn test_cancel_noop_when_none() {
        // Just verifies no panic when nothing is scheduled
        cancel_pomodoro_notification();
    }

    #[test]
    fn test_schedule_then_cancel() {
        // Verify cancel replaces token correctly
        let token = Arc::new(AtomicBool::new(false));
        {
            let mut guard = current_cancel_token().lock().unwrap();
            *guard = Some(token.clone());
        }
        assert!(!token.load(Ordering::SeqCst));
        cancel_pomodoro_notification();
        assert!(token.load(Ordering::SeqCst));

        let guard = current_cancel_token().lock().unwrap();
        assert!(guard.is_none());
    }
}
