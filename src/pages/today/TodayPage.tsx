import { useEffect, useState } from "react"
import { AcrylicPanel } from "@/components/shared/acrylic-panel"
import { Button } from "@/components/ui/button"
import { AiBriefCard } from "@/pages/today/AiBriefCard"
import type { TodayBriefingResponse } from "@/types/briefing"
import { getTodayBriefing } from "@/lib/api/briefing"
import { CalendarDays } from "lucide-react"

const PHASE_LABELS: Record<TodayBriefingResponse["phase"]["phase_type"], string> = {
  unknown: "未识别阶段",
  teaching: "教学周",
  exam: "考试周",
  break: "假期",
}

interface TodayPageProps {
  onNavigate: (key: string) => void
}

export function TodayPage({ onNavigate }: TodayPageProps) {
  const [briefing, setBriefing] = useState<TodayBriefingResponse | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [reloadKey, setReloadKey] = useState(0)

  useEffect(() => {
    let disposed = false
    getTodayBriefing()
      .then((data) => {
        if (!disposed) setBriefing(data)
      })
      .catch((err: string) => {
        if (!disposed) setError(err)
      })
    return () => {
      disposed = true
    }
  }, [reloadKey])

  if (error) {
    return (
      <div className="mx-auto flex w-full max-w-2xl flex-col gap-4 py-6">
        <AcrylicPanel className="p-6">
          <div className="flex flex-col items-center gap-3 text-center">
            <span className="flex h-11 w-11 items-center justify-center rounded-full bg-muted text-muted-foreground">
              <CalendarDays className="h-5 w-5" />
            </span>
            <div>
              <p className="text-sm font-medium text-foreground">今日概览加载失败</p>
              <p className="mt-1 text-xs text-muted-foreground">{error}</p>
            </div>
            <Button
              variant="outline"
              size="sm"
              onClick={() => {
                setError(null)
                setReloadKey((n) => n + 1)
              }}
            >
              重新加载
            </Button>
          </div>
        </AcrylicPanel>
      </div>
    )
  }

  if (!briefing) {
    return (
      <div className="mx-auto flex w-full max-w-2xl flex-col gap-4 py-6">
        <AcrylicPanel className="p-5">
          <div className="flex items-center gap-4">
            <div className="h-20 w-16 animate-pulse rounded-xl bg-muted/60" />
            <div className="flex-1 space-y-2">
              <div className="h-3 w-24 animate-pulse rounded bg-muted" />
              <div className="h-5 w-32 animate-pulse rounded bg-muted" />
              <div className="h-3 w-40 animate-pulse rounded bg-muted/60" />
            </div>
          </div>
        </AcrylicPanel>
        <AcrylicPanel className="p-5">
          <div className="grid grid-cols-2 gap-2">
            {[1, 2, 3, 4].map((i) => (
              <div key={i} className="h-20 animate-pulse rounded-xl bg-muted/60" />
            ))}
          </div>
        </AcrylicPanel>
      </div>
    )
  }

  const { date, weekday_label, phase } = briefing
  const [year, month, day] = date.split("-")

  return (
    <div className="mx-auto flex w-full max-w-2xl flex-col gap-4 py-6">
      {/* 页头：日历式日期卡 + 学期阶段 */}
      <AcrylicPanel className="overflow-hidden bg-card p-0">
        <div className="flex items-center gap-4 p-4 sm:gap-5 sm:p-5">
          <div className="flex w-16 shrink-0 flex-col items-center justify-center rounded-xl border border-border/50 bg-muted/30 py-2.5">
            <span className="text-[9px] font-medium leading-none text-muted-foreground tabular-nums">
              {Number(month)}月
            </span>
            <span className="mt-1 font-heading text-3xl font-semibold leading-none text-foreground tabular-nums">
              {Number(day)}
            </span>
            <span className="mt-1.5 text-[9px] leading-none text-muted-foreground/60 tabular-nums">
              {year}
            </span>
          </div>

          <div className="min-w-0 flex-1">
            <p className="text-[11px] font-medium text-muted-foreground">{weekday_label}</p>
            <h1 className="mt-1 font-heading text-xl font-semibold leading-tight text-foreground">
              今天
            </h1>
            <p className="mt-1 truncate text-[11px] text-muted-foreground">
              {phase.term_label || "当前学期"}
            </p>
          </div>

          <div className="flex shrink-0 flex-col items-end gap-1.5">
            <span className="inline-flex rounded-full bg-primary/10 px-2.5 py-1 text-[10px] font-medium text-primary">
              {PHASE_LABELS[phase.phase_type]}
            </span>
            {phase.current_week !== null && phase.current_week !== undefined && phase.current_week > 0 ? (
              <span className="text-[10px] text-muted-foreground tabular-nums">
                第 {phase.current_week} 周
              </span>
            ) : (
              <span className="text-[10px] text-muted-foreground/60">—</span>
            )}
          </div>
        </div>
      </AcrylicPanel>

      {/* 今日概览：结构化数据 + AI 今日重点（AI 未启用时仅结构化数据） */}
      <AiBriefCard briefing={briefing} onNavigate={onNavigate} />
    </div>
  )
}
