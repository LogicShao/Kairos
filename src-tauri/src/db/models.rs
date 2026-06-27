use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    pub id: i64,
    /// 全局通知开关，1 = 启用，0 = 禁用。
    pub enabled: bool,
    /// 考试提醒偏移量（JSON 数组，单位分钟），例如 [1440,60] 表示提前 1 天和提前 1 小时。
    pub exam_offsets_json: String,
    /// Android 通知渠道是否已创建（幂等保护）。
    pub android_channel_created: bool,
    /// UTC ISO 8601 创建时间。
    pub created_at: String,
    /// UTC ISO 8601 更新时间。
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateNotificationConfig {
    pub enabled: Option<bool>,
    pub exam_offsets_json: Option<String>,
    pub android_channel_created: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PomodoroConfig {
    pub id: i64,
    pub work_seconds: i64,
    pub short_break_seconds: i64,
    pub long_break_seconds: i64,
    pub sessions_before_long_break: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PomodoroSession {
    pub id: i64,
    /// 跨设备稳定标识（UUID）。合并键，不等于 SQLite id。
    #[serde(default)]
    pub sync_id: String,
    /// UTC ISO 8601 开始时间。
    pub started_at: String,
    /// UTC ISO 8601 结束时间。None 表示 session 尚未正常结束。
    pub ended_at: Option<String>,
    /// "work"、"short_break" 或 "long_break"。
    pub session_type: String,
    /// 关联任务的本地 SQLite id。跨设备同步时由 sync/exporter 重新映射。
    pub task_id: Option<i64>,
    /// 墓碑时间戳。null = 活跃，非 null = 已软删除。
    #[serde(default)]
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePomodoroSessionRequest {
    /// UTC ISO 8601 开始时间，由前端在计时开始时生成。
    pub started_at: String,
    /// "work"、"short_break" 或 "long_break"，与 PomodoroState.phase 对齐。
    pub session_type: String,
    /// 关联任务的本地 SQLite id；None 表示独立番茄钟。
    pub task_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePomodoroConfigRequest {
    pub work_seconds: i64,
    pub short_break_seconds: i64,
    pub long_break_seconds: i64,
    pub sessions_before_long_break: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: i64,
    /// 跨设备稳定标识（UUID）。合并键，不等于 SQLite id。
    #[serde(default)]
    pub sync_id: String,
    pub title: String,
    pub description: String,
    pub status: String,
    pub priority: String,
    /// 截止日期，格式 YYYY-MM-DD；None 表示未设置。
    pub due_date: Option<String>,
    /// JSON 字符串形式的标签数组，前端负责序列化/反序列化。
    pub tags: String,
    /// UTC ISO 8601 创建时间。
    pub created_at: String,
    /// UTC ISO 8601 更新时间，LWW 同步会使用该字段。
    pub updated_at: String,
    /// 墓碑时间戳。null = 活跃，非 null = 已软删除。
    #[serde(default)]
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTaskRequest {
    pub title: String,
    pub description: String,
    pub status: String,
    pub priority: String,
    pub due_date: Option<String>,
    pub tags: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTaskRequest {
    pub title: String,
    pub description: String,
    pub status: String,
    pub priority: String,
    pub due_date: Option<String>,
    pub tags: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Course {
    pub id: i64,
    /// 跨设备稳定标识（UUID）。合并键，不等于 SQLite id。
    #[serde(default)]
    pub sync_id: String,
    pub name: String,
    /// ISO weekday: 1 = 周一，7 = 周日。
    pub day_of_week: i64,
    /// 本地课程开始时间，格式 HH:mm。
    pub start_time: String,
    /// 本地课程结束时间，格式 HH:mm。
    pub end_time: String,
    /// 周次规则文本，例如 "1-17周全周"，由 schedule::matches_week_pattern 解释。
    pub week_pattern: String,
    /// 学期锚点日期，格式 YYYY-MM-DD，用于计算教学周。
    pub semester_start_date: String,
    pub location: String,
    pub teacher: String,
    /// 课程显示颜色，十六进制 RGB 字符串。
    pub color: String,
    /// 学期标识，例如 2026S1。
    pub semester: String,
    /// UTC ISO 8601 创建时间。
    pub created_at: String,
    /// UTC ISO 8601 更新时间，LWW 同步会使用该字段。
    pub updated_at: String,
    /// 墓碑时间戳。null = 活跃，非 null = 已软删除。
    #[serde(default)]
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCourseRequest {
    pub name: String,
    pub day_of_week: i64,
    pub start_time: String,
    pub end_time: String,
    pub week_pattern: String,
    pub semester_start_date: String,
    pub location: String,
    pub teacher: String,
    pub color: String,
    pub semester: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCourseRequest {
    pub name: String,
    pub day_of_week: i64,
    pub start_time: String,
    pub end_time: String,
    pub week_pattern: String,
    pub semester_start_date: String,
    pub location: String,
    pub teacher: String,
    pub color: String,
    pub semester: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Exam {
    pub id: i64,
    /// 跨设备稳定标识（UUID）。合并键，不等于 SQLite id。
    #[serde(default)]
    pub sync_id: String,
    pub course_name: String,
    /// 考试开始时间，RFC3339 字符串。
    pub exam_datetime: String,
    /// 考试结束时间，RFC3339 字符串；空字符串表示未提供结束时间。
    pub exam_end_datetime: String,
    pub location: String,
    pub notes: String,
    /// 关联课程的本地 SQLite id。跨设备同步时由 sync/exporter 重新映射。
    pub course_id: Option<i64>,
    /// 学期标识，例如 2026S1。
    pub semester: String,
    /// UTC ISO 8601 创建时间。
    pub created_at: String,
    /// UTC ISO 8601 更新时间，LWW 同步会使用该字段。
    pub updated_at: String,
    /// 墓碑时间戳。null = 活跃，非 null = 已软删除。
    #[serde(default)]
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateExamRequest {
    pub course_name: String,
    pub exam_datetime: String,
    pub exam_end_datetime: String,
    pub location: String,
    pub notes: String,
    pub course_id: Option<i64>,
    pub semester: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateExamRequest {
    pub course_name: String,
    pub exam_datetime: String,
    pub exam_end_datetime: String,
    pub location: String,
    pub notes: String,
    pub course_id: Option<i64>,
    pub semester: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    pub id: i64,
    pub server_url: String,
    pub username: String,
    pub password: String,
    pub auto_sync: bool,
    /// 上次成功同步时间，UTC ISO 8601；None 表示尚未同步。
    pub last_sync_at: Option<String>,
    /// 上次成功上传后服务端返回的 HTTP ETag。下次上传时通过 If-Match header 发送。
    #[serde(default)]
    pub remote_etag: Option<String>,
    /// 本设备唯一标识（UUID）。用于 trace 快照来源，不参与合并逻辑。
    #[serde(default)]
    pub device_id: Option<String>,
    /// 数据集唯一标识（UUID）。同一同步文件的所有设备共享此值。
    #[serde(default)]
    pub dataset_id: Option<String>,
}
