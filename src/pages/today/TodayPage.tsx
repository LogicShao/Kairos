import { useEffect, useState } from "react"
import { BookOpen, CalendarClock, ListTodo, Timer } from "lucide-react"
import { invoke } from "@tauri-apps/api/core"
import { AcrylicPanel } from "@/components/shared/acrylic-panel"
import { cn } from "@/lib/utils"
import { AiBriefCard } from "@/pages/today/AiBriefCard"
import type { TodayBriefingResponse } from "@/types/briefing"

const PHASE_LABELS: Record<TodayBriefingResponse["phase"]["phase_type"], string> = {
  unknown: "未识别阶段",
  teaching: "教学周",
  exam: "考试周",
  break: "假期",
}

function formatRemaining(seconds: number): string {
  const m = Math.floor(seconds / 60)
  const s = seconds % 60
  return `${m.toString().padStart(2, "0")}:${s.toString().padStart(2, "0")}`
}

function formatClock(value: string): string {
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return ""
  return date.toLocaleTimeString("zh-CN", {
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  })
}

function BriefingBlock({
  icon: Icon,
  iconColor,
  onClick,
  children,
}: {
  icon: typeof BookOpen
  iconColor: string
  onClick: () => void
  children: React.ReactNode
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        "flex items-start gap-2.5 rounded-lg border border-border/50 bg-card/50 px-3 py-2.5 text-left",
        "transition-colors hover:bg-card active:bg-muted/60 min-h-[3.25rem]",
      )}
    >
      <span className={cn("mt-0.5 flex shrink-0", iconColor)}>
        <Icon className="h-4 w-4" />
      </span>
      <span className="min-w-0 flex-1 leading-tight">{children}</span>
    </button>
  )
}

interface TodayPageProps {
  onNavigate: (key: string) => void
}

