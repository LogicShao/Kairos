/** 与后端 db::models::NotificationConfig 对齐，是本地通知的全局配置记录。 */
export interface NotificationConfig {
  id: number
  /** 全局通知开关，true = 启用。 */
  enabled: boolean
  /** 考试提醒偏移量（JSON 数组，单位分钟），例如 [1440,60] 表示提前 1 天和提前 1 小时。 */
  exam_offsets_json: string
  /** Android 通知渠道是否已创建（幂等保护）。 */
  android_channel_created: boolean
  /** UTC ISO 8601 创建时间。 */
  created_at: string
  /** UTC ISO 8601 更新时间。 */
  updated_at: string
}

/** 与后端 db::models::UpdateNotificationConfig 对齐，所有字段可选以支持局部更新。 */
export interface UpdateNotificationConfig {
  enabled?: boolean
  exam_offsets_json?: string
  android_channel_created?: boolean
}
