import type { CSSProperties } from "react"
import { useEffect, useState } from "react"
import { invoke } from "@tauri-apps/api/core"
import type { CalendarEvent, CalendarWeekCmd, CalendarWeekResponse } from "@/types/schedule"
import { cn } from "@/lib/utils"
import {
  BookOpen,
  CalendarDays,
  CheckCircle2,
  ChevronLeft,
  ChevronRight,
  GraduationCap,
  ListTodo,
  MapPin,
} from "lucide-react"

const DAY_SHORT = ["一", "二", "三", "四", "五", "六", "日"]
const HOUR_HEIGHT = 44
const START_HOUR = 7
const END_HOUR = 22
const TOTAL_HOURS = END_HOUR - START_HOUR
const COMPACT_TIME_AXIS_WIDTH = 28
const COMPACT_DAY_MIN_WIDTH = 44

type CalendarMode = "week" | "day"

function hexToRgb(hex: string): { r: number; g: number; b: number } | null {
  const match = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex)
  if (!match) return null
  return {
    r: parseInt(match[1], 16),
    g: parseInt(match[2], 16),
    b: parseInt(match[3], 16),
  }
}

function hexToRgba(hex: string, alpha: number): string {
  const rgb = hexToRgb(hex)
  if (!rgb) return hex
  return `rgba(${rgb.r}, ${rgb.g}, ${rgb.b}, ${alpha})`
}

function isDarkColor(hex: string): boolean {
  const rgb = hexToRgb(hex)
  if (!rgb) return false
  const luminance = (0.299 * rgb.r + 0.587 * rgb.g + 0.114 * rgb.b) / 255
  return luminance <= 0.55
}

function timeToMinutes(time: string): number {
  const [h, m] = time.split(":").map(Number)
  if (!Number.isFinite(h) || !Number.isFinite(m)) return START_HOUR * 60
  return h * 60 + m
}

function clampMinutes(minutes: number): number {
  return Math.max(START_HOUR * 60, Math.min(END_HOUR * 60, minutes))
}

function minutesToTop(minutes: number): number {
  return ((clampMinutes(minutes) - START_HOUR * 60) / 60) * HOUR_HEIGHT
}

function todayKey(): string {
  const d = new Date()
  const year = d.getFullYear()
  const month = String(d.getMonth() + 1).padStart(2, "0")
  const day = String(d.getDate()).padStart(2, "0")
  return `${year}-${month}-${day}`
}

function dayCellKey(weekStartDate: string, offset: number): string {
  const start = new Date(`${weekStartDate}T00:00:00`)
  if (Number.isNaN(start.getTime())) return ""
  start.setDate(start.getDate() + offset)
  const month = String(start.getMonth() + 1).padStart(2, "0")
  const day = String(start.getDate()).padStart(2, "0")
  return `${start.getFullYear()}-${month}-${day}`
}

function startOfWeekKey(date = new Date()): string {
  const d = new Date(date)
  d.setHours(0, 0, 0, 0)
  const dayFromMonday = d.getDay() === 0 ? 6 : d.getDay() - 1
  d.setDate(d.getDate() - dayFromMonday)
  const year = d.getFullYear()
  const month = String(d.getMonth() + 1).padStart(2, "0")
  const day = String(d.getDate()).padStart(2, "0")
  return `${year}-${month}-${day}`
}

function addDaysToKey(dateKey: string, days: number): string {
  const date = new Date(`${dateKey}T00:00:00`)
  if (Number.isNaN(date.getTime())) return startOfWeekKey()
  date.setDate(date.getDate() + days)
  return dayCellKey(startOfWeekKey(date), 0)
}

function formatWeekRange(startDate: string, endDate: string): string {
  if (!startDate || !endDate) return "日期周"
  return `${startDate.slice(5)} - ${endDate.slice(5)}`
}

function formatMonthLabel(startDate: string, endDate: string): string {
  const start = new Date(`${startDate}T00:00:00`)
  const end = new Date(`${endDate}T00:00:00`)
  if (Number.isNaN(start.getTime()) || Number.isNaN(end.getTime())) return "日历"

  const startYear = start.getFullYear()
  const endYear = end.getFullYear()
  const startMonth = start.getMonth() + 1
  const endMonth = end.getMonth() + 1

  if (startYear === endYear && startMonth === endMonth) {
    return `${startYear}年${startMonth}月`
  }
  if (startYear === endYear) {
    return `${startYear}年${startMonth}月 / ${endMonth}月`
  }
  return `${startYear}年${startMonth}月 / ${endYear}年${endMonth}月`
}

