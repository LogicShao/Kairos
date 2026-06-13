use serde::Serialize;

use crate::db::models::PomodoroConfig;

/// The current phase of the pomodoro timer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum TimerPhase {
    #[serde(rename = "work")]
    Work,
    #[serde(rename = "short_break")]
    ShortBreak,
    #[serde(rename = "long_break")]
    LongBreak,
}

/// Serializable snapshot of the timer state, sent to the frontend via IPC.
#[derive(Debug, Clone, Serialize)]
pub struct PomodoroState {
    pub phase: String,
    pub remaining_seconds: u32,
    pub total_seconds: u32,
    pub is_running: bool,
    pub completed_sessions: u32,
}

/// The core pomodoro timer engine.
///
/// Manages phase transitions, countdown, and completion tracking.
/// Does NOT spawn its own thread — external code calls [`tick`] on a regular
/// interval and emits events based on the return value.
pub struct PomodoroEngine {
    pub phase: TimerPhase,
    pub remaining_seconds: u32,
    pub total_seconds: u32,
    pub is_running: bool,
    pub completed_sessions: u32,
    /// ID of the currently active pomodoro session in the database (if any).
    /// Set externally by the command / tick loop when a work session begins.
    pub active_session_id: Option<i64>,
    config: PomodoroConfig,
}

impl PomodoroEngine {
    /// Create a new engine initialised in the Work phase with the given config.
    pub fn new(config: PomodoroConfig) -> Self {
        let work_seconds = seconds_from_config(config.work_seconds);
        Self {
            phase: TimerPhase::Work,
            remaining_seconds: work_seconds,
            total_seconds: work_seconds,
            is_running: false,
            completed_sessions: 0,
            active_session_id: None,
            config,
        }
    }

    /// Start (or resume) the countdown.
    pub fn start(&mut self) {
        self.is_running = true;
    }

    /// Pause the countdown.  Does not reset progress.
    pub fn pause(&mut self) {
        self.is_running = false;
    }

    /// Reset the **current** phase back to its full configured duration and
    /// stop the timer.  Does NOT change the phase or the completed-session
    /// counter.
    pub fn reset(&mut self) {
        self.is_running = false;
        self.remaining_seconds = self.total_seconds;
        self.active_session_id = None;
    }

    /// Advance the timer by one tick (≈1 second).
    ///
    /// Returns `Some(phase)` when the current phase just ended and the engine
    /// auto-switched to the next phase.  Returns `None` otherwise.
    pub fn tick(&mut self) -> Option<TimerPhase> {
        if !self.is_running {
            return None;
        }

        if self.remaining_seconds == 0 {
            return None;
        }

        self.remaining_seconds -= 1;

        if self.remaining_seconds == 0 {
            let ended_phase = self.phase;
            self.switch_phase();
            Some(ended_phase)
        } else {
            None
        }
    }

    /// Return a read-only snapshot suitable for sending to the frontend.
    pub fn get_state(&self) -> PomodoroState {
        PomodoroState {
            phase: phase_to_string(self.phase),
            remaining_seconds: self.remaining_seconds,
            total_seconds: self.total_seconds,
            is_running: self.is_running,
            completed_sessions: self.completed_sessions,
        }
    }

    /// Return a reference to the current config.
    pub fn get_config(&self) -> &PomodoroConfig {
        &self.config
    }

    /// Replace the config.
    ///
    /// Resets the current phase to the start of a Work phase using the new
    /// config values and pauses the timer.
    pub fn update_config(&mut self, config: PomodoroConfig) {
        self.config = config;
        self.phase = TimerPhase::Work;
        self.is_running = false;
        self.completed_sessions = 0;
        self.active_session_id = None;
        let work = seconds_from_config(self.config.work_seconds);
        self.remaining_seconds = work;
        self.total_seconds = work;
    }

    fn switch_phase(&mut self) {
        match self.phase {
            TimerPhase::Work => {
                self.completed_sessions += 1;

                let sessions_before_long =
                    self.config.sessions_before_long_break.max(1) as u32;

                if self.completed_sessions % sessions_before_long == 0 {
                    self.phase = TimerPhase::LongBreak;
                    self.total_seconds =
                        seconds_from_config(self.config.long_break_seconds);
                } else {
                    self.phase = TimerPhase::ShortBreak;
                    self.total_seconds =
                        seconds_from_config(self.config.short_break_seconds);
                }
            }
            TimerPhase::ShortBreak | TimerPhase::LongBreak => {
                self.phase = TimerPhase::Work;
                self.total_seconds =
                    seconds_from_config(self.config.work_seconds);
            }
        }

        self.remaining_seconds = self.total_seconds;
        self.is_running = true;
        self.active_session_id = None;
    }
}

fn seconds_from_config(val: i64) -> u32 {
    if val < 0 {
        0
    } else {
        val as u32
    }
}

