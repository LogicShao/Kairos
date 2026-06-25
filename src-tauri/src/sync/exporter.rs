//! Sync exporter: v2 快照协议的导出和导入。
//!
//! 导出: 从本地 SQLite 读取全部实体（含墓碑），序列化为 v2 JSON。
//! 导入: 解析远端快照（v1/v2 兼容），按 sync_id 匹配本地实体，LWW 决定保留哪一方。
//!
//! 合并语义:
//! - 匹配键: sync_id (v1 兼容回退: 按 SQLite id 匹配)
//! - 胜负: effective_timestamp = deleted_at ?? updated_at
//! - 墓碑: 软删除通过 deleted_at 传播，不物理删除数据

use std::collections::HashMap;

use rusqlite::{params, OptionalExtension, Result, Row, Transaction};
use serde::{Deserialize, Serialize};

use crate::db::models::{Course, Exam, PomodoroSession, Task};

/// v2 快照 JSON 的顶层结构。
/// schema_version = 2 表示新格式（含 sync_id + 墓碑），v1 旧格式兼容导入。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncData {
    /// 快照格式版本。2 = 当前，1 = 旧格式（无 sync_id，仍可导入）。
    #[serde(default = "default_schema_version")]
    pub schema_version: i64,
    /// 数据集 UUID，所有设备共享同一值。
    #[serde(default)]
    pub dataset_id: String,
    /// 导出设备 UUID，仅用于 trace 来源。
    #[serde(default)]
    pub device_id: String,
    pub tasks: Vec<Task>,
    pub courses: Vec<Course>,
    pub exams: Vec<Exam>,
    pub pomodoro_sessions: Vec<PomodoroSession>,
    /// 快照导出时间（ISO 8601），同时作为 last_sync_at 写入本地。
    pub exported_at: String,
}

fn default_schema_version() -> i64 {
    // v1 旧格式默认值，新快照始终显式设为 2
    1
}

/// 单次同步的实体变更统计。
/// merged: 成功写入本地的实体数（新增 + 被远端覆盖的更新）。
/// conflicts: 被拒绝的远端实体数 = sum(远端实体总数) - merged。
/// 注意: conflicts 含义是"本地版本较新，远端更新被忽略"，不是传统意义的"编辑冲突"。
#[derive(Debug, Clone, Serialize)]
pub struct SyncStats {
    /// 本次同步成功写入本地数据库的任务数（新增 + 被远端覆盖的更新）。
    pub tasks_merged: usize,
    /// 本次同步成功写入本地数据库的课程数。
    pub courses_merged: usize,
    /// 本次同步成功写入本地数据库的考试数。
    pub exams_merged: usize,
    /// 本次同步成功写入本地数据库的番茄钟 session 数。
    pub sessions_merged: usize,
    /// 被拒绝的远端实体数（本地版本更新或相等时保留本地）。
    pub conflicts: usize,
}

/// sync_now 命令的返回结构，前端 SyncSettings 组件展示此数据。
#[derive(Debug, Clone, Serialize)]
pub struct SyncResult {
    /// 本次是否成功上传到远端。
    pub uploaded: bool,
    /// 本次是否下载了远端数据（404 = false）。
    pub downloaded: bool,
    /// 本次下载合并和 ETag 冲突重试合并的累计统计。
    pub stats: SyncStats,
}

pub fn export_all(conn: &rusqlite::Connection) -> Result<SyncData> {
    let config = crate::db::sync::get_sync_config(conn)?;
    let tasks = export_tasks(conn)?;
    let courses = export_courses(conn)?;
    let exams = export_exams(conn)?;
    let pomodoro_sessions = export_pomodoro_sessions(conn)?;
    let exported_at = crate::db::chrono_now();

    Ok(SyncData {
        schema_version: 2,
        dataset_id: sync_config_value(&config.dataset_id),
        device_id: sync_config_value(&config.device_id),
        tasks,
        courses,
        exams,
        pomodoro_sessions,
        exported_at,
    })
}

