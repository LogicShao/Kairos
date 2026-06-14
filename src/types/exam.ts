export interface Exam {
  id: number
  course_name: string
  exam_datetime: string
  location: string
  notes: string
  course_id: number | null
  created_at: string
  updated_at: string
  days_until?: number
}

export interface CreateExamRequest {
  course_name: string
  exam_datetime: string
  location?: string
  notes?: string
  course_id?: number | null
}

export interface UpdateExamRequest {
  course_name?: string
  exam_datetime?: string
  location?: string
  notes?: string
  course_id?: number | null
}
