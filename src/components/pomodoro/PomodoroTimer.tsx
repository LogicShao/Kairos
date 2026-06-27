import { useState, useEffect, useCallback } from "react"
import { invoke } from "@tauri-apps/api/core"
import { listen, type UnlistenFn } from "@tauri-apps/api/event"
import type { PomodoroState, PomodoroConfig, PomodoroPhase } from "@/types/pomodoro"
import { cn } from "@/lib/utils"
import { Button } from "@/components/ui/button"
import { Stepper } from "@/components/ui/stepper"
import { Modal } from "@/components/shared/modal"
import { PageShell } from "@/components/shared/page-shell"
import { Play, Pause, RotateCcw, Settings } from "lucide-react"

const CIRCUMFERENCE = 2 * Math.PI * 120

const PHASE_LABELS: Record<PomodoroPhase, string> = {
  work: "专注",
  short_break: "短休",
  long_break: "长休",
}

function formatTime(seconds: number): string {
  const m = Math.floor(seconds / 60)
  const s = seconds % 60
  return `${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`
}

const RANGES = {
  work: { min: 1, max: 120, default: 25, step: 5 },
  shortBreak: { min: 1, max: 30, default: 5, step: 1 },
  longBreak: { min: 1, max: 60, default: 15, step: 5 },
  sessions: { min: 1, max: 10, default: 4, step: 1 },
}