pub fn import_all(conn: &mut rusqlite::Connection, data: &SyncData) -> Result<SyncStats> {
    let tx = conn.transaction()?;

    let tasks_merged = merge_tasks(&tx, &data.tasks)?;
    let task_id_map = build_entity_id_map(
        &tx,
        data.tasks
            .iter()
            .map(|t| (t.id, normalized_task(t).sync_id))
            .collect(),
        "tasks",
    )?;
    let courses_merged = merge_courses(&tx, &data.courses)?;
    let course_id_map = build_entity_id_map(
        &tx,
        data.courses
            .iter()
            .map(|c| (c.id, normalized_course(c).sync_id))
            .collect(),
        "courses",
    )?;
    let exams_merged = merge_exams(&tx, &data.exams, &course_id_map)?;
    let sessions_merged = merge_pomodoro_sessions(&tx, &data.pomodoro_sessions, &task_id_map)?;
    tx.commit()?;

    let conflicts = (data.tasks.len().saturating_sub(tasks_merged))
        + (data.courses.len().saturating_sub(courses_merged))
        + (data.exams.len().saturating_sub(exams_merged))
        + (data.pomodoro_sessions.len().saturating_sub(sessions_merged));

    Ok(SyncStats {
        tasks_merged,
        courses_merged,
        exams_merged,
        sessions_merged,
        conflicts,
    })
}

fn sync_config_value(value: &Option<String>) -> String {
    value.clone().unwrap_or_default()
}

// 规则 4：四次相同 export 模式 → 宏消除重复
macro_rules! export_entity {
    ($name:ident, $ty:ident, $sql:expr, $($field:ident : $idx:expr),+ $(,)?) => {
        fn $name(conn: &rusqlite::Connection) -> Result<Vec<$ty>> {
            let mut stmt = conn.prepare($sql)?;
            let rows = stmt.query_map([], |row| {
                Ok($ty {
                    $($field: row.get($idx)?),+
                })
            })?;
            rows.collect()
        }
    };
}

export_entity!(export_tasks, Task,
    "SELECT id, sync_id, title, description, status, priority, due_date, tags, created_at, updated_at, deleted_at FROM tasks ORDER BY id",
    id: 0, sync_id: 1, title: 2, description: 3, status: 4, priority: 5,
    due_date: 6, tags: 7, created_at: 8, updated_at: 9, deleted_at: 10,
);

export_entity!(export_courses, Course,
    "SELECT id, sync_id, name, day_of_week, start_time, end_time, week_pattern, semester_start_date, location, teacher, color, semester, created_at, updated_at, deleted_at FROM courses ORDER BY id",
    id: 0, sync_id: 1, name: 2, day_of_week: 3, start_time: 4, end_time: 5,
    week_pattern: 6, semester_start_date: 7, location: 8, teacher: 9, color: 10,
    semester: 11, created_at: 12, updated_at: 13, deleted_at: 14,
);

export_entity!(export_exams, Exam,
    "SELECT id, sync_id, course_name, exam_datetime, exam_end_datetime, location, notes, course_id, semester, created_at, updated_at, deleted_at FROM exams ORDER BY id",
    id: 0, sync_id: 1, course_name: 2, exam_datetime: 3, exam_end_datetime: 4,
    location: 5, notes: 6, course_id: 7, semester: 8, created_at: 9, updated_at: 10,
    deleted_at: 11,
);

export_entity!(export_pomodoro_sessions, PomodoroSession,
    "SELECT id, sync_id, started_at, ended_at, session_type, task_id, deleted_at FROM pomodoro_sessions ORDER BY id",
    id: 0, sync_id: 1, started_at: 2, ended_at: 3, session_type: 4, task_id: 5,
    deleted_at: 6,
);

