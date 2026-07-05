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
    /// true = 上次退出时计时器正在运行，需用户处理。
    pub interrupted: bool,
    /// 中断对应的活跃 session id（如果有）。
    pub interrupted_session_id: Option<i64>,
    /// 上次运行时记录的时间戳（UTC ISO 8601）。
    pub last_seen_at: Option<String>,
}

/// 应用启动时用于恢复番茄钟引擎的持久化状态。
#[derive(Debug, Clone)]
pub struct RestoredPomodoroState {
    pub phase: TimerPhase,
    pub remaining_seconds: u32,
    pub total_seconds: u32,
    pub was_running: bool,
    pub completed_sessions: u32,
    pub active_session_id: Option<i64>,
    pub last_seen_at: Option<String>,
}

/// 一次阶段切换的完整快照，供外层持久化刚结束的 session。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PomodoroPhaseTransition {
    pub ended_phase: TimerPhase,
    pub next_phase: TimerPhase,
    pub ended_session_id: Option<i64>,
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
    /// true = 上次退出时正在运行，需用户处理中断。
    pub interrupted: bool,
    /// 中断对应的活跃 work session id（如果有）。
    pub interrupted_session_id: Option<i64>,
    /// 上次记录状态的 UTC ISO 8601 时间戳。
    pub last_seen_at: Option<String>,
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
            interrupted: false,
            interrupted_session_id: None,
            last_seen_at: None,
            config,
        }
    }

    /// Restore an engine from persisted state (after app restart).
    ///
    /// If `is_running` was true when persisted, the engine sets
    /// `interrupted = true` and forces `is_running = false`, so the
    /// frontend can prompt the user instead of silently counting offline time.
    pub fn restore(config: PomodoroConfig, restored: RestoredPomodoroState) -> Self {
        let interrupted = restored.was_running;
        Self {
            phase: restored.phase,
            remaining_seconds: restored.remaining_seconds,
            total_seconds: restored.total_seconds,
            is_running: false, // always start paused after restart
            completed_sessions: restored.completed_sessions,
            active_session_id: restored.active_session_id,
            interrupted,
            interrupted_session_id: if interrupted {
                restored.active_session_id
            } else {
                None
            },
            last_seen_at: restored.last_seen_at,
            config,
        }
    }

    /// Set interruption metadata (used after resolve actions).
    pub fn set_interrupted(&mut self, interrupted: bool) {
        self.interrupted = interrupted;
        if !interrupted {
            self.interrupted_session_id = None;
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
    /// Returns `Some(transition)` when the current phase just ended and the engine
    /// auto-switched to the next phase.  Returns `None` otherwise.
    pub fn tick(&mut self) -> Option<PomodoroPhaseTransition> {
        if !self.is_running {
            return None;
        }

        if self.remaining_seconds == 0 {
            return None;
        }

        self.remaining_seconds -= 1;

        if self.remaining_seconds == 0 {
            let ended_phase = self.phase;
            let ended_session_id = self.active_session_id;
            let next_phase = self.advance_phase(true);
            Some(PomodoroPhaseTransition {
                ended_phase,
                next_phase,
                ended_session_id,
            })
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
            interrupted: self.interrupted,
            interrupted_session_id: self.interrupted_session_id,
            last_seen_at: self.last_seen_at.clone(),
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
        self.interrupted = false;
        self.interrupted_session_id = None;
        self.last_seen_at = None;
        let work = seconds_from_config(self.config.work_seconds);
        self.remaining_seconds = work;
        self.total_seconds = work;
    }

    /// 完成当前阶段并切换到下一阶段，但保持暂停状态。
    pub fn complete_phase_paused(&mut self) -> TimerPhase {
        self.advance_phase(false)
    }

    fn advance_phase(&mut self, run_next_phase: bool) -> TimerPhase {
        match self.phase {
            TimerPhase::Work => {
                self.completed_sessions += 1;

                let sessions_before_long = self.config.sessions_before_long_break.max(1) as u32;

                if self.completed_sessions % sessions_before_long == 0 {
                    self.phase = TimerPhase::LongBreak;
                    self.total_seconds = seconds_from_config(self.config.long_break_seconds);
                } else {
                    self.phase = TimerPhase::ShortBreak;
                    self.total_seconds = seconds_from_config(self.config.short_break_seconds);
                }
            }
            TimerPhase::ShortBreak | TimerPhase::LongBreak => {
                self.phase = TimerPhase::Work;
                self.total_seconds = seconds_from_config(self.config.work_seconds);
            }
        }

        self.remaining_seconds = self.total_seconds;
        self.is_running = run_next_phase;
        self.active_session_id = None;
        self.phase
    }
}

pub fn seconds_from_config(val: i64) -> u32 {
    if val < 0 {
        0
    } else {
        val as u32
    }
}

pub fn phase_to_string(phase: TimerPhase) -> String {
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

    fn tick_times(engine: &mut PomodoroEngine, count: usize) -> Option<PomodoroPhaseTransition> {
        let mut last_transition = None;
        for _ in 0..count {
            last_transition = engine.tick();
        }
        last_transition
    }

    fn assert_work_to_short_break(
        result: Option<PomodoroPhaseTransition>,
        ended_session_id: Option<i64>,
    ) {
        assert_eq!(
            result,
            Some(PomodoroPhaseTransition {
                ended_phase: TimerPhase::Work,
                next_phase: TimerPhase::ShortBreak,
                ended_session_id,
            })
        );
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

        let result = tick_times(&mut engine, 3);

        assert_work_to_short_break(result, None);
        assert_eq!(engine.phase, TimerPhase::ShortBreak);
        assert_eq!(engine.remaining_seconds, 1);
        assert_eq!(engine.completed_sessions, 1);
        assert!(engine.is_running);
    }

    #[test]
    fn long_break_after_n_sessions() {
        let mut engine = PomodoroEngine::new(default_config());
        engine.start();

        tick_times(&mut engine, 3);
        assert_eq!(engine.phase, TimerPhase::ShortBreak);
        assert_eq!(engine.completed_sessions, 1);

        tick_times(&mut engine, 1);
        assert_eq!(engine.phase, TimerPhase::Work);
        assert_eq!(engine.remaining_seconds, 3);

        tick_times(&mut engine, 3);
        assert_eq!(engine.phase, TimerPhase::LongBreak);
        assert_eq!(engine.completed_sessions, 2);
        assert_eq!(engine.remaining_seconds, 2);
    }

    #[test]
    fn break_switches_back_to_work() {
        let mut engine = PomodoroEngine::new(default_config());
        engine.start();

        tick_times(&mut engine, 3);
        assert_eq!(engine.phase, TimerPhase::ShortBreak);

        tick_times(&mut engine, 1);
        assert_eq!(engine.phase, TimerPhase::Work);
        assert_eq!(engine.remaining_seconds, 3);
    }

    #[test]
    fn pause_stops_ticking() {
        let mut engine = PomodoroEngine::new(default_config());
        engine.start();
        tick_times(&mut engine, 1);
        assert_eq!(engine.remaining_seconds, 2);

        engine.pause();
        tick_times(&mut engine, 2);
        assert_eq!(engine.remaining_seconds, 2);
        assert!(!engine.is_running);
    }

    #[test]
    fn reset_restores_full_duration() {
        let mut engine = PomodoroEngine::new(default_config());
        engine.start();
        tick_times(&mut engine, 2);
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

    #[test]
    fn every_session_long_break_when_threshold_one() {
        let config = PomodoroConfig {
            id: 1,
            work_seconds: 2,
            short_break_seconds: 1,
            long_break_seconds: 3,
            sessions_before_long_break: 1,
        };
        let mut engine = PomodoroEngine::new(config);
        engine.start();

        tick_times(&mut engine, 2);
        assert_eq!(engine.phase, TimerPhase::LongBreak);
        assert_eq!(engine.remaining_seconds, 3);
        assert_eq!(engine.completed_sessions, 1);
    }

    #[test]
    fn reset_during_break_restores_break_duration() {
        let mut engine = PomodoroEngine::new(default_config());
        engine.start();
        tick_times(&mut engine, 3);
        assert_eq!(engine.phase, TimerPhase::ShortBreak);
        assert_eq!(engine.remaining_seconds, 1);

        engine.reset();
        assert_eq!(engine.phase, TimerPhase::ShortBreak);
        assert_eq!(engine.remaining_seconds, 1);
        assert!(!engine.is_running);
    }

    #[test]
    fn multiple_cycles_track_correct_count() {
        let mut engine = PomodoroEngine::new(default_config());
        engine.start();
        for _ in 0..50 {
            engine.tick();
        }
        assert!(engine.completed_sessions >= 6);
    }

    #[test]
    fn tick_at_zero_no_double_advance() {
        let mut engine = PomodoroEngine::new(default_config());
        engine.start();
        tick_times(&mut engine, 3);
        assert_eq!(engine.phase, TimerPhase::ShortBreak);
        assert_eq!(engine.remaining_seconds, 1);
    }

    #[test]
    fn pause_during_break_and_resume() {
        let mut engine = PomodoroEngine::new(default_config());
        engine.start();
        tick_times(&mut engine, 3);
        assert_eq!(engine.phase, TimerPhase::ShortBreak);
        assert_eq!(engine.remaining_seconds, 1);

        engine.pause();
        tick_times(&mut engine, 1);
        assert_eq!(engine.remaining_seconds, 1);
        assert!(!engine.is_running);

        engine.start();
        tick_times(&mut engine, 1);
        assert_eq!(engine.phase, TimerPhase::Work);
        assert_eq!(engine.remaining_seconds, 3);
    }

    #[test]
    fn get_state_reflects_current_phase() {
        let mut engine = PomodoroEngine::new(default_config());
        engine.start();

        let state = engine.get_state();
        assert_eq!(state.phase, "work");
        assert_eq!(state.remaining_seconds, 3);
        assert_eq!(state.total_seconds, 3);
        assert!(state.is_running);
        assert_eq!(state.completed_sessions, 0);
        assert!(!state.interrupted);
        assert!(state.interrupted_session_id.is_none());
    }

    #[test]
    fn restore_from_paused_state() {
        let config = default_config();
        let engine = PomodoroEngine::restore(
            config,
            RestoredPomodoroState {
                phase: TimerPhase::ShortBreak,
                remaining_seconds: 120,
                total_seconds: 300,
                was_running: false,
                completed_sessions: 3,
                active_session_id: None,
                last_seen_at: Some("2026-06-28T10:00:00Z".to_string()),
            },
        );

        assert_eq!(engine.phase, TimerPhase::ShortBreak);
        assert_eq!(engine.remaining_seconds, 120);
        assert_eq!(engine.total_seconds, 300);
        assert!(!engine.is_running);
        assert_eq!(engine.completed_sessions, 3);
        assert!(!engine.interrupted, "paused → not interrupted");
        assert!(engine.interrupted_session_id.is_none());
    }

    #[test]
    fn restore_from_running_sets_interrupted() {
        let config = default_config();
        let engine = PomodoroEngine::restore(
            config,
            RestoredPomodoroState {
                phase: TimerPhase::Work,
                remaining_seconds: 800,
                total_seconds: 1500,
                was_running: true,
                completed_sessions: 2,
                active_session_id: Some(42),
                last_seen_at: Some("2026-06-28T10:30:00Z".to_string()),
            },
        );

        assert_eq!(engine.phase, TimerPhase::Work);
        assert_eq!(engine.remaining_seconds, 800);
        assert!(!engine.is_running, "restore always starts paused");
        assert!(engine.interrupted, "was running → interrupted");
        assert_eq!(engine.interrupted_session_id, Some(42));
        assert_eq!(engine.completed_sessions, 2);
    }

    #[test]
    fn restore_completed_sessions_preserved() {
        let config = default_config();
        let engine = PomodoroEngine::restore(
            config,
            RestoredPomodoroState {
                phase: TimerPhase::Work,
                remaining_seconds: 1500,
                total_seconds: 1500,
                was_running: false,
                completed_sessions: 5,
                active_session_id: None,
                last_seen_at: None,
            },
        );

        assert_eq!(engine.completed_sessions, 5);
    }

    #[test]
    fn set_interrupted_clears_session_id() {
        let mut engine = PomodoroEngine::new(default_config());
        engine.interrupted = true;
        engine.interrupted_session_id = Some(42);

        engine.set_interrupted(false);
        assert!(!engine.interrupted);
        assert!(engine.interrupted_session_id.is_none());
    }

    #[test]
    fn tick_transition_preserves_ending_session_id() {
        let mut engine = PomodoroEngine::new(default_config());
        engine.active_session_id = Some(99);
        engine.start();

        let result = tick_times(&mut engine, 3);

        assert_work_to_short_break(result, Some(99));
        assert!(engine.active_session_id.is_none());
    }

    #[test]
    fn complete_phase_paused_switches_without_running() {
        let mut engine = PomodoroEngine::new(default_config());
        engine.active_session_id = Some(7);

        let next = engine.complete_phase_paused();

        assert_eq!(next, TimerPhase::ShortBreak);
        assert_eq!(engine.completed_sessions, 1);
        assert_eq!(engine.phase, TimerPhase::ShortBreak);
        assert!(!engine.is_running);
        assert!(engine.active_session_id.is_none());
    }
}
