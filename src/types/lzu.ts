/** 与后端 lzu::models::AuthStatus 对齐（src-tauri/src/lzu/models.rs）。 */
export interface LzuAuthStatus {
  /** 是否已登录。 */
  is_logged_in: boolean
  /** 登录用户名，未登录时为 null。 */
  username?: string | null
}

/** 与后端 commands::lzu::LzuCourseImportResult 对齐（src-tauri/src/commands/lzu.rs）。 */
export interface LzuCourseImportResult {
  /** 从 LZU API 拉取到的课程记录数。 */
  parsed: number
  /** 实际写入数据库的记录数。 */
  imported: number
  /** 因去重跳过的记录数。 */
  skipped: number
  /** 无法映射的课程记录数。 */
  failed: number
  /** 后端生成的中文摘要。 */
  message: string
}
