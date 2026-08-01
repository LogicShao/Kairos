import type { NextCourse, TodayBriefingResponse } from "@/types/briefing"
import type { MainNavigationTarget } from "@/types/widget"

export interface WidgetNextItem {
  label: string
  title: string
  detail: string
  target: MainNavigationTarget
}

export function formatDuration(totalSeconds: number): string {
  const safeSeconds = Math.max(0, Math.floor(totalSeconds))
  const minutes = Math.floor(safeSeconds / 60)
  const seconds = safeSeconds % 60
  return `${minutes.toString().padStart(2, "0")}:${seconds.toString().padStart(2, "0")}`
}

export function phaseLabel(phase: string): string {
  switch (phase) {
    case "work":
      return "专注"
    case "short_break":
      return "短休息"
    case "long_break":
      return "长休息"
    default:
      return "计时"
  }
}

export function formatExamTime(value: string): string {
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) {
    return value
  }
  return date.toLocaleString("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  })
}

function courseWidgetItem(label: string, course: NextCourse): WidgetNextItem {
  return {
    label,
    title: course.title,
    detail: `${course.start_time}-${course.end_time}${course.location ? ` · ${course.location}` : ""}`,
    target: "courses",
  }
}

export function getNextWidgetItem(briefing: TodayBriefingResponse): WidgetNextItem {
  if (briefing.courses.current_course) {
    return courseWidgetItem("当前课程", briefing.courses.current_course)
  }

  if (briefing.courses.next_course) {
    return courseWidgetItem("下一节课", briefing.courses.next_course)
  }

  if (briefing.exam) {
    return {
      label: "最近考试",
      title: briefing.exam.course_name,
      detail: `${briefing.exam.days_until} 天后 · ${formatExamTime(briefing.exam.exam_datetime)}`,
      target: "exams",
    }
  }

  const task = briefing.tasks.spotlight[0]
  if (task) {
    return {
      label: task.due_date === briefing.date ? "今日到期" : "待办提醒",
      title: task.title,
      detail: task.due_date ? `截止 ${task.due_date}` : "无截止日期",
      target: "todo",
    }
  }

  return {
    label: briefing.pomodoro.is_running ? "专注中" : "今天",
    title: briefing.pomodoro.is_running ? phaseLabel(briefing.pomodoro.phase) : "暂无紧急事项",
    detail: briefing.pomodoro.is_running
      ? `${formatDuration(briefing.pomodoro.remaining_seconds)} 剩余`
      : `${briefing.weekday_label} · ${briefing.date}`,
    target: briefing.pomodoro.is_running ? "pomodoro" : "today",
  }
}