function eventTypeLabel(kind: CalendarEvent["kind"]): string {
  switch (kind) {
    case "course":
      return "课程"
    case "exam":
      return "考试"
    case "task":
      return "待办"
  }
}

function eventTimeLabel(event: CalendarEvent): string {
  if (event.kind === "task") return "全天截止"
  return `${event.start_time}-${event.end_time}`
}

function eventToneClass(kind: CalendarEvent["kind"]): string {
  switch (kind) {
    case "course":
      return "text-primary"
    case "exam":
      return "text-destructive"
    case "task":
      return "text-amber-600 dark:text-amber-300"
  }
}

function EventTypeIcon({
  kind,
  className,
}: {
  kind: CalendarEvent["kind"]
  className?: string
}) {
  if (kind === "course") return <BookOpen className={className} />
  if (kind === "exam") return <GraduationCap className={className} />
  return <ListTodo className={className} />
}

interface EventCardProps {
  event: CalendarEvent
  compact?: boolean
  onClick: () => void
  className?: string
}

function EventCard({ event, compact = false, onClick, className }: EventCardProps) {
  const isDone = event.kind === "task" && event.tags.includes("完成")
  const isExam = event.kind === "exam"
  const markerStyle: CSSProperties = {
    backgroundColor: isDone ? undefined : event.color,
  }
  const cardStyle: CSSProperties = {
    borderColor: isExam ? event.color : undefined,
    backgroundColor: isExam ? hexToRgba(event.color, 0.08) : undefined,
  }

  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        "flex w-full items-start gap-3 rounded-lg border border-border/60 bg-card/75 text-left transition-colors hover:bg-card active:bg-muted/60",
        compact ? "min-h-14 px-2.5 py-2" : "min-h-16 px-3 py-3",
        isExam && "border-dashed",
        className,
      )}
      style={cardStyle}
    >
      <span
        className={cn(
          "mt-0.5 flex shrink-0 items-center justify-center rounded-lg text-white",
          compact ? "h-8 w-8" : "h-9 w-9",
          isDone && "bg-muted text-muted-foreground",
        )}
        style={markerStyle}
      >
        {isDone ? (
          <CheckCircle2 className="h-4 w-4" />
        ) : (
          <EventTypeIcon kind={event.kind} className="h-4 w-4" />
        )}
      </span>

      <span className="min-w-0 flex-1">
        <span className="flex flex-wrap items-center gap-x-2 gap-y-0.5 text-xs text-muted-foreground">
          <span className={cn("font-medium", eventToneClass(event.kind))}>
            {eventTypeLabel(event.kind)}
          </span>
          <span className="tabular-nums">{eventTimeLabel(event)}</span>
        </span>
        <span
          className={cn(
            "mt-1 block break-words font-semibold leading-snug text-foreground",
            compact ? "text-xs" : "text-sm",
            isDone && "text-muted-foreground line-through",
          )}
        >
          {event.title}
        </span>
        {event.location && (
          <span className="mt-1 flex items-center gap-1 text-xs text-muted-foreground">
            <MapPin className="h-3 w-3 shrink-0" />
            <span className="truncate">{event.location}</span>
          </span>
        )}
        {!compact && event.tags.length > 0 && (
          <span className="mt-2 flex flex-wrap gap-1">
            {event.tags.slice(0, 3).map((tag) => (
              <span
                key={`${event.kind}-${event.id}-${tag}`}
                className="rounded-md bg-muted px-1.5 py-0.5 text-[10px] font-medium text-muted-foreground"
              >
                {tag}
              </span>
            ))}
          </span>
        )}
      </span>
    </button>
  )
}

function getEventBlockMetrics(event: CalendarEvent, minHeight: number) {
  const startMin = timeToMinutes(event.start_time)
  const endMin = Math.max(startMin + 30, timeToMinutes(event.end_time))
  const dayEnd = END_HOUR * 60
  const clampedStart = Math.min(clampMinutes(startMin), dayEnd - 30)
  const clampedEnd = Math.min(dayEnd, Math.max(clampedStart + 30, clampMinutes(endMin)))

  return {
    top: minutesToTop(clampedStart),
    height: Math.max(minHeight, ((clampedEnd - clampedStart) / 60) * HOUR_HEIGHT - 2),
  }
}