export function TodayPage({ onNavigate }: TodayPageProps) {
  const [briefing, setBriefing] = useState<TodayBriefingResponse | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    invoke<TodayBriefingResponse>("get_today_briefing")
      .then(setBriefing)
      .catch((err: string) => setError(err))
  }, [])

  if (error) {
    return (
      <div className="mx-auto flex w-full max-w-2xl flex-col gap-4 py-6">
        <AcrylicPanel className="bg-card p-6">
          <p className="text-sm text-muted-foreground">今日概览加载失败</p>
        </AcrylicPanel>
      </div>
    )
  }

  if (!briefing) {
    return (
      <div className="mx-auto flex w-full max-w-2xl flex-col gap-4 py-6">
        <AcrylicPanel className="bg-card p-6">
          <div className="h-5 w-36 animate-pulse rounded bg-muted" />
          <div className="mt-4 grid grid-cols-2 gap-2">
            {[1, 2, 3, 4].map((i) => (
              <div key={i} className="h-14 animate-pulse rounded-lg bg-muted/60" />
            ))}
          </div>
        </AcrylicPanel>
      </div>
    )
  }

  const { date, weekday_label, courses, tasks, exam, pomodoro, phase } = briefing

  return (
    <div className="mx-auto flex w-full max-w-2xl flex-col gap-4 py-6">
      <AcrylicPanel className="bg-card p-5">
        {/* Header */}
        <div className="mb-5 flex items-center justify-between">
          <div>
            <h1 className="font-heading text-lg font-semibold text-foreground">今天</h1>
            <p className="mt-0.5 text-xs text-muted-foreground">
              {date} {weekday_label}
            </p>
          </div>
          <div className="rounded-md border border-border/60 bg-muted/40 px-2.5 py-1 text-right">
            <p className="text-xs font-medium text-foreground">{PHASE_LABELS[phase.phase_type]}</p>
            {phase.current_week && (
              <p className="mt-0.5 text-[10px] text-muted-foreground">
                第 {phase.current_week} 周
              </p>
            )}
          </div>
        </div>

        {/* 4 info blocks in 2-column grid */}
        <div className="grid grid-cols-2 gap-2">
          {/* Courses */}
          <BriefingBlock
            icon={BookOpen}
            iconColor="text-blue-500"
            onClick={() => onNavigate("courses")}
          >
            {courses.today_count > 0 ? (
              <>
                <span className="font-semibold text-foreground">{courses.today_count}</span>
                <span className="text-xs text-muted-foreground"> 节课</span>
                {courses.current_course && (
                  <span className="mt-0.5 block text-[11px] leading-tight text-muted-foreground">
                    当前 {courses.current_course.title} · {courses.current_course.start_time}
                    {courses.current_course.location && ` 在 ${courses.current_course.location}`}
                  </span>
                )}
                {!courses.current_course && courses.next_course && (
                  <span className="mt-0.5 block text-[11px] leading-tight text-muted-foreground">
                    下一节 {courses.next_course.title} · {courses.next_course.start_time}
                    {courses.next_course.location && ` 在 ${courses.next_course.location}`}
                  </span>
                )}
              </>
            ) : (
              <span className="text-xs text-muted-foreground">今天没有课程</span>
            )}
          </BriefingBlock>

          {/* Tasks */}
          <BriefingBlock
            icon={ListTodo}
            iconColor="text-amber-500"
            onClick={() => onNavigate("todo")}
          >
            {tasks.overdue_count === 0 && tasks.due_today_count === 0 && tasks.daily_unfinished_count === 0 ? (
              <span className="text-xs text-muted-foreground">今天暂无到期待办</span>
            ) : (
              <>
                {tasks.overdue_count > 0 && (
                  <span className="block text-xs">
                    <span className="font-semibold text-red-500">{tasks.overdue_count}</span>
                    <span className="text-muted-foreground"> 个逾期</span>
                  </span>
                )}
                {tasks.due_today_count > 0 && (
                  <span className="block text-xs">
                    <span className="font-semibold text-foreground">{tasks.due_today_count}</span>
                    <span className="text-muted-foreground"> 个今天到期</span>
                  </span>
                )}
                {tasks.daily_unfinished_count > 0 && (
                  <span className="block text-xs">
                    <span className="font-semibold text-primary">{tasks.daily_unfinished_count}</span>
                    <span className="text-muted-foreground"> 个每日任务待完成</span>
                  </span>
                )}
                {tasks.spotlight.length > 0 ? (
                  <span className="mt-0.5 block truncate text-[11px] text-muted-foreground">
                    {tasks.spotlight[0].title}
                  </span>
                ) : tasks.daily_spotlight.length > 0 ? (
                  <span className="mt-0.5 block truncate text-[11px] text-muted-foreground">
                    {tasks.daily_spotlight[0].title}
                  </span>
                ) : null}
              </>
            )}
          </BriefingBlock>

          {/* Exam */}
          <BriefingBlock
            icon={CalendarClock}
            iconColor="text-purple-500"
            onClick={() => onNavigate("exams")}
          >
            {exam ? (
              <>
                <span className="block truncate text-xs font-semibold text-foreground">
                  {exam.course_name}
                </span>
                <span className="mt-0.5 block text-[11px] text-muted-foreground">
                  {exam.days_until === 0
                    ? "今天考试"
                    : `剩余 ${exam.days_until} 天`}
                  {formatClock(exam.exam_datetime) && ` · ${formatClock(exam.exam_datetime)}`}
                  {exam.location && ` · ${exam.location}`}
                </span>
              </>
            ) : (
              <span className="text-xs text-muted-foreground">近期没有考试</span>
            )}
          </BriefingBlock>

          {/* Pomodoro */}
          <BriefingBlock
            icon={Timer}
            iconColor="text-emerald-500"
            onClick={() => onNavigate("pomodoro")}
          >
            {pomodoro.is_running ? (
              <>
                <span className="block text-xs font-semibold text-foreground">
                  {pomodoro.phase === "work" ? "专注中" : pomodoro.phase === "short_break" ? "短休息" : "长休息"}
                </span>
                <span className="mt-0.5 block text-[11px] text-muted-foreground">
                  剩余 {formatRemaining(pomodoro.remaining_seconds)}
                </span>
              </>
            ) : (
              <span className="text-xs text-muted-foreground">可以开始今天的第一轮专注</span>
            )}
          </BriefingBlock>
        </div>
      </AcrylicPanel>

      {/* AI 每日摘要（未启用时组件内部返回 null） */}
      <AiBriefCard onNavigate={onNavigate} />
    </div>
  )
}
