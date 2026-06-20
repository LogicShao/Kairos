export interface Exam {
  id: number
  course_name: string
  exam_datetime: string
  exam_end_datetime: string
  location: string
  notes: string
  course_id: number | null
  semester: string
  created_at: string
  updated_at: string
  days_until?: number
}

export interface CreateExamRequest {
  course_name: string
  exam_datetime: string
  exam_end_datetime?: string
  location?: string
  notes?: string
  course_id?: number | null
  semester?: string
}

export interface UpdateExamRequest {
  course_name?: string
  exam_datetime?: string
  exam_end_datetime?: string
  location?: string
  notes?: string
  course_id?: number | null
  semester?: string
}