fn merge_tasks(tx: &Transaction<'_>, remote: &[Task]) -> Result<usize> {
    let mut merged = 0usize;

    for task in remote {
        let is_legacy = task.sync_id.is_empty();
        let remote = normalized_task(task);
        let local =
            find_local_meta::<LocalMeta>(tx, "tasks", &remote.sync_id, remote.id, is_legacy)?;

        match local {
            None => {
                tx.execute(
                    "INSERT INTO tasks (sync_id, title, description, status, priority, due_date, tags, created_at, updated_at, deleted_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    params![
                        remote.sync_id,
                        remote.title,
                        remote.description,
                        remote.status,
                        remote.priority,
                        remote.due_date,
                        remote.tags,
                        remote.created_at,
                        remote.updated_at,
                        remote.deleted_at,
                    ],
                )?;
                merged += 1;
            }
            Some(local) => {
                let newer =
                    remote_effective_timestamp(&remote.updated_at, remote.deleted_at.as_deref())
                        > local.effective_timestamp();
                if resolve_merge(tx, "tasks", &local, &remote.sync_id, newer, |tx| {
                    tx.execute(
                        "UPDATE tasks
                         SET title = ?1, description = ?2, status = ?3, priority = ?4,
                             due_date = ?5, tags = ?6, created_at = ?7, updated_at = ?8,
                             deleted_at = ?9, sync_id = ?10
                         WHERE id = ?11",
                        params![
                            remote.title,
                            remote.description,
                            remote.status,
                            remote.priority,
                            remote.due_date,
                            remote.tags,
                            remote.created_at,
                            remote.updated_at,
                            remote.deleted_at,
                            remote.sync_id,
                            local.id,
                        ],
                    )
                    .map(|_| ())
                })? {
                    merged += 1;
                }
            }
        }
    }

    Ok(merged)
}

fn merge_courses(tx: &Transaction<'_>, remote: &[Course]) -> Result<usize> {
    let mut merged = 0usize;

    for course in remote {
        let is_legacy = course.sync_id.is_empty();
        let remote = normalized_course(course);
        let local =
            find_local_meta::<LocalMeta>(tx, "courses", &remote.sync_id, remote.id, is_legacy)?;

        match local {
            None => {
                tx.execute(
                    "INSERT INTO courses (sync_id, name, day_of_week, start_time, end_time, week_pattern, semester_start_date, location, teacher, color, semester, created_at, updated_at, deleted_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                    params![
                        remote.sync_id,
                        remote.name,
                        remote.day_of_week,
                        remote.start_time,
                        remote.end_time,
                        remote.week_pattern,
                        remote.semester_start_date,
                        remote.location,
                        remote.teacher,
                        remote.color,
                        remote.semester,
                        remote.created_at,
                        remote.updated_at,
                        remote.deleted_at,
                    ],
                )?;
                merged += 1;
            }
            Some(local) => {
                let newer =
                    remote_effective_timestamp(&remote.updated_at, remote.deleted_at.as_deref())
                        > local.effective_timestamp();
                if resolve_merge(tx, "courses", &local, &remote.sync_id, newer, |tx| {
                    tx.execute(
                        "UPDATE courses
                         SET name = ?1, day_of_week = ?2, start_time = ?3, end_time = ?4,
                             week_pattern = ?5, semester_start_date = ?6, location = ?7, teacher = ?8,
                             color = ?9, semester = ?10, created_at = ?11, updated_at = ?12,
                             deleted_at = ?13, sync_id = ?14
                         WHERE id = ?15",
                        params![
                            remote.name,
                            remote.day_of_week,
                            remote.start_time,
                            remote.end_time,
                            remote.week_pattern,
                            remote.semester_start_date,
                            remote.location,
                            remote.teacher,
                            remote.color,
                            remote.semester,
                            remote.created_at,
                            remote.updated_at,
                            remote.deleted_at,
                            remote.sync_id,
                            local.id,
                        ],
                    ).map(|_| ())
                })? {
                    merged += 1;
                }
            }
        }
    }

    Ok(merged)
}