fn phase_to_string(phase: TimerPhase) -> String {
    match phase {
        TimerPhase::Work => "work".to_string(),
        TimerPhase::ShortBreak => "short_break".to_string(),
        TimerPhase::LongBreak => "long_break".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> PomodoroConfig {
        PomodoroConfig {
            id: 1,
            work_seconds: 3,
            short_break_seconds: 1,
            long_break_seconds: 2,
            sessions_before_long_break: 2,
        }
    }

    #[test]
    fn new_engine_starts_in_work_paused() {
        let engine = PomodoroEngine::new(default_config());
        assert_eq!(engine.phase, TimerPhase::Work);
        assert!(!engine.is_running);
        assert_eq!(engine.remaining_seconds, 3);
        assert_eq!(engine.total_seconds, 3);
        assert_eq!(engine.completed_sessions, 0);
    }

    #[test]
    fn tick_when_paused_does_nothing() {
        let mut engine = PomodoroEngine::new(default_config());
        let result = engine.tick();
        assert!(result.is_none());
        assert_eq!(engine.remaining_seconds, 3);
    }

    #[test]
    fn start_and_tick_decrements() {
        let mut engine = PomodoroEngine::new(default_config());
        engine.start();
        assert!(engine.is_running);

        engine.tick();
        assert_eq!(engine.remaining_seconds, 2);
        engine.tick();
        assert_eq!(engine.remaining_seconds, 1);
    }

    #[test]
    fn tick_returns_phase_on_expiry() {
        let mut engine = PomodoroEngine::new(default_config());
        engine.start();

        engine.tick();
        engine.tick();
        let result = engine.tick();

        assert_eq!(result, Some(TimerPhase::Work));
        assert_eq!(engine.phase, TimerPhase::ShortBreak);
        assert_eq!(engine.remaining_seconds, 1);
        assert_eq!(engine.completed_sessions, 1);
        assert!(engine.is_running);
    }

    #[test]
    fn long_break_after_n_sessions() {
        let mut engine = PomodoroEngine::new(default_config());
        engine.start();

        engine.tick();
        engine.tick();
        engine.tick();
        assert_eq!(engine.phase, TimerPhase::ShortBreak);
        assert_eq!(engine.completed_sessions, 1);

        engine.tick();
        assert_eq!(engine.phase, TimerPhase::Work);
        assert_eq!(engine.remaining_seconds, 3);

        engine.tick();
        engine.tick();
        engine.tick();
        assert_eq!(engine.phase, TimerPhase::LongBreak);
        assert_eq!(engine.completed_sessions, 2);
        assert_eq!(engine.remaining_seconds, 2);
    }

    #[test]
    fn break_switches_back_to_work() {
        let mut engine = PomodoroEngine::new(default_config());
        engine.start();

        engine.tick();
        engine.tick();
        engine.tick();
        assert_eq!(engine.phase, TimerPhase::ShortBreak);

        engine.tick();
        assert_eq!(engine.phase, TimerPhase::Work);
        assert_eq!(engine.remaining_seconds, 3);
    }

    #[test]
    fn pause_stops_ticking() {
        let mut engine = PomodoroEngine::new(default_config());
        engine.start();
        engine.tick();
        assert_eq!(engine.remaining_seconds, 2);

        engine.pause();
        engine.tick();
        engine.tick();
        assert_eq!(engine.remaining_seconds, 2);
        assert!(!engine.is_running);
    }

    #[test]
    fn reset_restores_full_duration() {
        let mut engine = PomodoroEngine::new(default_config());
        engine.start();
        engine.tick();
        engine.tick();
        assert_eq!(engine.remaining_seconds, 1);

        engine.reset();
        assert_eq!(engine.remaining_seconds, 3);
        assert!(!engine.is_running);
    }

    #[test]
    fn update_config_resets_to_work() {
        let mut engine = PomodoroEngine::new(default_config());
        engine.start();
        engine.tick();

        let new_config = PomodoroConfig {
            id: 1,
            work_seconds: 10,
            short_break_seconds: 2,
            long_break_seconds: 5,
            sessions_before_long_break: 3,
        };
        engine.update_config(new_config);

        assert_eq!(engine.phase, TimerPhase::Work);
        assert_eq!(engine.remaining_seconds, 10);
        assert_eq!(engine.total_seconds, 10);
        assert!(!engine.is_running);
        assert_eq!(engine.completed_sessions, 0);
    }

    #[test]
    fn negative_config_values_clamped_to_zero() {
        let config = PomodoroConfig {
            id: 1,
            work_seconds: -5,
            short_break_seconds: -1,
            long_break_seconds: -3,
            sessions_before_long_break: 2,
        };
        let engine = PomodoroEngine::new(config);
        assert_eq!(engine.remaining_seconds, 0);
    }
}
