import { ArrowUpRight, Pause, Play, Timer } from "lucide-react"
import type { PomodoroState } from "@/types/pomodoro"
import type { MainNavigationTarget } from "@/types/widget"
import { cn } from "@/lib/utils"
import { formatDuration, phaseLabel } from "@/components/widget/widget-format"

interface MediumWidgetProps {
  pomodoro: PomodoroState
  busy: boolean
  onToggle: () => void
  onOpen: (target: MainNavigationTarget) => void
}

export function MediumWidget({ pomodoro, busy, onToggle, onOpen }: MediumWidgetProps) {
  const progress =
    pomodoro.total_seconds > 0
      ? Math.max(0, Math.min(100, ((pomodoro.total_seconds - pomodoro.remaining_seconds) / pomodoro.total_seconds) * 100))
      : 0
  const ToggleIcon = pomodoro.is_running ? Pause : Play

  return (
    <div className="flex h-full flex-col justify-between px-4 py-3">
      <button
        type="button"
        onClick={() => onOpen("pomodoro")}
        className="flex items-center justify-between gap-3 rounded-md text-left text-muted-foreground hover:text-foreground"
      >
        <span className="flex items-center gap-2 text-xs font-medium">
          <Timer className="h-3.5 w-3.5" />
          {phaseLabel(pomodoro.phase)}
        </span>
        <ArrowUpRight className="h-3.5 w-3.5" />
      </button>

      <div className="flex items-end justify-between gap-3">
        <div className="min-w-0">
          <div className="font-heading text-4xl font-semibold leading-none tabular-nums text-foreground">
            {formatDuration(pomodoro.remaining_seconds)}
          </div>
          <div className="mt-2 text-xs text-muted-foreground">
            今日完成 {pomodoro.completed_sessions} 轮
          </div>
        </div>

        <button
          type="button"
          onClick={onToggle}
          disabled={busy}
          aria-label={pomodoro.is_running ? "暂停番茄钟" : "开始番茄钟"}
          className={cn(
            "flex h-11 w-11 shrink-0 items-center justify-center rounded-lg transition-colors",
            "bg-primary text-primary-foreground hover:bg-primary/80 disabled:opacity-50",
          )}
        >
          <ToggleIcon className="h-5 w-5" />
        </button>
      </div>

      <div className="h-1.5 overflow-hidden rounded-full bg-muted/60">
        <div className="h-full rounded-full bg-primary transition-[width]" style={{ width: `${progress}%` }} />
      </div>
    </div>
  )
}