export function PomodoroTimer() {
  const [state, setState] = useState<PomodoroState | null>(null)
  const [error, setError] = useState<string | null>(null)

  // Settings modal
  const [settingsOpen, setSettingsOpen] = useState(false)
  const [workMinutes, setWorkMinutes] = useState(RANGES.work.default)
  const [shortBreakMinutes, setShortBreakMinutes] = useState(RANGES.shortBreak.default)
  const [longBreakMinutes, setLongBreakMinutes] = useState(RANGES.longBreak.default)
  const [sessionsBeforeLongBreak, setSessionsBeforeLongBreak] = useState(RANGES.sessions.default)
  const [savingConfig, setSavingConfig] = useState(false)
  const [configLoading, setConfigLoading] = useState(false)

  useEffect(() => {
    let unlistenTick: UnlistenFn | undefined

    async function init() {
      try {
        const initial = await invoke<PomodoroState>("get_pomodoro_state")
        setState(initial)
      } catch {
        setState({
          phase: "work",
          remaining_seconds: 1500,
          total_seconds: 1500,
          is_running: false,
          completed_sessions: 0,
        })
        setError("Tauri 不可用 — 展示离线 UI")
        return
      }

      try {
        unlistenTick = await listen<PomodoroState>("pomodoro-tick", (event) => {
          setState(event.payload)
        })
      } catch {
        setError("无法监听计时器事件")
      }
    }

    init()

    return () => {
      unlistenTick?.()
    }
  }, [])

  const handleStartPause = useCallback(() => {
    if (!state) return
    if (state.is_running) {
      invoke("pause_pomodoro").catch(console.error)
    } else {
      invoke("start_pomodoro").catch(console.error)
    }
  }, [state])

  const handleReset = useCallback(() => {
    invoke("reset_pomodoro").catch(console.error)
  }, [])

  const handleOpenSettings = useCallback(async () => {
    setSettingsOpen(true)
    setConfigLoading(true)
    try {
      const config = await invoke<PomodoroConfig>("get_pomodoro_config")
      setWorkMinutes(Math.floor(config.work_seconds / 60))
      setShortBreakMinutes(Math.floor(config.short_break_seconds / 60))
      setLongBreakMinutes(Math.floor(config.long_break_seconds / 60))
      setSessionsBeforeLongBreak(config.sessions_before_long_break)
    } catch {
      // keep defaults if backend unavailable
    } finally {
      setConfigLoading(false)
    }
  }, [])

  const handleSaveConfig = useCallback(async () => {
    if (
      workMinutes < RANGES.work.min || workMinutes > RANGES.work.max ||
      shortBreakMinutes < RANGES.shortBreak.min || shortBreakMinutes > RANGES.shortBreak.max ||
      longBreakMinutes < RANGES.longBreak.min || longBreakMinutes > RANGES.longBreak.max ||
      sessionsBeforeLongBreak < RANGES.sessions.min || sessionsBeforeLongBreak > RANGES.sessions.max
    ) {
      return
    }

    setSavingConfig(true)
    try {
      await invoke("update_pomodoro_config", {
        config: {
          work_seconds: workMinutes * 60,
          short_break_seconds: shortBreakMinutes * 60,
          long_break_seconds: longBreakMinutes * 60,
          sessions_before_long_break: sessionsBeforeLongBreak,
        },
      })
      setSettingsOpen(false)
      const newState = await invoke<PomodoroState>("get_pomodoro_state")
      setState(newState)
    } catch (e) {
      console.error(e)
    } finally {
      setSavingConfig(false)
    }
  }, [workMinutes, shortBreakMinutes, longBreakMinutes, sessionsBeforeLongBreak])

  if (!state) {
    return (
      <div className="flex items-center justify-center h-96">
        <div className="animate-spin h-8 w-8 border-2 border-primary border-t-transparent rounded-full" />
      </div>
    )
  }

  const progress =
    state.total_seconds > 0
      ? 1 - state.remaining_seconds / state.total_seconds
      : 0
  const offset = CIRCUMFERENCE * (1 - progress)
  const isWork = state.phase === "work"

  const settingsButton = (
    <Button
      variant="ghost"
      size="icon"
      className="rounded-full min-h-11 min-w-11 md:min-h-0 md:min-w-0"
      onClick={handleOpenSettings}
      aria-label="番茄钟设置"
    >
      <Settings className="h-4 w-4" />
    </Button>
  )

  return (
    <PageShell title="专注" width="md" centered action={settingsButton}>
      <div className="flex flex-col items-center gap-6">
      <div className="relative w-64 h-64 md:w-72 md:h-72">
        <svg viewBox="0 0 300 300" className="w-full h-full -rotate-90">
          <circle
            cx="150"
            cy="150"
            r="120"
            fill="none"
            stroke="currentColor"
            strokeWidth="8"
            className="text-muted/30"
          />
          <circle
            cx="150"
            cy="150"
            r="120"
            fill="none"
            stroke="currentColor"
            strokeWidth="8"
            strokeLinecap="round"
            strokeDasharray={CIRCUMFERENCE}
            strokeDashoffset={offset}
            className={cn(
              isWork ? "text-primary" : "text-emerald-400",
              "transition-[stroke-dashoffset] duration-1000 ease-linear",
            )}
            style={{
              filter: `drop-shadow(0 0 7px ${isWork ? "oklch(0.66 0.15 235 / 0.55)" : "oklch(0.72 0.15 160 / 0.55)"})`,
            }}
          />
        </svg>

        <div className="absolute inset-0 flex flex-col items-center justify-center pointer-events-none">
          <span className="text-4xl md:text-5xl font-mono font-medium tabular-nums tracking-tight text-foreground">
            {formatTime(state.remaining_seconds)}
          </span>
          <span
            className={cn(
              "text-sm font-medium mt-1",
              isWork ? "text-primary" : "text-emerald-400",
            )}
          >
            {PHASE_LABELS[state.phase]}
          </span>
          {state.completed_sessions > 0 && (
            <span className="text-xs text-muted-foreground mt-0.5">
              {state.completed_sessions} 次完成
            </span>
          )}
        </div>
      </div>

      {error && (
        <p className="text-xs text-muted-foreground">{error}</p>
      )}

      <div className="grid grid-cols-3 items-center w-full max-w-[18rem] mx-auto">
        {/* 左侧占位 — 保持播放按钮视觉居中 */}
        <div />
        <Button
          size="icon-lg"
          className="rounded-full shadow-lg shadow-primary/25 min-h-11 min-w-11 md:min-h-0 md:min-w-0 justify-self-center"
          onClick={handleStartPause}
          aria-label={state.is_running ? "暂停" : "开始"}
        >
          {state.is_running ? <Pause className="h-5 w-5" /> : <Play className="h-5 w-5" />}
        </Button>
        <Button
          variant="ghost"
          size="icon"
          className="rounded-full min-h-11 min-w-11 md:min-h-0 md:min-w-0 justify-self-center"
          onClick={handleReset}
          aria-label="重置"
        >
          <RotateCcw className="h-4 w-4" />
        </Button>
      </div>

      <Modal
        open={settingsOpen}
        onOpenChange={setSettingsOpen}
        title="番茄钟设置"
        description="自定义工作时长、休息时长和长休间隔"
      >
        {configLoading ? (
          <div className="flex items-center justify-center py-8">
            <div className="animate-spin h-6 w-6 border-2 border-primary border-t-transparent rounded-full" />
          </div>
        ) : (
          <div className="space-y-4">
            <div className="space-y-3">
              <div className="flex items-center justify-between">
                <span className="text-sm text-foreground">工作时长</span>
                <Stepper value={workMinutes} onChange={setWorkMinutes} min={RANGES.work.min} max={RANGES.work.max} step={RANGES.work.step} />
              </div>
              <div className="flex items-center justify-between">
                <span className="text-sm text-foreground">短休时长</span>
                <Stepper value={shortBreakMinutes} onChange={setShortBreakMinutes} min={RANGES.shortBreak.min} max={RANGES.shortBreak.max} step={RANGES.shortBreak.step} />
              </div>
              <div className="flex items-center justify-between">
                <span className="text-sm text-foreground">长休时长</span>
                <Stepper value={longBreakMinutes} onChange={setLongBreakMinutes} min={RANGES.longBreak.min} max={RANGES.longBreak.max} step={RANGES.longBreak.step} />
              </div>
              <div className="flex items-center justify-between">
                <span className="text-sm text-foreground">长休前番茄数</span>
                <Stepper value={sessionsBeforeLongBreak} onChange={setSessionsBeforeLongBreak} min={RANGES.sessions.min} max={RANGES.sessions.max} step={RANGES.sessions.step} />
              </div>
            </div>

            <div className="flex items-center justify-end gap-2 pt-1">
              <Button
                type="button"
                variant="ghost"
                size="sm"
                onClick={() => setSettingsOpen(false)}
              >
                取消
              </Button>
              <Button
                type="button"
                size="sm"
                disabled={savingConfig || configLoading}
                onClick={handleSaveConfig}
              >
                {savingConfig ? "保存中..." : "保存"}
              </Button>
            </div>
          </div>
        )}
      </Modal>
      </div>
    </PageShell>
  )
}