fn merge_exams(
    tx: &Transaction<'_>,
    remote: &[Exam],
    course_id_map: &HashMap<i64, i64>,
) -> Result<usize> {
    let mut merged = 0usize;

    for exam in remote {
        let is_legacy = exam.sync_id.is_empty();
        let remote = normalized_exam(exam);
        let course_id = map_remote_entity_id(course_id_map, remote.course_id);
        let local =
            find_local_meta::<LocalMeta>(tx, "exams", &remote.sync_id, remote.id, is_legacy)?;

        match local {
            None => {
                tx.execute(
                    "INSERT INTO exams (sync_id, course_name, exam_datetime, exam_end_datetime, location, notes, course_id, semester, created_at, updated_at, deleted_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                    params![
                        remote.sync_id,
                        remote.course_name,
                        remote.exam_datetime,
                        remote.exam_end_datetime,
                        remote.location,
                        remote.notes,
                        course_id,
                        remote.semester,
                        remote.created_at,
                        remote.updated_at,
                        remote.deleted_at,
                    ],
                )?;
                merged += 1;
            }
            Some(local) => {
                let newer =
                    remote_effective_timestamp(&remote.updated_at, remote.deleted_at.as_deref())
                        > local.effective_timestamp();
                if resolve_merge(tx, "exams", &local, &remote.sync_id, newer, |tx| {
                    tx.execute(
                        "UPDATE exams
                         SET course_name = ?1, exam_datetime = ?2, exam_end_datetime = ?3, location = ?4,
                             notes = ?5, course_id = ?6, semester = ?7, created_at = ?8, updated_at = ?9,
                             deleted_at = ?10, sync_id = ?11
                         WHERE id = ?12",
                        params![
                            remote.course_name,
                            remote.exam_datetime,
                            remote.exam_end_datetime,
                            remote.location,
                            remote.notes,
                            course_id,
                            remote.semester,
                            remote.created_at,
                            remote.updated_at,
                            remote.deleted_at,
                            remote.sync_id,
                            local.id,
                        ],
                    ).map(|_| ())
                })? {
                    merged += 1;
                }
            }
        }
    }

    Ok(merged)
}

fn merge_pomodoro_sessions(
    tx: &Transaction<'_>,
    remote: &[PomodoroSession],
    task_id_map: &HashMap<i64, i64>,
) -> Result<usize> {
    let mut merged = 0usize;

    for session in remote {
        let is_legacy = session.sync_id.is_empty();
        let remote = normalized_session(session);
        let task_id = map_remote_entity_id(task_id_map, remote.task_id);
        let local = find_local_meta::<SessionMeta>(
            tx,
            "pomodoro_sessions",
            &remote.sync_id,
            remote.id,
            is_legacy,
        )?;

        match local {
            None => {
                tx.execute(
                    "INSERT INTO pomodoro_sessions (sync_id, started_at, ended_at, session_type, task_id, deleted_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        remote.sync_id,
                        remote.started_at,
                        remote.ended_at,
                        remote.session_type,
                        task_id,
                        remote.deleted_at,
                    ],
                )?;
                merged += 1;
            }
            Some(local) => {
                let newer =
                    remote_session_effective_timestamp(&remote) > local.effective_timestamp();
                if resolve_merge(
                    tx,
                    "pomodoro_sessions",
                    &local,
                    &remote.sync_id,
                    newer,
                    |tx| {
                        tx.execute(
                            "UPDATE pomodoro_sessions
                         SET started_at = ?1, ended_at = ?2, session_type = ?3, task_id = ?4,
                             deleted_at = ?5, sync_id = ?6
                         WHERE id = ?7",
                            params![
                                remote.started_at,
                                remote.ended_at,
                                remote.session_type,
                                task_id,
                                remote.deleted_at,
                                remote.sync_id,
                                local.id,
                            ],
                        )
                        .map(|_| ())
                    },
                )? {
                    merged += 1;
                }
            }
        }
    }

    Ok(merged)
}

