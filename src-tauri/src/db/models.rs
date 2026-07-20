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

/// 桌面小组件本地配置。id 固定为 1，enabled 表示应用启动时是否恢复窗口。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetConfig {
    pub id: i64,
    /// 1 = 启动并显示 widget 窗口，0 = 不创建/隐藏 widget 窗口。
    pub enabled: bool,
    /// "small"、"medium" 或 "large"，同时决定默认窗口尺寸和渲染布局。
    pub mode: String,
    /// 前端视觉透明度，范围 0.6..=1.0；Tauri 窗口仅负责透明背景。
    pub opacity: f64,
    /// 1 = widget 窗口置顶。
    pub always_on_top: bool,
    /// 1 = 禁止前端拖动保存位置。
    pub locked: bool,
    /// 窗口左上角物理像素 x 坐标；None 表示首次创建使用系统默认位置。
    pub x: Option<i64>,
    /// 窗口左上角物理像素 y 坐标；None 表示首次创建使用系统默认位置。
    pub y: Option<i64>,
    /// 窗口宽度，逻辑像素。
    pub width: i64,
    /// 窗口高度，逻辑像素。
    pub height: i64,
    /// UTC ISO 8601 创建时间。
    pub created_at: String,
    /// UTC ISO 8601 更新时间。
    pub updated_at: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateWidgetConfigRequest {
    pub enabled: Option<bool>,
    pub mode: Option<String>,
    pub opacity: Option<f64>,
    pub always_on_top: Option<bool>,
    pub locked: Option<bool>,
    pub x: Option<i64>,
    pub y: Option<i64>,
    pub width: Option<i64>,
    pub height: Option<i64>,
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

/// 番茄钟运行态持久化记录，用于应用重启后恢复本日状态。
/// 每个应用实例最多一条记录（id = 1），date_key 跨天时被视为过期。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PomodoroRuntimeState {
    pub id: i64,
    /// 当前阶段："work" / "short_break" / "long_break"。
    pub phase: String,
    /// 当前阶段剩余秒数。
    pub remaining_seconds: i64,
    /// 当前阶段总秒数（用于进度环计算）。
    pub total_seconds: i64,
    /// 1 = 计时器正在运行，0 = 已暂停。
    pub is_running: bool,
    /// 当前活跃 work session 的数据库 id；休息阶段可为 NULL。
    pub active_session_id: Option<i64>,
    /// 本地日期 YYYY-MM-DD，用于判断状态是否属于今天。
    pub date_key: String,
    /// 上次更新时的 UTC ISO 8601 时间戳。
    pub last_seen_at: String,
    /// 1 = 上次退出时处于运行中，需要用户处理中断。
    pub interrupted: bool,
    /// UTC ISO 8601 创建时间。
    pub created_at: String,
    /// UTC ISO 8601 更新时间。
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePomodoroConfigRequest {
    pub work_seconds: i64,
    pub short_break_seconds: i64,
    pub long_break_seconds: i64,
    pub sessions_before_long_break: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PomodoroProfile {
    pub id: i64,
    pub name: String,
    pub work_seconds: i64,
    pub short_break_seconds: i64,
    pub long_break_seconds: i64,
    pub sessions_before_long_break: i64,
    /// 1 = built-in profile, cannot be deleted.
    pub is_builtin: bool,
    /// UTC ISO 8601 创建时间。
    pub created_at: String,
    /// UTC ISO 8601 更新时间。
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePomodoroProfileRequest {
    pub name: String,
    pub work_seconds: i64,
    pub short_break_seconds: i64,
    pub long_break_seconds: i64,
    pub sessions_before_long_break: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePomodoroProfileRequest {
    pub name: String,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SemesterContext {
    pub id: i64,
    /// 学期上下文来源，例如 "lzu"。
    pub source: String,
    /// LZU 学年 xn，缺失时不从课程反推。
    pub academic_year: Option<String>,
    /// LZU 学期 xqm / xq，缺失时不从课程反推。
    pub term: Option<String>,
    /// 与课程 semester 对齐的本地学期标识，例如 2026S1。
    pub term_label: String,
    /// 学期锚点日期，格式 YYYY-MM-DD。
    pub start_date: String,
    /// LZU 导入时返回的当前周，仅作刷新参考；自然当前周仍由日期推导。
    pub current_week: Option<i64>,
    /// LZU 总周次 zzx。接口缺失时保持 None。
    pub total_weeks: Option<i64>,
    /// UTC ISO 8601 最后刷新时间。
    pub refreshed_at: String,
    /// UTC ISO 8601 创建时间。
    pub created_at: String,
    /// UTC ISO 8601 更新时间。
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpsertSemesterContextRequest {
    pub source: String,
    pub academic_year: Option<String>,
    pub term: Option<String>,
    pub term_label: String,
    pub start_date: String,
    pub current_week: Option<i64>,
    pub total_weeks: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TermPhase {
    pub id: i64,
    /// 跨设备稳定标识（UUID）。合并键，不等于 SQLite id。
    #[serde(default)]
    pub sync_id: String,
    /// 学期标识，例如 2026S1，与 semester_context.term_label 对齐。
    pub term_label: String,
    /// "teaching"、"exam"、"break"。
    pub phase_type: String,
    /// 起始教学周，最小为 1。
    pub start_week: i64,
    /// 结束教学周，必须大于等于 start_week。
    pub end_week: i64,
    /// 1 = 课程在该阶段可见。
    pub affects_courses: bool,
    /// 1 = 考试通知在该阶段可用。
    pub affects_exam_notifications: bool,
    /// 关联 pomodoro profile 名称。
    pub pomodoro_profile: String,
    /// 预留通知规则 JSON，默认 {}。
    pub notification_rules_json: String,
    /// 同一学期内展示/匹配排序。
    pub sort_order: i64,
    /// 墓碑时间戳。null = 活跃，非 null = 已软删除。
    #[serde(default)]
    pub deleted_at: Option<String>,
    /// UTC ISO 8601 创建时间。
    pub created_at: String,
    /// UTC ISO 8601 更新时间，LWW 同步会使用该字段。
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTermPhaseRequest {
    pub term_label: String,
    pub phase_type: String,
    pub start_week: i64,
    pub end_week: i64,
    pub affects_courses: bool,
    pub affects_exam_notifications: bool,
    pub pomodoro_profile: String,
    pub notification_rules_json: String,
    pub sort_order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTermPhaseRequest {
    pub term_label: String,
    pub phase_type: String,
    pub start_week: i64,
    pub end_week: i64,
    pub affects_courses: bool,
    pub affects_exam_notifications: bool,
    pub pomodoro_profile: String,
    pub notification_rules_json: String,
    pub sort_order: i64,
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
