export type TaskStatus = "todo" | "in_progress" | "done"
export type TaskPriority = "high" | "medium" | "low"

/** 与后端 db::models::Task 对齐，是 get_all_tasks / get_task 返回的活跃任务实体。 */
export interface Task {
  id: number
  /** 跨设备稳定标识（UUID）。同步合并键，不等于本地 SQLite id。 */
  sync_id: string
  title: string
  description: string
  status: TaskStatus
  priority: TaskPriority
  /** 截止日期，来自 HTML date 输入，格式为 YYYY-MM-DD；null 表示未设置。 */
  due_date: string | null
  /** JSON 字符串形式的标签数组，由前端表单负责序列化/反序列化。 */
  tags: string
  /** UTC ISO 8601 创建时间，由后端 db::chrono_now 生成。 */
  created_at: string
  /** UTC ISO 8601 更新时间。LWW 同步以此字段比较胜负（墓碑优先取 deleted_at）。 */
  updated_at: string
  /** 墓碑时间戳。正常列表查询只返回 null；非 null 表示已软删除并参与同步传播。 */
  deleted_at: string | null
}

/** create_task 命令入参；可省略字段由 Rust command 使用默认值补齐。 */
export interface CreateTaskRequest {
  title: string
  description?: string
  status?: TaskStatus
  priority?: TaskPriority
  /** YYYY-MM-DD 或 null；省略/空值都会创建为无截止日期。 */
  due_date?: string | null
  /** JSON 字符串形式的标签数组。 */
  tags?: string
}

/** update_task 命令入参；省略字段表示沿用当前任务值。 */
export interface UpdateTaskRequest {
  title?: string
  description?: string
  status?: TaskStatus
  priority?: TaskPriority
  /** 当前后端无法区分“省略”和“清空”，null 会被视为未提供。 */
  due_date?: string | null
  /** JSON 字符串形式的标签数组。 */
  tags?: string
}

/** get_all_tasks 过滤与排序参数，字段名与 commands::tasks::TaskFilterParams 对齐。 */
export interface TaskFilterParams {
  status_filter?: string | null
  priority_filter?: string | null
  sort_by?: string | null
  sort_order?: string | null
}
