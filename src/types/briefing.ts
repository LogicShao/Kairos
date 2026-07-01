/** 与后端 commands::briefing::TodayBriefingResponse 对齐。 */

export interface NextCourse {
  title: string
  start_time: string
  end_time: string
  location: string
}

export interface TodayCourses {
  today_count: number
  next_course: NextCourse | null
}

export interface TaskSpotlight {
  id: number
  title: string
  priority: "high" | "medium" | "low"
  due_date: string | null
}

export interface TodayTasks {
  overdue_count: number
  due_today_count: number
  spotlight: TaskSpotlight[]
}

export interface UpcomingExam {
  id: number
  course_name: string
  exam_datetime: string
  days_until: number
  location: string
}

export interface PomodoroBriefing {
  is_running: boolean
  phase: "work" | "short_break" | "long_break"
  remaining_seconds: number
  completed_sessions: number
}

/** get_today_briefing 命令的完整响应。 */
export interface TodayBriefingResponse {
  date: string
  weekday_label: string
  courses: TodayCourses
  tasks: TodayTasks
  exam: UpcomingExam | null
  pomodoro: PomodoroBriefing
}
