export interface PomodoroState {
  phase: "work" | "short_break" | "long_break"
  remaining_seconds: number
  total_seconds: number
  is_running: boolean
  completed_sessions: number
}
