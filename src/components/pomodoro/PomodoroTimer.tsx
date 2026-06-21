import { useState, useEffect, useCallback } from "react"
import { invoke } from "@tauri-apps/api/core"
import { listen, type UnlistenFn } from "@tauri-apps/api/event"
import type { PomodoroState } from "@/types/pomodoro"
import { cn } from "@/lib/utils"
import { Button } from "@/components/ui/button"
import { Play, Pause, RotateCcw } from "lucide-react"

const CIRCUMFERENCE = 2 * Math.PI * 120

const PHASE_LABELS: Record<string, string> = {
  work: "专注",
  short_break: "短休",
  long_break: "长休",
}

function formatTime(seconds: number): string {
  const m = Math.floor(seconds / 60)
  const s = seconds % 60
  return `${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`
}

export function PomodoroTimer() {
  const [state, setState] = useState<PomodoroState | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    let unlistenTick: UnlistenFn | undefined
    let unlistenPhase: UnlistenFn | undefined

    if (typeof Notification !== "undefined" && Notification.permission === "default") {
      Notification.requestPermission()
    }

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

        unlistenPhase = await listen<string>("pomodoro-phase-change", (event) => {
          const phase = event.payload
          const label = PHASE_LABELS[phase] ?? phase
          if (typeof Notification !== "undefined" && Notification.permission === "granted") {
            new Notification("Kairos", {
              body: phase === "work"
                ? `休息结束，开始新的${label}`
                : `${label}时间到！`,
            })
          }
        })
      } catch {
        setError("无法监听计时器事件")
      }
    }

    init()

    return () => {
      unlistenTick?.()
      unlistenPhase?.()
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

  return (
    <div className="flex flex-col items-center gap-6 w-full">
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
              filter: `drop-shadow(0 0 7px ${isWork ? "oklch(0.62 0.16 275 / 0.55)" : "oklch(0.72 0.15 160 / 0.55)"})`,
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
    </div>
  )
}