interface CalendarWeekTimetableProps {
  weekData: CalendarWeekResponse
  allEvents: CalendarEvent[]
  today: string
  selectedDayIndex: number
  compact?: boolean
  onDayClick: (dayIndex: number) => void
  onEventClick: (event: CalendarEvent) => void
}

function CalendarWeekTimetable({
  weekData,
  allEvents,
  today,
  selectedDayIndex,
  compact = false,
  onDayClick,
  onEventClick,
}: CalendarWeekTimetableProps) {
  const gridTemplateColumns = compact
    ? `${COMPACT_TIME_AXIS_WIDTH}px repeat(7, minmax(${COMPACT_DAY_MIN_WIDTH}px, 1fr))`
    : "48px repeat(7, minmax(0, 1fr))"
  const timelineHeight = TOTAL_HOURS * HOUR_HEIGHT

  return (
    <div className={cn("overflow-x-auto pb-4", compact ? "-mx-4 px-1" : "px-1")}>
      <div
        className={cn(
          "grid w-full select-none",
          compact ? "min-w-[336px]" : "min-w-[680px]",
        )}
      >
        <div className="grid" style={{ gridTemplateColumns }}>
          <div />
          {DAY_SHORT.map((day, dayIndex) => {
            const dateKey = dayCellKey(weekData.week_start_date, dayIndex)
            const dateNum = dateKey ? dateKey.slice(8) : ""
            const isToday = dateKey === today

            return (
              <button
                key={day}
                type="button"
                onClick={() => onDayClick(dayIndex)}
                className={cn(
                  "flex h-12 min-w-11 flex-col items-center justify-center text-center transition-colors",
                  isToday
                    ? "bg-primary text-primary-foreground shadow-sm"
                    : dayIndex === selectedDayIndex
                      ? "bg-primary/10 text-primary"
                      : "text-foreground hover:bg-muted/40",
                )}
              >
                <span className="text-[10px] font-medium leading-tight">{day}</span>
                <span className="mt-0.5 text-xs font-semibold leading-tight tabular-nums">
                  {dateNum || "--"}
                </span>
              </button>
            )
          })}

          <div className="border-y border-border/35 bg-card px-1.5 py-2 text-[10px] font-medium text-muted-foreground">
            截止
          </div>
          {DAY_SHORT.map((day, dayIndex) => {
            const dateKey = dayCellKey(weekData.week_start_date, dayIndex)
            const taskEvents = allEvents.filter(
              (event) => event.kind === "task" && event.day_of_week === dayIndex + 1,
            )
            const isToday = dateKey === today

            return (
              <div
                key={`tasks-${day}`}
                className={cn(
                  "min-h-12 border-y border-l border-border/35 px-1 py-1.5",
                  isToday && "bg-primary/[0.04]",
                )}
              >
                {taskEvents.length === 0 ? (
                  <span className="block px-1 py-1 text-[10px] text-muted-foreground/50">
                    无
                  </span>
                ) : (
                  <div className="space-y-1">
                    {taskEvents.map((event) => {
                      const isDone = event.tags.includes("完成")
                      const taskStyle: CSSProperties = {
                        borderColor: event.color,
                        backgroundColor: hexToRgba(event.color, 0.12),
                      }

                      return (
                        <button
                          key={`task-${event.id}`}
                          type="button"
                          onClick={() => onEventClick(event)}
                          className={cn(
                            "flex min-h-7 w-full items-center gap-1 rounded-md border px-1.5 py-1 text-left text-[10px] font-medium leading-tight transition-colors hover:bg-card",
                            isDone && "text-muted-foreground line-through",
                          )}
                          style={taskStyle}
                        >
                          <ListTodo className="h-3 w-3 shrink-0" />
                          <span className="line-clamp-2">{event.title}</span>
                        </button>
                      )
                    })}
                  </div>
                )}
              </div>
            )
          })}

          <div className="relative bg-card" style={{ height: `${timelineHeight}px` }}>
            {Array.from({ length: TOTAL_HOURS }, (_, i) => {
              if (i === 0) return <div key={i} style={{ height: `${HOUR_HEIGHT}px` }} />
              return (
                <div
                  key={i}
                  className={cn(
                    "absolute text-muted-foreground/70 tabular-nums",
                    compact ? "right-1 text-[9px]" : "right-1.5 text-[10px]",
                  )}
                  style={{ top: `${i * HOUR_HEIGHT - (compact ? 5 : 6)}px` }}
                >
                  {compact
                    ? String(START_HOUR + i).padStart(2, "0")
                    : `${String(START_HOUR + i).padStart(2, "0")}:00`}
                </div>
              )
            })}
          </div>

          {DAY_SHORT.map((day, dayIndex) => {
            const dateKey = dayCellKey(weekData.week_start_date, dayIndex)
            const isToday = dateKey === today
            const isSelected = dayIndex === selectedDayIndex
            const timedEvents = allEvents.filter(
              (event) => event.kind !== "task" && event.day_of_week === dayIndex + 1,
            )

            return (
              <div
                key={`timeline-${day}`}
                className={cn(
                  "relative border-l border-border/25",
                  isToday && "bg-primary/[0.04]",
                  isSelected && !isToday && "bg-primary/[0.02]",
                )}
                style={{ height: `${timelineHeight}px` }}
              >
                {Array.from({ length: TOTAL_HOURS }, (_, i) => (
                  <div
                    key={i}
                    className="absolute inset-x-0 border-t border-border/25"
                    style={{ top: `${i * HOUR_HEIGHT}px` }}
                  />
                ))}

                {timedEvents.map((event) => {
                  const { top, height } = getEventBlockMetrics(event, compact ? 26 : 28)
                  const isExam = event.kind === "exam"
                  const dark = isDarkColor(event.color)
                  const blockStyle: CSSProperties = {
                    top: `${top}px`,
                    height: `${height}px`,
                    backgroundColor: isExam ? hexToRgba(event.color, 0.16) : event.color,
                    borderColor: isExam ? event.color : undefined,
                  }

                  return (
                    <button
                      key={`${event.kind}-${event.id}`}
                      type="button"
                      onClick={() => onEventClick(event)}
                      className={cn(
                        "absolute overflow-hidden rounded-lg text-left transition-all active:brightness-90",
                        compact ? "left-px right-px px-1.5 py-1" : "left-0.5 right-0.5 px-2 py-1.5",
                        isExam ? "border-2 border-dashed" : "hover:z-20 hover:brightness-105 hover:shadow-lg",
                        !isExam && dark ? "text-white" : "text-foreground",
                      )}
                      style={blockStyle}
                    >
                      {compact ? (
                        <span className="flex h-full min-w-0 flex-col justify-center">
                          <span className="block truncate text-[10px] font-semibold leading-tight">
                            {event.title}
                          </span>
                          {height >= 36 && (
                            <span className="block truncate text-[9px] leading-tight opacity-70">
                              {event.start_time}
                            </span>
                          )}
                        </span>
                      ) : (
                        <>
                          <span className="flex items-center gap-1">
                            <EventTypeIcon kind={event.kind} className="h-3.5 w-3.5 shrink-0" />
                            <span className="line-clamp-2 text-[11px] font-semibold leading-tight">
                              {event.title}
                            </span>
                          </span>
                          <span className="block text-[9px] leading-tight opacity-70">
                            {event.start_time}
                          </span>
                          {event.location && (
                            <span className="block truncate text-[10px] leading-tight opacity-70">
                              {event.location}
                            </span>
                          )}
                        </>
                      )}
                    </button>
                  )
                })}
              </div>
            )
          })}
        </div>
      </div>
    </div>
  )
}