/// 统一的元数据行接口，消除 find_local_meta / find_local_session_meta 重复
trait MetaRow: Sized {
    fn columns() -> &'static str;
    fn from_row(row: &Row<'_>) -> Result<Self>;
    fn id(&self) -> i64;
    fn sync_id(&self) -> &str;
    fn effective_timestamp(&self) -> &str;
}

fn find_local_meta<M: MetaRow>(
    tx: &Transaction<'_>,
    table: &str,
    sync_id: &str,
    remote_id: i64,
    allow_id_fallback: bool,
) -> Result<Option<M>> {
    let by_sync_id = tx
        .query_row(
            &format!("SELECT {} FROM {table} WHERE sync_id = ?1", M::columns()),
            params![sync_id],
            M::from_row,
        )
        .optional()?;

    if by_sync_id.is_some() || !allow_id_fallback {
        return Ok(by_sync_id);
    }

    tx.query_row(
        &format!("SELECT {} FROM {table} WHERE id = ?1", M::columns()),
        params![remote_id],
        M::from_row,
    )
    .optional()
}

#[derive(Debug)]
struct LocalMeta {
    id: i64,
    sync_id: String,
    updated_at: String,
    deleted_at: Option<String>,
}

impl MetaRow for LocalMeta {
    fn columns() -> &'static str {
        "id, sync_id, updated_at, deleted_at"
    }
    fn from_row(row: &Row<'_>) -> Result<Self> {
        Ok(LocalMeta {
            id: row.get(0)?,
            sync_id: row.get(1)?,
            updated_at: row.get(2)?,
            deleted_at: row.get(3)?,
        })
    }
    fn id(&self) -> i64 {
        self.id
    }
    fn sync_id(&self) -> &str {
        &self.sync_id
    }
    fn effective_timestamp(&self) -> &str {
        remote_effective_timestamp(&self.updated_at, self.deleted_at.as_deref())
    }
}

#[derive(Debug)]
struct SessionMeta {
    id: i64,
    sync_id: String,
    ended_at: Option<String>,
    deleted_at: Option<String>,
}

impl MetaRow for SessionMeta {
    fn columns() -> &'static str {
        "id, sync_id, ended_at, deleted_at"
    }
    fn from_row(row: &Row<'_>) -> Result<Self> {
        Ok(SessionMeta {
            id: row.get(0)?,
            sync_id: row.get(1)?,
            ended_at: row.get(2)?,
            deleted_at: row.get(3)?,
        })
    }
    fn id(&self) -> i64 {
        self.id
    }
    fn sync_id(&self) -> &str {
        &self.sync_id
    }
    fn effective_timestamp(&self) -> &str {
        self.deleted_at
            .as_deref()
            .or(self.ended_at.as_deref())
            .unwrap_or("")
    }
}

