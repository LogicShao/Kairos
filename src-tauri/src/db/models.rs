use serde::{Deserialize, Serialize};

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
    pub started_at: String,
    pub ended_at: Option<String>,
    pub session_type: String,
    pub task_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePomodoroSessionRequest {
    pub started_at: String,
    pub session_type: String,
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
    pub title: String,
    pub description: String,
    pub status: String,
    pub priority: String,
    pub due_date: Option<String>,
    pub tags: String,
    pub created_at: String,
    pub updated_at: String,
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
    pub created_at: String,
    pub updated_at: String,
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
    pub course_name: String,
    pub exam_datetime: String,
    pub exam_end_datetime: String,
    pub location: String,
    pub notes: String,
    pub course_id: Option<i64>,
    pub semester: String,
    pub created_at: String,
    pub updated_at: String,
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
    pub last_sync_at: Option<String>,
}
