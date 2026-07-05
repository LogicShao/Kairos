import { useCallback, useEffect, useRef, useState } from "react"
import { invoke } from "@tauri-apps/api/core"
import { listen, type UnlistenFn } from "@tauri-apps/api/event"
import { getCurrentWindow } from "@tauri-apps/api/window"
import type { TodayBriefingResponse } from "@/types/briefing"
import type { PomodoroState } from "@/types/pomodoro"
import type { MainNavigationTarget, SaveWidgetPositionRequest, WidgetConfig } from "@/types/widget"
import { LargeWidget } from "@/components/widget/LargeWidget"
import { MediumWidget } from "@/components/widget/MediumWidget"
import { SmallWidget } from "@/components/widget/SmallWidget"
import { WidgetFrame } from "@/components/widget/WidgetFrame"

async function fetchWidgetData(): Promise<[WidgetConfig, TodayBriefingResponse, PomodoroState]> {
  return Promise.all([
    invoke<WidgetConfig>("get_widget_config"),
    invoke<TodayBriefingResponse>("get_today_briefing"),
    invoke<PomodoroState>("get_pomodoro_state"),
  ])
}

export function WidgetApp() {
  const [config, setConfig] = useState<WidgetConfig | null>(null)
  const [briefing, setBriefing] = useState<TodayBriefingResponse | null>(null)
  const [pomodoro, setPomodoro] = useState<PomodoroState | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [pomodoroBusy, setPomodoroBusy] = useState(false)
  const configRef = useRef<WidgetConfig | null>(null)

  useEffect(() => {
    configRef.current = config
  }, [config])

  const refreshData = useCallback(async () => {
    setError(null)
    try {
      const [nextConfig, nextBriefing, nextPomodoro] = await fetchWidgetData()
      setConfig(nextConfig)
      setBriefing(nextBriefing)
      setPomodoro(nextPomodoro)
    } catch (err) {
      setError(typeof err === "string" ? err : "无法加载小组件")
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    let cancelled = false

    fetchWidgetData()
      .then(([nextConfig, nextBriefing, nextPomodoro]) => {
        if (cancelled) return
        setConfig(nextConfig)
        setBriefing(nextBriefing)
        setPomodoro(nextPomodoro)
      })
      .catch((err) => {
        if (cancelled) return
        setError(typeof err === "string" ? err : "无法加载小组件")
      })
      .finally(() => {
        if (cancelled) return
        setLoading(false)
      })

    return () => {
      cancelled = true
    }
  }, [])

  useEffect(() => {
    let unlistenConfig: UnlistenFn | undefined
    let unlistenPomodoro: UnlistenFn | undefined
    let unlistenSync: UnlistenFn | undefined

    listen<WidgetConfig>("widget-config-updated", (event) => {
      setConfig(event.payload)
    }).then((fn) => {
      unlistenConfig = fn
    })

    listen<PomodoroState>("pomodoro-tick", (event) => {
      setPomodoro(event.payload)
    }).then((fn) => {
      unlistenPomodoro = fn
    })

    listen("sync-finished", () => {
      void refreshData()
    }).then((fn) => {
      unlistenSync = fn
    })

    const interval = window.setInterval(() => {
      void refreshData()
    }, 60_000)

    return () => {
      unlistenConfig?.()
      unlistenPomodoro?.()
      unlistenSync?.()
      window.clearInterval(interval)
    }
  }, [refreshData])

  useEffect(() => {
    const appWindow = getCurrentWindow()
    let unlisten: UnlistenFn | undefined
    let timer: ReturnType<typeof window.setTimeout> | undefined

    appWindow.onMoved((event) => {
      const latestConfig = configRef.current
      if (!latestConfig || latestConfig.locked) return

      if (timer) {
        window.clearTimeout(timer)
      }
      timer = window.setTimeout(() => {
        const request: SaveWidgetPositionRequest = {
          x: event.payload.x,
          y: event.payload.y,
          width: latestConfig.width,
          height: latestConfig.height,
        }
        void invoke<WidgetConfig>("save_widget_position", { request }).then(setConfig)
      }, 600)
    }).then((fn) => {
      unlisten = fn
    })

    return () => {
      if (timer) {
        window.clearTimeout(timer)
      }
      unlisten?.()
    }
  }, [])

  const handleDragStart = useCallback(() => {
    if (configRef.current?.locked) return
    void getCurrentWindow().startDragging().catch((err) => {
      setError(typeof err === "string" ? err : "无法拖动小组件")
    })
  }, [])

  const handleClose = useCallback(() => {
    void invoke<WidgetConfig>("hide_widget").catch((err) => {
      setError(typeof err === "string" ? err : "无法关闭小组件")
    })
  }, [])

  const handleOpen = useCallback((target: MainNavigationTarget) => {
    void invoke("open_main_window", { target }).catch((err) => {
      setError(typeof err === "string" ? err : "无法打开主窗口")
    })
  }, [])

  const handlePomodoroToggle = useCallback(async () => {
    if (!pomodoro) return
    setPomodoroBusy(true)
    setError(null)
    try {
      await invoke(pomodoro.is_running ? "pause_pomodoro" : "start_pomodoro")
      const nextPomodoro = await invoke<PomodoroState>("get_pomodoro_state")
      setPomodoro(nextPomodoro)
      await refreshData()
    } catch (err) {
      setError(typeof err === "string" ? err : "番茄钟操作失败")
    } finally {
      setPomodoroBusy(false)
    }
  }, [pomodoro, refreshData])

  const hasData = briefing && pomodoro

  return (
    <WidgetFrame
      config={config}
      loading={loading}
      error={error}
      onRetry={refreshData}
      onClose={handleClose}
      onDragStart={handleDragStart}
    >
      {hasData && config?.mode === "small" && <SmallWidget briefing={briefing} onOpen={handleOpen} />}
      {hasData && config?.mode === "medium" && (
        <MediumWidget pomodoro={pomodoro} busy={pomodoroBusy} onToggle={handlePomodoroToggle} onOpen={handleOpen} />
      )}
      {hasData && config?.mode === "large" && (
        <LargeWidget briefing={briefing} pomodoro={pomodoro} onOpen={handleOpen} />
      )}
      {!hasData && !loading && !error && (
        <div className="flex h-full items-center justify-center px-4 text-center text-xs text-muted-foreground">
          暂无可显示数据
        </div>
      )}
    </WidgetFrame>
  )
}
