export interface PomodoroState {
  phase: "work" | "short_break" | "long_break"
  remaining_seconds: number
  total_seconds: number
  is_running: boolean
  completed_sessions: number
}

export interface PomodoroConfig {
  work_seconds: number
  short_break_seconds: number
  long_break_seconds: number
  sessions_before_long_break: number
}