/// 统一的冲突解决：远端时间戳更新 → 执行 do_update；否则 sync_id 回填。
/// 返回 true 表示执行了变更（update 或 sync_id 回填），用于 merged 计数。
fn resolve_merge<M: MetaRow>(
    tx: &Transaction<'_>,
    table: &str,
    local: &M,
    remote_sync_id: &str,
    remote_is_newer: bool,
    do_update: impl FnOnce(&Transaction<'_>) -> Result<()>,
) -> Result<bool> {
    if remote_is_newer {
        do_update(tx)?;
        return Ok(true);
    }
    if local.sync_id() != remote_sync_id {
        update_local_sync_id(tx, table, local.id(), remote_sync_id)?;
        return Ok(true);
    }
    Ok(false)
}

fn update_local_sync_id(
    tx: &Transaction<'_>,
    table: &str,
    local_id: i64,
    sync_id: &str,
) -> Result<()> {
    tx.execute(
        &format!("UPDATE {table} SET sync_id = ?1 WHERE id = ?2"),
        params![sync_id, local_id],
    )?;
    Ok(())
}

/// effective_timestamp: 有墓碑取 deleted_at，无墓碑取 updated_at。
/// 这确保删除操作的时间戳可以"赢过"普通的修改时间戳——即一个来自其他设备的墓碑能覆盖本地较旧的活跃实体。
fn remote_effective_timestamp<'a>(updated_at: &'a str, deleted_at: Option<&'a str>) -> &'a str {
    deleted_at.unwrap_or(updated_at)
}

/// PomodoroSession 没有 updated_at 字段，effective_timestamp 取:
/// deleted_at → ended_at → started_at（优先级从高到低）
fn remote_session_effective_timestamp(session: &PomodoroSession) -> &str {
    session
        .deleted_at
        .as_deref()
        .or(session.ended_at.as_deref())
        .unwrap_or(&session.started_at)
}

// 规则 4：四次相同逻辑 → 宏消除重复
macro_rules! normalized {
    ($name:ident, $ty:ty, $entity:expr) => {
        fn $name(item: &$ty) -> $ty {
            let mut normalized = item.clone();
            if normalized.sync_id.is_empty() {
                normalized.sync_id = legacy_sync_id($entity, normalized.id);
            }
            normalized
        }
    };
}
normalized!(normalized_task, Task, "task");
normalized!(normalized_course, Course, "course");
normalized!(normalized_exam, Exam, "exam");
normalized!(normalized_session, PomodoroSession, "pomodoro-session");

/// v1 兼容: 为无 sync_id 的远端实体生成稳定的导出标识。
/// 格式: legacy-{entity}-{sqlite_id}，同一实体在多次同步中保持同一值。
fn legacy_sync_id(entity: &str, id: i64) -> String {
    format!("legacy-{entity}-{id}")
}

fn build_entity_id_map(
    tx: &Transaction<'_>,
    remote_ids: Vec<(i64, String)>,
    table: &str,
) -> Result<HashMap<i64, i64>> {
    let mut map = HashMap::new();

    for (remote_id, sync_id) in remote_ids {
        if let Some(local_id) = tx
            .query_row(
                &format!("SELECT id FROM {table} WHERE sync_id = ?1"),
                params![sync_id],
                |row| row.get(0),
            )
            .optional()?
        {
            map.insert(remote_id, local_id);
        }
    }

    Ok(map)
}

fn map_remote_entity_id(map: &HashMap<i64, i64>, remote_id: Option<i64>) -> Option<i64> {
    remote_id.and_then(|id| map.get(&id).copied())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations;
    use rusqlite::Connection;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().expect("Failed to open in-memory DB");
        conn.pragma_update(None, "foreign_keys", "ON")
            .expect("Failed to enable foreign keys");
        migrations::run_migrations(&conn).expect("Migrations failed");
        conn
    }

    fn sample_sync_data() -> SyncData {
        SyncData {
            schema_version: 2,
            dataset_id: "dataset-test".to_string(),
            device_id: "device-test".to_string(),
            tasks: vec![],
            courses: vec![],
            exams: vec![],
            pomodoro_sessions: vec![],
            exported_at: "2024-06-01T10:00:00Z".to_string(),
        }
    }

    fn sample_task(id: i64, sync_id: &str, updated_at: &str) -> Task {
        Task {
            id,
            sync_id: sync_id.to_string(),
            title: "Test Task".to_string(),
            description: "Desc".to_string(),
            status: "todo".to_string(),
            priority: "high".to_string(),
            due_date: None,
            tags: "[]".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: updated_at.to_string(),
            deleted_at: None,
        }
    }

    fn sample_course(id: i64, sync_id: &str, updated_at: &str) -> Course {
        Course {
            id,
            sync_id: sync_id.to_string(),
            name: "Math 101".to_string(),
            day_of_week: 1,
            start_time: "08:00".to_string(),
            end_time: "09:30".to_string(),
            week_pattern: "1-16".to_string(),
            semester_start_date: "2026-02-24".to_string(),
            location: "Room 101".to_string(),
            teacher: "Prof. Smith".to_string(),
            color: "#3B82F6".to_string(),
            semester: "2024S1".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: updated_at.to_string(),
            deleted_at: None,
        }
    }

    fn sample_exam(id: i64, sync_id: &str, updated_at: &str) -> Exam {
        Exam {
            id,
            sync_id: sync_id.to_string(),
            course_name: "Math 101".to_string(),
            exam_datetime: "2024-06-15T10:00:00Z".to_string(),
            exam_end_datetime: "2024-06-15T12:00:00Z".to_string(),
            location: "Hall A".to_string(),
            notes: "".to_string(),
            course_id: Some(1),
            semester: "2024S1".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: updated_at.to_string(),
            deleted_at: None,
        }
    }

    #[test]
    fn test_export_empty_db() {
        let conn = setup_db();
        let data = export_all(&conn).expect("Failed to export");
        assert_eq!(data.schema_version, 2);
        assert!(!data.dataset_id.is_empty());
        assert!(!data.device_id.is_empty());
        assert!(data.tasks.is_empty());
        assert!(data.courses.is_empty());
        assert!(data.exams.is_empty());
        assert!(data.pomodoro_sessions.is_empty());
    }

    #[test]
    fn test_export_import_roundtrip_tasks() {
        let mut conn = setup_db();

        let mut data = sample_sync_data();
        data.tasks = vec![sample_task(1, "task-sync-1", "2024-01-01T00:00:00Z")];

        let stats = import_all(&mut conn, &data).expect("Failed to import");
        assert_eq!(stats.tasks_merged, 1);
        assert_eq!(stats.conflicts, 0);

        let exported = export_all(&conn).expect("Failed to re-export");
        assert_eq!(exported.tasks.len(), 1);
        assert_eq!(exported.tasks[0].title, "Test Task");
        assert_eq!(exported.tasks[0].priority, "high");
    }

    #[test]
    fn test_lww_newer_remote_wins() {
        let mut conn = setup_db();

        let mut old = sample_task(1, "task-sync-1", "2024-01-01T00:00:00Z");
        old.title = "Old Title".to_string();
        old.priority = "medium".to_string();
        let mut data1 = sample_sync_data();
        data1.tasks = vec![old];
        import_all(&mut conn, &data1).expect("Failed initial import");

        let mut newer = sample_task(99, "task-sync-1", "2024-02-01T00:00:00Z");
        newer.title = "Newer Title".to_string();
        newer.status = "done".to_string();
        let mut data2 = sample_sync_data();
        data2.tasks = vec![newer];
        let stats = import_all(&mut conn, &data2).expect("Failed merge import");
        assert_eq!(stats.tasks_merged, 1);

        let exported = export_all(&conn).expect("Failed to re-export");
        assert_eq!(exported.tasks.len(), 1);
        assert_eq!(exported.tasks[0].title, "Newer Title");
        assert_eq!(exported.tasks[0].status, "done");
        assert_eq!(exported.tasks[0].priority, "high");
    }

    #[test]
    fn test_lww_local_newer_keeps_local() {
        let mut conn = setup_db();

        let mut local_task = sample_task(1, "task-sync-1", "2024-03-01T00:00:00Z");
        local_task.title = "Local Title".to_string();
        local_task.status = "in_progress".to_string();
        let mut data1 = sample_sync_data();
        data1.tasks = vec![local_task];
        import_all(&mut conn, &data1).expect("Failed initial import");

        let mut older_remote = sample_task(99, "task-sync-1", "2024-02-01T00:00:00Z");
        older_remote.title = "Older Remote".to_string();
        older_remote.priority = "low".to_string();
        let mut data2 = sample_sync_data();
        data2.tasks = vec![older_remote];
        let stats = import_all(&mut conn, &data2).expect("Failed merge import");
        assert_eq!(stats.tasks_merged, 0);
        assert_eq!(stats.conflicts, 1);

        let exported = export_all(&conn).expect("Failed to re-export");
        assert_eq!(exported.tasks[0].title, "Local Title");
        assert_eq!(exported.tasks[0].status, "in_progress");
        assert_eq!(exported.tasks[0].priority, "high");
    }

    #[test]
    fn test_import_courses_and_exams() {
        let mut conn = setup_db();

        let course = sample_course(1, "course-sync-1", "2024-01-01T00:00:00Z");
        let exam = sample_exam(1, "exam-sync-1", "2024-01-01T00:00:00Z");
        let mut data = sample_sync_data();
        data.courses = vec![course];
        data.exams = vec![exam];

        let stats = import_all(&mut conn, &data).expect("Failed to import");
        assert_eq!(stats.courses_merged, 1);
        assert_eq!(stats.exams_merged, 1);

        let exported = export_all(&conn).expect("Failed to re-export");
        assert_eq!(exported.courses.len(), 1);
        assert_eq!(exported.courses[0].name, "Math 101");
        assert_eq!(exported.courses[0].week_pattern, "1-16");
        assert_eq!(exported.exams.len(), 1);
        assert_eq!(exported.exams[0].course_name, "Math 101");
        assert_eq!(exported.exams[0].exam_end_datetime, "2024-06-15T12:00:00Z");
        assert_eq!(exported.exams[0].course_id, Some(exported.courses[0].id));
    }

    #[test]
    fn test_course_edit_merges_by_sync_id_not_sqlite_id() {
        let mut conn = setup_db();

        let mut local = sample_course(1, "course-sync-1", "2024-01-01T00:00:00Z");
        local.start_time = "08:00".to_string();
        let mut initial = sample_sync_data();
        initial.courses = vec![local];
        import_all(&mut conn, &initial).expect("initial import");

        let mut remote = sample_course(42, "course-sync-1", "2024-02-01T00:00:00Z");
        remote.start_time = "10:00".to_string();
        let mut incoming = sample_sync_data();
        incoming.courses = vec![remote];
        let stats = import_all(&mut conn, &incoming).expect("merge import");

        assert_eq!(stats.courses_merged, 1);
        let exported = export_all(&conn).expect("export");
        assert_eq!(exported.courses.len(), 1);
        assert_eq!(exported.courses[0].sync_id, "course-sync-1");
        assert_eq!(exported.courses[0].start_time, "10:00");
    }

    #[test]
    fn test_newer_tombstone_hides_course_from_active_queries() {
        let mut conn = setup_db();

        let course = sample_course(1, "course-sync-1", "2024-01-01T00:00:00Z");
        let mut initial = sample_sync_data();
        initial.courses = vec![course];
        import_all(&mut conn, &initial).expect("initial import");

        let mut tombstone = sample_course(99, "course-sync-1", "2024-01-01T00:00:00Z");
        tombstone.deleted_at = Some("2024-03-01T00:00:00Z".to_string());
        let mut incoming = sample_sync_data();
        incoming.courses = vec![tombstone];

        let stats = import_all(&mut conn, &incoming).expect("tombstone import");

        assert_eq!(stats.courses_merged, 1);
        let active = crate::db::courses::get_all_courses(&conn, None).expect("active courses");
        assert!(active.is_empty());

        let exported = export_all(&conn).expect("export");
        assert_eq!(exported.courses.len(), 1);
        assert_eq!(
            exported.courses[0].deleted_at.as_deref(),
            Some("2024-03-01T00:00:00Z")
        );
    }

    #[test]
    fn test_v1_remote_without_sync_id_imports_once() {
        let mut conn = setup_db();

        let mut v1_course = sample_course(7, "", "2024-01-01T00:00:00Z");
        v1_course.name = "Legacy Course".to_string();
        let mut v1 = sample_sync_data();
        v1.schema_version = 1;
        v1.courses = vec![v1_course.clone()];

        import_all(&mut conn, &v1).expect("first v1 import");
        import_all(&mut conn, &v1).expect("second v1 import");

        let exported = export_all(&conn).expect("export");
        assert_eq!(exported.courses.len(), 1);
        assert_eq!(exported.courses[0].sync_id, "legacy-course-7");
        assert_eq!(exported.courses[0].name, "Legacy Course");
    }

    #[test]
    fn test_v1_remote_matches_existing_local_row_by_id_once() {
        let mut conn = setup_db();
        conn.execute(
            "INSERT INTO courses (
                id, sync_id, name, day_of_week, start_time, end_time, week_pattern,
                semester_start_date, location, teacher, color, semester, created_at, updated_at
             )
             VALUES (7, 'local-random-sync-id', 'Existing Course', 1, '08:00', '09:30', '1-16',
                     '2026-02-24', 'Room 101', 'Prof. Smith', '#3B82F6', '2024S1',
                     '2024-01-01T00:00:00Z', '2024-01-01T00:00:00Z')",
            [],
        )
        .expect("seed local course");

        let mut v1_course = sample_course(7, "", "2024-02-01T00:00:00Z");
        v1_course.name = "Legacy Remote Update".to_string();
        let mut v1 = sample_sync_data();
        v1.schema_version = 1;
        v1.courses = vec![v1_course];

        let stats = import_all(&mut conn, &v1).expect("v1 import");

        assert_eq!(stats.courses_merged, 1);
        let exported = export_all(&conn).expect("export");
        assert_eq!(exported.courses.len(), 1);
        assert_eq!(exported.courses[0].id, 7);
        assert_eq!(exported.courses[0].sync_id, "legacy-course-7");
        assert_eq!(exported.courses[0].name, "Legacy Remote Update");
    }
}
