import { useEffect, useState } from "react"
import type { WeekScheduleItem, WeekScheduleResponse } from "@/types/schedule"
import { useTheme } from "@/hooks/use-theme"
import { cn } from "@/lib/utils"
import { blendHex, blockPalette } from "@/lib/color"
import { GraduationCap, MapPin } from "lucide-react"
import {
  TOTAL_HOURS,
  HOUR_HEIGHT,
  START_HOUR,
  END_HOUR,
  timeToMinutes,
  minutesToTop,
  dayCellKey,
} from "./utils"

interface WeekGridProps {
  weekData: WeekScheduleResponse
  today: string
  mobileDayIndex: number
  onBlockClick: (item: WeekScheduleItem) => void
}

function currentMinutes(): number {
  const now = new Date()
  return now.getHours() * 60 + now.getMinutes()
}

function formatClock(minutes: number): string {
  const hours = String(Math.floor(minutes / 60)).padStart(2, "0")
  const mins = String(minutes % 60).padStart(2, "0")
  return `${hours}:${mins}`
}

interface CourseBlockProps {
  item: WeekScheduleItem
  top: number
  height: number
  compact: boolean
  dark: boolean
  onClick: () => void
}

/** 课程/考试色块：浅色高亮底 + 左侧课程色书脊，避免整块高饱和色抢占视觉。 */
function CourseBlock({ item, top, height, compact, dark, onClick }: CourseBlockProps) {
  const isExam = item.kind === "exam"
  const palette = blockPalette(item.color, dark)
  const background = isExam
    ? blendHex(item.color, dark ? "#3d3d3d" : "#ffffff", 0.9)
    : palette.background
  const border = isExam
    ? blendHex(item.color, dark ? "#3d3d3d" : "#ffffff", 0.25)
    : palette.border
  const showLocation = item.location.length > 0 && (compact ? height >= 48 : height >= 54)

  return (
    <button
      type="button"
      onClick={onClick}
      title={`${item.title}${item.location ? ` — ${item.location}` : ""}`}
      className={cn(
        "absolute overflow-hidden rounded-lg border text-left transition-all duration-150",
        compact
          ? "left-px right-px px-1.5 py-1"
          : "left-1 right-1 px-2 py-1.5 md:left-1.5 md:right-1.5",
        isExam
          ? "cursor-default border-[1.5px] border-dashed"
          : "cursor-pointer border-l-[3px] hover:z-20 hover:-translate-y-px hover:shadow-md focus-visible:z-20 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
      )}
      style={{
        top: `${top}px`,
        height: `${height}px`,
        backgroundColor: background,
        borderColor: border,
        borderLeftColor: isExam ? border : palette.accent,
        color: palette.ink,
      }}
    >
      <span
        className={cn(
          "block font-semibold leading-tight",
          compact ? "line-clamp-2 text-[10px]" : "line-clamp-2 text-[11px]",
        )}
      >
        {isExam && (
          <GraduationCap className="mr-0.5 inline h-3 w-3 -translate-y-px" aria-hidden="true" />
        )}
        {item.title}
      </span>
      {showLocation && (
        <span
          className={cn(
            "flex items-center gap-0.5 text-[9px] leading-tight opacity-75",
            compact ? "mt-0.5" : "mt-1",
          )}
        >
          <MapPin className="h-2.5 w-2.5 shrink-0" aria-hidden="true" />
          <span className="truncate">{item.location}</span>
        </span>
      )}
    </button>
  )
}