interface CalendarViewProps {
  onNavigate: (key: string) => void
}

export function CalendarView({ onNavigate }: CalendarViewProps) {
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [weekData, setWeekData] = useState<CalendarWeekResponse | null>(null)
  const [weekStartDate, setWeekStartDate] = useState(() => startOfWeekKey())
  const [viewMode, setViewMode] = useState<CalendarMode>("day")
  const [selectedDayIndex, setSelectedDayIndex] = useState(
    new Date().getDay() === 0 ? 6 : new Date().getDay() - 1,
  )
  const [weekRefresh, setWeekRefresh] = useState(0)

  const semester = (() => {
    const now = new Date()
    const y = now.getFullYear()
    return now.getMonth() >= 1 && now.getMonth() <= 6 ? `${y}S1` : `${y}S2`
  })()

  useEffect(() => {
    let cancelled = false

    async function loadWeek() {
      setLoading(true)
      setError(null)
      try {
        const cmd: CalendarWeekCmd = {
          semester,
          week_index: 1,
          week_start_date: weekStartDate,
        }
        const res = await invoke<CalendarWeekResponse>("get_calendar_week", {
          cmd,
        })
        if (cancelled) return
        setWeekData(res)
      } catch (e) {
        if (!cancelled) {
          setWeekData(null)
          setError(typeof e === "string" ? e : "无法加载日历数据")
        }
      } finally {
        if (!cancelled) setLoading(false)
      }
    }

    void loadWeek()
    return () => {
      cancelled = true
    }
  }, [semester, weekStartDate, weekRefresh])

  const handlePrevWeek = () => setWeekStartDate((date) => addDaysToKey(date, -7))
  const handleNextWeek = () => setWeekStartDate((date) => addDaysToKey(date, 7))

  const handleDayClick = (dayIndex: number) => {
    setSelectedDayIndex(dayIndex)
    setViewMode("day")
  }

  const handleEventClick = (event: CalendarEvent) => {
    onNavigate(event.source_link)
  }

  const today = todayKey()
  const currentWeekStart = startOfWeekKey()
  const isCurrentWeek = weekStartDate === currentWeekStart
  const weekRangeLabel = formatWeekRange(
    weekData?.week_start_date ?? weekStartDate,
    weekData?.week_end_date ?? dayCellKey(weekStartDate, 6),
  )
  const monthLabel = formatMonthLabel(
    weekData?.week_start_date ?? weekStartDate,
    weekData?.week_end_date ?? dayCellKey(weekStartDate, 6),
  )
  const semesterWeekLabel =
    weekData && weekData.week_index >= 1 ? `第 ${weekData.week_index} 周` : "日期周"

  const allEvents = weekData?.events ?? []
  const selectedDate = weekData ? dayCellKey(weekData.week_start_date, selectedDayIndex) : ""
  const selectedEvents = allEvents.filter((event) => event.day_of_week === selectedDayIndex + 1)

  return (
    <div className="flex h-full flex-col">
      <div className="flex shrink-0 flex-wrap items-center gap-2 px-3 py-2">
        <button
          type="button"
          onClick={handlePrevWeek}
          aria-label="上一周"
          className="inline-flex h-9 w-9 shrink-0 items-center justify-center rounded-lg text-muted-foreground transition-colors hover:bg-muted/60 hover:text-foreground"
        >
          <ChevronLeft className="h-4 w-4" />
        </button>

        <button
          type="button"
          onClick={() => {
            if (!isCurrentWeek) {
              setWeekStartDate(currentWeekStart)
            } else {
              setWeekRefresh((n) => n + 1)
            }
          }}
          className="inline-flex min-h-11 items-center rounded-md px-2.5 text-left transition-colors hover:bg-muted/60"
        >
          <span className="flex flex-col leading-tight">
            <span className="text-sm font-semibold text-foreground">{monthLabel}</span>
            <span className="mt-0.5 text-[11px] font-medium tabular-nums text-muted-foreground">
              {weekRangeLabel}
              {isCurrentWeek ? " · 本周" : ""}
            </span>
          </span>
        </button>

        <button
          type="button"
          onClick={handleNextWeek}
          aria-label="下一周"
          className="inline-flex h-9 w-9 shrink-0 items-center justify-center rounded-lg text-muted-foreground transition-colors hover:bg-muted/60 hover:text-foreground"
        >
          <ChevronRight className="h-4 w-4" />
        </button>

        {!isCurrentWeek && (
          <button
            type="button"
            onClick={() => setWeekStartDate(currentWeekStart)}
            className="h-9 shrink-0 rounded-md px-2 text-[11px] font-medium text-primary transition-colors hover:bg-primary/10"
          >
            今天
          </button>
        )}

        <div className="min-w-full flex-1 sm:min-w-0" />

        <div className="grid h-9 grid-cols-2 rounded-lg bg-muted p-1 text-xs font-medium">
          {(["week", "day"] as CalendarMode[]).map((mode) => (
            <button
              key={mode}
              type="button"
              onClick={() => setViewMode(mode)}
              aria-current={viewMode === mode ? "page" : undefined}
              className={cn(
                "rounded-md px-3 transition-colors",
                viewMode === mode
                  ? "bg-card text-foreground shadow-sm"
                  : "text-muted-foreground hover:text-foreground",
              )}
            >
              {mode === "week" ? "周视图" : "日视图"}
            </button>
          ))}
        </div>

        <span className="hidden text-[10px] text-muted-foreground/60 tabular-nums sm:inline">
          {semester} · {semesterWeekLabel}
        </span>
      </div>

      {viewMode === "day" && (
        <div className="shrink-0 overflow-x-auto pb-1">
          <div className="grid min-w-[348px] select-none grid-cols-7">
            {DAY_SHORT.map((day, i) => {
              const dateKey = weekData ? dayCellKey(weekData.week_start_date, i) : ""
              const dateNum = dateKey ? dateKey.slice(8) : ""
              const isToday = dateKey === today
              const isSelected = i === selectedDayIndex

              return (
                <button
                  key={day}
                  type="button"
                  onClick={() => handleDayClick(i)}
                  className={cn(
                    "flex h-12 min-w-11 flex-col items-center justify-center text-center transition-colors",
                    isToday
                      ? "bg-primary text-primary-foreground shadow-sm"
                      : isSelected
                        ? "bg-primary/10 text-primary"
                        : "text-foreground hover:bg-muted/40",
                  )}
                >
                  <span className="text-[10px] font-medium leading-tight">{day}</span>
                  <span className="mt-0.5 text-xs font-semibold leading-tight tabular-nums">
                    {dateNum || "--"}
                  </span>
                </button>
              )
            })}
          </div>
        </div>
      )}

      <div className="min-h-0 flex-1 overflow-y-auto pb-16 md:pb-4">
        {loading && (
          <div className="flex items-center justify-center py-12">
            <div className="h-6 w-6 animate-spin rounded-full border-2 border-primary border-t-transparent" />
          </div>
        )}

        {!loading && error && !weekData && (
          <div className="p-8 text-center">
            <p className="text-sm text-muted-foreground">{error}</p>
            <p className="mt-1 text-xs text-muted-foreground/60">
              请先导入课表，或先创建带截止日期的待办。
            </p>
            <button
              type="button"
              onClick={() => setWeekRefresh((n) => n + 1)}
              className="mt-3 inline-flex h-9 items-center rounded-md px-3 text-xs font-medium text-primary transition-colors hover:bg-primary/10"
            >
              重试
            </button>
          </div>
        )}

        {!loading && !error && !weekData && (
          <p className="py-8 text-center text-sm text-muted-foreground">暂无日历数据</p>
        )}

        {!loading && weekData && allEvents.length === 0 && (
          <div className="mx-auto max-w-sm px-6 py-10 text-center">
            <CalendarDays className="mx-auto h-8 w-8 text-muted-foreground/60" />
            <p className="mt-3 text-sm font-medium text-foreground">本周暂无安排</p>
            <p className="mt-1 text-xs text-muted-foreground">
              有截止日期的待办、课程和考试会显示在这里。
            </p>
          </div>
        )}

        {!loading && weekData && allEvents.length > 0 && viewMode === "week" && (
          <>
            <div className="hidden md:block">
              <CalendarWeekTimetable
                weekData={weekData}
                allEvents={allEvents}
                today={today}
                selectedDayIndex={selectedDayIndex}
                onDayClick={handleDayClick}
                onEventClick={handleEventClick}
              />
            </div>
            <div className="md:hidden">
              <CalendarWeekTimetable
                weekData={weekData}
                allEvents={allEvents}
                today={today}
                selectedDayIndex={selectedDayIndex}
                compact
                onDayClick={handleDayClick}
                onEventClick={handleEventClick}
              />
            </div>
          </>
        )}

        {!loading && weekData && allEvents.length > 0 && viewMode === "day" && (
          <div className="px-1 pb-4">
            {selectedEvents.length === 0 ? (
              <div className="px-4 py-10 text-center">
                <p className="text-sm font-medium text-foreground">
                  {selectedDate || "这一天"}暂无安排
                </p>
                <p className="mt-1 text-xs text-muted-foreground">
                  切换上方日期查看本周其他日程。
                </p>
              </div>
            ) : (
              <div className="mx-auto max-w-2xl space-y-2">
                <div className="px-2 pb-1 pt-2 text-xs font-medium text-muted-foreground">
                  {selectedDate} 周{DAY_SHORT[selectedDayIndex]}
                </div>
                {selectedEvents.map((event) => (
                  <EventCard
                    key={`day-${event.kind}-${event.id}`}
                    event={event}
                    onClick={() => handleEventClick(event)}
                  />
                ))}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  )
}
