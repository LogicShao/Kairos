export interface Course {
  id: number
  name: string
  day_of_week: number
  start_time: string
  end_time: string
  week_pattern: string
  semester_start_date: string
  location: string
  teacher: string
  color: string
  semester: string
  created_at: string
  updated_at: string
}

export interface CreateCourseRequest {
  name: string
  day_of_week: number
  start_time: string
  end_time: string
  week_pattern?: string
  semester_start_date?: string
  location?: string
  teacher?: string
  color?: string
  semester?: string
}

export interface UpdateCourseRequest {
  name?: string
  day_of_week?: number
  start_time?: string
  end_time?: string
  week_pattern?: string
  semester_start_date?: string
  location?: string
  teacher?: string
  color?: string
  semester?: string
}

export interface CourseFilterParams {
  semester?: string | null
}