export function WeekGrid({ weekData, today, mobileDayIndex, onBlockClick }: WeekGridProps) {
  const { theme } = useTheme()
  const dark = theme === "dark"
  const [nowMinutes, setNowMinutes] = useState(currentMinutes)

  useEffect(() => {
    const timer = window.setInterval(() => setNowMinutes(currentMinutes()), 60_000)
    return () => window.clearInterval(timer)
  }, [])

  const nowTop =
    nowMinutes >= START_HOUR * 60 && nowMinutes <= END_HOUR * 60
      ? minutesToTop(nowMinutes)
      : null
  const showNow =
    nowTop !== null && today >= weekData.week_start_date && today <= weekData.week_end_date
  const timelineHeight = TOTAL_HOURS * HOUR_HEIGHT

  function renderGrid(compact: boolean) {
    return (
      <div
        className="grid w-full select-none"
        style={{
          gridTemplateColumns: compact
            ? "28px repeat(7, minmax(0, 1fr))"
            : "48px repeat(7, minmax(0, 1fr))",
        }}
      >
        {/* 时间标尺 */}
        <div className="relative" style={{ height: `${timelineHeight}px` }}>
          {Array.from({ length: TOTAL_HOURS }, (_, i) => {
            const top = i * HOUR_HEIGHT
            const hideLabel = showNow && nowTop !== null && Math.abs(nowTop - top) < 14
            return (
              <div key={i} className="absolute inset-x-0" style={{ top: `${top}px` }}>
                <span
                  className={cn(
                    "absolute right-0 h-px bg-border/60",
                    compact ? "w-1" : "w-1.5",
                  )}
                  aria-hidden="true"
                />
                {!hideLabel && (
                  <span
                    className={cn(
                      "absolute text-muted-foreground tabular-nums",
                      i === 0 ? "top-0.5" : "-translate-y-1/2",
                      compact ? "right-1.5 text-[9px]" : "right-2.5 text-[10px]",
                    )}
                  >
                    {compact
                      ? String(START_HOUR + i).padStart(2, "0")
                      : `${String(START_HOUR + i).padStart(2, "0")}:00`}
                  </span>
                )}
              </div>
            )
          })}

          {showNow && nowTop !== null && (
            <div
              className="absolute right-1 z-30 -translate-y-1/2"
              style={{ top: `${nowTop}px` }}
            >
              <span
                className={cn(
                  "inline-flex rounded bg-destructive font-semibold leading-none text-white tabular-nums shadow-sm",
                  compact ? "px-0.5 py-0.5 text-[8px]" : "px-1 py-0.5 text-[9px]",
                )}
              >
                {formatClock(nowMinutes)}
              </span>
            </div>
          )}
        </div>

        {/* 七天课程列 */}
        {Array.from({ length: 7 }, (_, dayIdx) => {
          const dateKey = dayCellKey(weekData.week_start_date, dayIdx)
          const isTodayCol = dateKey === today
          const isSelected = dayIdx === mobileDayIndex
          const isWeekend = dayIdx >= 5
          const dayItems = (weekData.items ?? []).filter(
            (it) => it.day_of_week === dayIdx + 1,
          )

          return (
            <div
              key={dayIdx}
              className={cn(
                "relative border-l border-border/40",
                isWeekend && !isTodayCol && "bg-muted/30",
                isTodayCol && "bg-primary/[0.04]",
                compact && isSelected && !isTodayCol && "bg-primary/[0.02]",
              )}
              style={{ height: `${timelineHeight}px` }}
            >
              {Array.from({ length: TOTAL_HOURS }, (_, i) => (
                <div
                  key={i}
                  className="absolute inset-x-0 border-t border-border/30"
                  style={{ top: `${i * HOUR_HEIGHT}px` }}
                />
              ))}

              {dayItems.map((item) => {
                const startMin = timeToMinutes(item.start_time)
                const endMin = timeToMinutes(item.end_time)
                const top = minutesToTop(startMin)
                const height = Math.max(
                  compact ? 26 : 24,
                  ((endMin - startMin) / 60) * HOUR_HEIGHT - 2,
                )

                return (
                  <CourseBlock
                    key={`${compact ? "m-" : ""}${item.kind}-${item.id}`}
                    item={item}
                    top={top}
                    height={height}
                    compact={compact}
                    dark={dark}
                    onClick={() => onBlockClick(item)}
                  />
                )
              })}

              {isTodayCol && showNow && nowTop !== null && (
                <div
                  className="pointer-events-none absolute inset-x-0 z-30"
                  style={{ top: `${nowTop}px` }}
                  aria-hidden="true"
                >
                  <div className="h-px bg-destructive" />
                  <span className="absolute -left-[3px] -top-[3px] h-2 w-2 rounded-full border-2 border-card bg-destructive" />
                </div>
              )}
            </div>
          )
        })}
      </div>
    )
  }

  return (
    <>
      {/* 桌面：时间标尺 + 七天网格 */}
      <div className="hidden md:block">
        <div className="glass-edge overflow-hidden rounded-xl border border-border/40 bg-card/75 backdrop-blur-md">
          {renderGrid(false)}
        </div>
      </div>

      {/* 移动：压缩时间标尺，七天等分视口 */}
      <div className="md:hidden">
        <div className="glass-edge overflow-hidden rounded-xl border border-border/40 bg-card/75 backdrop-blur-md">
          {renderGrid(true)}
        </div>
      </div>
    </>
  )
}
