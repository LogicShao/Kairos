import { useState, useEffect, useCallback } from "react"
import { invoke } from "@tauri-apps/api/core"
import type {
  PomodoroState,
  PomodoroConfig,
  PomodoroPhase,
  ResolvePomodoroInterruptionRequest,
} from "@/types/pomodoro"
import type { SyncFinishedEvent } from "@/types/sync"
import { cn } from "@/lib/utils"
import { userErrorMessage } from "@/lib/errors"
import { listenWithCleanup } from "@/lib/tauri-events"
import { Button } from "@/components/ui/button"
import { Stepper } from "@/components/ui/stepper"
import { Modal } from "@/components/shared/modal"
import { PageShell } from "@/components/shared/page-shell"
import { Play, Pause, RotateCcw, Settings } from "lucide-react"

const CIRCUMFERENCE = 2 * Math.PI * 120

/** Android WebView 渲染性能较弱：禁用高开销的 SVG drop-shadow 与长过渡动画。 */
const IS_ANDROID = typeof navigator !== "undefined" && navigator.userAgent.includes("Android")

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
  const [resolving, setResolving] = useState(false)

  // Settings modal
  const [settingsOpen, setSettingsOpen] = useState(false)
  const [workMinutes, setWorkMinutes] = useState(RANGES.work.default)
  const [shortBreakMinutes, setShortBreakMinutes] = useState(RANGES.shortBreak.default)
  const [longBreakMinutes, setLongBreakMinutes] = useState(RANGES.longBreak.default)
  const [sessionsBeforeLongBreak, setSessionsBeforeLongBreak] = useState(RANGES.sessions.default)
  const [autoStartNextPhase, setAutoStartNextPhase] = useState(false)
  const [savingConfig, setSavingConfig] = useState(false)
  const [configLoading, setConfigLoading] = useState(false)

  useEffect(() => {
    let disposed = false
    let cleanupTick: (() => void) | undefined
    let cleanupSyncFinished: (() => void) | undefined

    async function refreshState() {
      const nextState = await invoke<PomodoroState>("get_pomodoro_state")
      if (disposed) return
      setState(nextState)
    }

    async function init() {
      try {
        await refreshState()
      } catch {
        if (disposed) return
        setState({
          phase: "work",
          remaining_seconds: 1500,
          total_seconds: 1500,
          is_running: false,
          completed_sessions: 0,
          interrupted: false,
          interrupted_session_id: null,
          last_seen_at: null,
        })
        setError("Tauri 不可用 — 展示离线 UI")
        return
      }

      if (disposed) return

      cleanupTick = listenWithCleanup<PomodoroState>(
        "pomodoro-tick",
        (event) => {
          // 状态未变化时跳过重渲染（暂停阶段每秒也会收到 tick 事件）。
          setState((prev) => {
            if (!prev) return event.payload
            const next = event.payload
            if (
              prev.phase === next.phase &&
              prev.remaining_seconds === next.remaining_seconds &&
              prev.total_seconds === next.total_seconds &&
              prev.is_running === next.is_running &&
              prev.completed_sessions === next.completed_sessions &&
              prev.interrupted === next.interrupted
            ) {
              return prev
            }
            return next
          })
        },
        () => {
          setError("无法监听计时器事件")
        },
      )

      cleanupSyncFinished = listenWithCleanup<SyncFinishedEvent>(
        "sync-finished",
        () => {
          refreshState().catch(() => {
            if (!disposed) {
              setError("无法刷新同步后的专注状态")
            }
          })
        },
        () => {
          setError("无法监听同步事件")
        },
      )
    }

    init()

    return () => {
      disposed = true
      cleanupTick?.()
      cleanupSyncFinished?.()
    }
  }, [])

  const handleStartPause = useCallback(() => {
    if (!state) return
    if (state.is_running) {
      invoke("pause_pomodoro")
        .then(() => setError(null))
        .catch((err) => {
          setError(userErrorMessage(err, "暂停番茄钟失败"))
        })
    } else {
      invoke("start_pomodoro")
        .then(() => setError(null))
        .catch((err) => {
          setError(userErrorMessage(err, "启动番茄钟失败"))
        })
    }
  }, [state])

  const handleReset = useCallback(() => {
    invoke("reset_pomodoro")
      .then(() => setError(null))
      .catch((err) => {
        setError(userErrorMessage(err, "重置番茄钟失败"))
      })
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
      setAutoStartNextPhase(config.auto_start_next_phase)
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
          auto_start_next_phase: autoStartNextPhase,
        },
      })
      setSettingsOpen(false)
      const newState = await invoke<PomodoroState>("get_pomodoro_state")
      setState(newState)
      setError(null)
    } catch (e) {
      setError(userErrorMessage(e, "保存番茄钟设置失败"))
    } finally {
      setSavingConfig(false)
    }
  }, [workMinutes, shortBreakMinutes, longBreakMinutes, sessionsBeforeLongBreak, autoStartNextPhase])

  /** 处理中断操作 */
  const handleResolveInterruption = useCallback(async (action: "continue" | "discard" | "complete") => {
    setResolving(true)
    try {
      const request: ResolvePomodoroInterruptionRequest = { action }
      const newState = await invoke<PomodoroState>("resolve_pomodoro_interruption", {
        request,
      })
      setState(newState)
      setError(null)
    } catch (e) {
      setError(userErrorMessage(e, "处理中断状态失败"))
    } finally {
      setResolving(false)
    }
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
              IS_ANDROID
                ? "transition-[stroke-dashoffset] duration-150 ease-linear"
                : "transition-[stroke-dashoffset] duration-1000 ease-linear",
            )}
            style={
              IS_ANDROID
                ? undefined
                : {
                    filter: `drop-shadow(0 0 7px ${isWork ? "oklch(0.66 0.15 235 / 0.55)" : "oklch(0.72 0.15 160 / 0.55)"})`,
                  }
            }
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

      {/* ─── 中断处理提示 ─── */}
      {state.interrupted && (
        <div className="flex flex-col items-center gap-3 w-full max-w-xs">
          <p className="text-sm text-amber-500 font-medium">
            上次专注被中断
            {state.last_seen_at && (
              <span className="text-xs text-muted-foreground block">
                上次运行于 {new Date(state.last_seen_at).toLocaleString("zh-CN")}
              </span>
            )}
          </p>
          <div className="flex gap-2">
            <Button
              variant="outline"
              size="sm"
              disabled={resolving}
              onClick={() => handleResolveInterruption("continue")}
            >
              继续
            </Button>
            <Button
              variant="outline"
              size="sm"
              disabled={resolving}
              onClick={() => handleResolveInterruption("discard")}
            >
              丢弃
            </Button>
            <Button
              variant="default"
              size="sm"
              disabled={resolving}
              onClick={() => handleResolveInterruption("complete")}
            >
              补记完成
            </Button>
          </div>
        </div>
      )}

      {/* ─── 控制按钮（中断时不显示操作按钮，直到处理完毕） ─── */}
      {!state.interrupted && (
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
      )}

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

            <div className="flex items-center justify-between gap-4 border-t border-border/50 pt-3">
              <div className="min-w-0">
                <span className="block text-sm text-foreground">自动开始下一阶段</span>
                <span className="mt-0.5 block text-[11px] text-muted-foreground/70">
                  关闭时阶段结束后暂停，需按"开始"才进入下一阶段
                </span>
              </div>
              <button
                type="button"
                role="switch"
                aria-checked={autoStartNextPhase}
                aria-label={autoStartNextPhase ? "关闭自动开始下一阶段" : "开启自动开始下一阶段"}
                onClick={() => setAutoStartNextPhase((v) => !v)}
                className={cn(
                  "relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors",
                  "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2",
                  autoStartNextPhase ? "bg-primary" : "bg-muted-foreground/25",
                )}
              >
                <span
                  className={cn(
                    "pointer-events-none h-5 w-5 rounded-full bg-card shadow-sm ring-0 transition-transform",
                    autoStartNextPhase ? "translate-x-5" : "translate-x-0",
                  )}
                />
              </button>
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
