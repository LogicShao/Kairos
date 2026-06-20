/** 与后端 schedule::WeekScheduleItem / WeekScheduleResponse 对齐（src-tauri/src/schedule.rs）。 */
export interface WeekScheduleItem {
  kind: "course" | "exam"
  id: number
  title: string
  day_of_week: number
  start_time: string
  end_time: string
  location: string
  teacher: string
  color: string
  notes: string
  week_pattern: string
  course_id: number | null
}

export interface WeekScheduleResponse {
  week_index: number
  semester: string
  semester_start_date: string
  week_start_date: string
  week_end_date: string
  items: WeekScheduleItem[]
}
