export type TaskStatus = "todo" | "in_progress" | "done"
export type TaskPriority = "high" | "medium" | "low"

export interface Task {
  id: number
  title: string
  description: string
  status: TaskStatus
  priority: TaskPriority
  due_date: string | null
  tags: string
  created_at: string
  updated_at: string
}

export interface CreateTaskRequest {
  title: string
  description?: string
  status?: TaskStatus
  priority?: TaskPriority
  due_date?: string | null
  tags?: string
}

export interface UpdateTaskRequest {
  title?: string
  description?: string
  status?: TaskStatus
  priority?: TaskPriority
  due_date?: string | null
  tags?: string
}

export interface TaskFilterParams {
  status_filter?: string | null
  priority_filter?: string | null
  sort_by?: string | null
  sort_order?: string | null
}
