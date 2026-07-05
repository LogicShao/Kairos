import type { ReactNode } from "react"
import { ArrowUpRight, BookOpen, CheckSquare, GraduationCap, Timer } from "lucide-react"
import type { TodayBriefingResponse } from "@/types/briefing"
import type { PomodoroState } from "@/types/pomodoro"
import type { MainNavigationTarget } from "@/types/widget"
import { formatDuration, formatExamTime, phaseLabel } from "@/components/widget/widget-format"

interface LargeWidgetProps {
  briefing: TodayBriefingResponse
  pomodoro: PomodoroState
  onOpen: (target: MainNavigationTarget) => void
}

interface SummaryButtonProps {
  title: string
  value: string
  detail: string
  target: MainNavigationTarget
  icon: ReactNode
  onOpen: (target: MainNavigationTarget) => void
}

function SummaryButton({ title, value, detail, target, icon, onOpen }: SummaryButtonProps) {
  return (
    <button
      type="button"
      onClick={() => onOpen(target)}
      className="flex min-h-[4.35rem] items-start gap-2 rounded-md border border-border/50 bg-muted/20 px-3 py-2 text-left transition-colors hover:bg-muted/40"
    >
      <span className="mt-0.5 flex h-7 w-7 shrink-0 items-center justify-center rounded-md bg-primary/10 text-primary">
        {icon}
      </span>
      <span className="min-w-0 flex-1">
        <span className="flex items-center justify-between gap-2 text-[11px] font-medium text-muted-foreground">
          {title}
          <ArrowUpRight className="h-3 w-3 shrink-0" />
        </span>
        <span className="mt-0.5 block truncate text-sm font-semibold text-foreground">{value}</span>
        <span className="mt-0.5 block truncate text-[11px] text-muted-foreground">{detail}</span>
      </span>
    </button>
  )
}

export function LargeWidget({ briefing, pomodoro, onOpen }: LargeWidgetProps) {
  const currentCourse = briefing.courses.current_course
  const nextCourse = briefing.courses.next_course
  const courseDetail = currentCourse
    ? `当前 ${currentCourse.start_time} ${currentCourse.title}`
    : nextCourse
      ? `${nextCourse.start_time} ${nextCourse.title}`
      : "今日暂无后续课程"
  const taskCount = briefing.tasks.overdue_count + briefing.tasks.due_today_count
  const firstTask = briefing.tasks.spotlight[0]
  const exam = briefing.exam

  return (
    <div className="grid h-full grid-rows-[auto_1fr] gap-3 px-4 py-3">
      <div className="flex items-center justify-between gap-3">
        <div className="min-w-0">
          <div className="text-sm font-semibold text-foreground">今天</div>
          <div className="truncate text-[11px] text-muted-foreground">
            {briefing.date} · {briefing.weekday_label}
          </div>
        </div>
        <button
          type="button"
          onClick={() => onOpen("today")}
          className="rounded-md border border-border/60 px-2.5 py-1 text-[11px] text-muted-foreground hover:bg-muted/40 hover:text-foreground"
        >
          概览
        </button>
      </div>

      <div className="grid min-h-0 grid-cols-2 gap-2">
        <SummaryButton
          title="课程"
          value={`${briefing.courses.today_count} 节`}
          detail={courseDetail}
          target="courses"
          icon={<BookOpen className="h-4 w-4" />}
          onOpen={onOpen}
        />
        <SummaryButton
          title="待办"
          value={`${taskCount} 项`}
          detail={firstTask ? firstTask.title : "无今日到期或逾期"}
          target="todo"
          icon={<CheckSquare className="h-4 w-4" />}
          onOpen={onOpen}
        />
        <SummaryButton
          title="考试"
          value={exam ? `${exam.days_until} 天后` : "暂无"}
          detail={exam ? `${exam.course_name} · ${formatExamTime(exam.exam_datetime)}` : "没有未来考试"}
          target="exams"
          icon={<GraduationCap className="h-4 w-4" />}
          onOpen={onOpen}
        />
        <SummaryButton
          title="专注"
          value={formatDuration(pomodoro.remaining_seconds)}
          detail={`${phaseLabel(pomodoro.phase)} · 已完成 ${pomodoro.completed_sessions} 轮`}
          target="pomodoro"
          icon={<Timer className="h-4 w-4" />}
          onOpen={onOpen}
        />
      </div>
    </div>
  )
}
