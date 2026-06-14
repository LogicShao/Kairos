use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};

use crate::db::models::{Course, Exam, PomodoroSession, Task};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncData {
    pub tasks: Vec<Task>,
    pub courses: Vec<Course>,
    pub exams: Vec<Exam>,
    pub pomodoro_sessions: Vec<PomodoroSession>,
    pub exported_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncStats {
    pub tasks_merged: usize,
    pub courses_merged: usize,
    pub exams_merged: usize,
    pub sessions_merged: usize,
    pub conflicts: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncResult {
    pub uploaded: bool,
    pub downloaded: bool,
    pub stats: SyncStats,
}

pub fn export_all(conn: &Connection) -> Result<SyncData> {
    let tasks = export_tasks(conn)?;
    let courses = export_courses(conn)?;
    let exams = export_exams(conn)?;
    let pomodoro_sessions = export_pomodoro_sessions(conn)?;
    let exported_at = crate::db::chrono_now();

    Ok(SyncData {
        tasks,
        courses,
        exams,
        pomodoro_sessions,
        exported_at,
    })
}

pub fn import_all(conn: &Connection, data: &SyncData) -> Result<SyncStats> {
    let tasks_merged = merge_tasks(conn, &data.tasks)?;
    let courses_merged = merge_courses(conn, &data.courses)?;
    let exams_merged = merge_exams(conn, &data.exams)?;
    let sessions_merged = merge_pomodoro_sessions(conn, &data.pomodoro_sessions)?;

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

fn export_tasks(conn: &Connection) -> Result<Vec<Task>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, description, status, priority, due_date, tags, created_at, updated_at
         FROM tasks ORDER BY id",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Task {
            id: row.get(0)?,
            title: row.get(1)?,
            description: row.get(2)?,
            status: row.get(3)?,
            priority: row.get(4)?,
            due_date: row.get(5)?,
            tags: row.get(6)?,
            created_at: row.get(7)?,
            updated_at: row.get(8)?,
        })
    })?;
    rows.collect()
}

fn export_courses(conn: &Connection) -> Result<Vec<Course>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, day_of_week, start_time, end_time, location, teacher, color, semester, created_at, updated_at
         FROM courses ORDER BY id",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Course {
            id: row.get(0)?,
            name: row.get(1)?,
            day_of_week: row.get(2)?,
            start_time: row.get(3)?,
            end_time: row.get(4)?,
            location: row.get(5)?,
            teacher: row.get(6)?,
            color: row.get(7)?,
            semester: row.get(8)?,
            created_at: row.get(9)?,
            updated_at: row.get(10)?,
        })
    })?;
    rows.collect()
}

fn export_exams(conn: &Connection) -> Result<Vec<Exam>> {
    let mut stmt = conn.prepare(
        "SELECT id, course_name, exam_datetime, location, notes, course_id, created_at, updated_at
         FROM exams ORDER BY id",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Exam {
            id: row.get(0)?,
            course_name: row.get(1)?,
            exam_datetime: row.get(2)?,
            location: row.get(3)?,
            notes: row.get(4)?,
            course_id: row.get(5)?,
            created_at: row.get(6)?,
            updated_at: row.get(7)?,
        })
    })?;
    rows.collect()
}

fn export_pomodoro_sessions(conn: &Connection) -> Result<Vec<PomodoroSession>> {
    let mut stmt = conn.prepare(
        "SELECT id, started_at, ended_at, session_type, task_id
         FROM pomodoro_sessions ORDER BY id",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(PomodoroSession {
            id: row.get(0)?,
            started_at: row.get(1)?,
            ended_at: row.get(2)?,
            session_type: row.get(3)?,
            task_id: row.get(4)?,
        })
    })?;
    rows.collect()
}

fn merge_tasks(conn: &Connection, remote: &[Task]) -> Result<usize> {
    let mut merged = 0usize;

    let mut stmt = conn.prepare("SELECT updated_at FROM tasks WHERE id = ?1")?;

    for task in remote {
        let local_updated: Option<String> = stmt
            .query_row(params![task.id], |row| row.get(0))
            .ok();

        match local_updated {
            None => {
                conn.execute(
                    "INSERT INTO tasks (id, title, description, status, priority, due_date, tags, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        task.id,
                        task.title,
                        task.description,
                        task.status,
                        task.priority,
                        task.due_date,
                        task.tags,
                        task.created_at,
                        task.updated_at,
                    ],
                )?;
                merged += 1;
            }
            Some(local_ts) => {
                if task.updated_at > local_ts {
                    conn.execute(
                        "UPDATE tasks
                         SET title = ?1, description = ?2, status = ?3, priority = ?4,
                             due_date = ?5, tags = ?6, created_at = ?7, updated_at = ?8
                         WHERE id = ?9",
                        params![
                            task.title,
                            task.description,
                            task.status,
                            task.priority,
                            task.due_date,
                            task.tags,
                            task.created_at,
                            task.updated_at,
                            task.id,
                        ],
                    )?;
                    merged += 1;
                }
            }
        }
    }

    Ok(merged)
}

fn merge_courses(conn: &Connection, remote: &[Course]) -> Result<usize> {
    let mut merged = 0usize;

    let mut stmt = conn.prepare("SELECT updated_at FROM courses WHERE id = ?1")?;

    for course in remote {
        let local_updated: Option<String> = stmt
            .query_row(params![course.id], |row| row.get(0))
            .ok();

        match local_updated {
            None => {
                conn.execute(
                    "INSERT INTO courses (id, name, day_of_week, start_time, end_time, location, teacher, color, semester, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                    params![
                        course.id,
                        course.name,
                        course.day_of_week,
                        course.start_time,
                        course.end_time,
                        course.location,
                        course.teacher,
                        course.color,
                        course.semester,
                        course.created_at,
                        course.updated_at,
                    ],
                )?;
                merged += 1;
            }
            Some(local_ts) => {
                if course.updated_at > local_ts {
                    conn.execute(
                        "UPDATE courses
                         SET name = ?1, day_of_week = ?2, start_time = ?3, end_time = ?4,
                             location = ?5, teacher = ?6, color = ?7, semester = ?8,
                             created_at = ?9, updated_at = ?10
                         WHERE id = ?11",
                        params![
                            course.name,
                            course.day_of_week,
                            course.start_time,
                            course.end_time,
                            course.location,
                            course.teacher,
                            course.color,
                            course.semester,
                            course.created_at,
                            course.updated_at,
                            course.id,
                        ],
                    )?;
                    merged += 1;
                }
            }
        }
    }

    Ok(merged)
}

fn merge_exams(conn: &Connection, remote: &[Exam]) -> Result<usize> {
    let mut merged = 0usize;

    let mut stmt = conn.prepare("SELECT updated_at FROM exams WHERE id = ?1")?;

    for exam in remote {
        let local_updated: Option<String> = stmt
            .query_row(params![exam.id], |row| row.get(0))
            .ok();

        match local_updated {
            None => {
                conn.execute(
                    "INSERT INTO exams (id, course_name, exam_datetime, location, notes, course_id, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        exam.id,
                        exam.course_name,
                        exam.exam_datetime,
                        exam.location,
                        exam.notes,
                        exam.course_id,
                        exam.created_at,
                        exam.updated_at,
                    ],
                )?;
                merged += 1;
            }
            Some(local_ts) => {
                if exam.updated_at > local_ts {
                    conn.execute(
                        "UPDATE exams
                         SET course_name = ?1, exam_datetime = ?2, location = ?3,
                             notes = ?4, course_id = ?5, created_at = ?6, updated_at = ?7
                         WHERE id = ?8",
                        params![
                            exam.course_name,
                            exam.exam_datetime,
                            exam.location,
                            exam.notes,
                            exam.course_id,
                            exam.created_at,
                            exam.updated_at,
                            exam.id,
                        ],
                    )?;
                    merged += 1;
                }
            }
        }
    }

    Ok(merged)
}

fn merge_pomodoro_sessions(conn: &Connection, remote: &[PomodoroSession]) -> Result<usize> {
    let mut merged = 0usize;

    for session in remote {
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM pomodoro_sessions WHERE id = ?1",
                params![session.id],
                |row| row.get::<_, i64>(0),
            )
            .map(|count| count > 0)?;

        if !exists {
            conn.execute(
                "INSERT INTO pomodoro_sessions (id, started_at, ended_at, session_type, task_id)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    session.id,
                    session.started_at,
                    session.ended_at,
                    session.session_type,
                    session.task_id,
                ],
            )?;
            merged += 1;
        }
    }

    Ok(merged)
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

    #[test]
    fn test_export_empty_db() {
        let conn = setup_db();
        let data = export_all(&conn).expect("Failed to export");
        assert!(data.tasks.is_empty());
        assert!(data.courses.is_empty());
        assert!(data.exams.is_empty());
        assert!(data.pomodoro_sessions.is_empty());
    }

    #[test]
    fn test_export_import_roundtrip_tasks() {
        let conn = setup_db();

        let task = Task {
            id: 1,
            title: "Test Task".to_string(),
            description: "Desc".to_string(),
            status: "todo".to_string(),
            priority: "high".to_string(),
            due_date: None,
            tags: "[]".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let data = SyncData {
            tasks: vec![task.clone()],
            courses: vec![],
            exams: vec![],
            pomodoro_sessions: vec![],
            exported_at: "2024-06-01T10:00:00Z".to_string(),
        };

        let stats = import_all(&conn, &data).expect("Failed to import");
        assert_eq!(stats.tasks_merged, 1);
        assert_eq!(stats.conflicts, 0);

        let exported = export_all(&conn).expect("Failed to re-export");
        assert_eq!(exported.tasks.len(), 1);
        assert_eq!(exported.tasks[0].title, "Test Task");
        assert_eq!(exported.tasks[0].priority, "high");
    }

    #[test]
    fn test_lww_newer_remote_wins() {
        let conn = setup_db();

        let old = Task {
            id: 1,
            title: "Old Title".to_string(),
            description: "".to_string(),
            status: "todo".to_string(),
            priority: "medium".to_string(),
            due_date: None,
            tags: "[]".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let data1 = SyncData {
            tasks: vec![old],
            courses: vec![],
            exams: vec![],
            pomodoro_sessions: vec![],
            exported_at: "2024-06-01T10:00:00Z".to_string(),
        };
        import_all(&conn, &data1).expect("Failed initial import");

        let newer = Task {
            id: 1,
            title: "Newer Title".to_string(),
            description: "".to_string(),
            status: "done".to_string(),
            priority: "high".to_string(),
            due_date: None,
            tags: "[]".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-02-01T00:00:00Z".to_string(),
        };

        let data2 = SyncData {
            tasks: vec![newer],
            courses: vec![],
            exams: vec![],
            pomodoro_sessions: vec![],
            exported_at: "2024-07-01T10:00:00Z".to_string(),
        };
        let stats = import_all(&conn, &data2).expect("Failed merge import");
        assert_eq!(stats.tasks_merged, 1);

        let exported = export_all(&conn).expect("Failed to re-export");
        assert_eq!(exported.tasks[0].title, "Newer Title");
        assert_eq!(exported.tasks[0].status, "done");
        assert_eq!(exported.tasks[0].priority, "high");
    }

    #[test]
    fn test_lww_local_newer_keeps_local() {
        let conn = setup_db();

        let local_task = Task {
            id: 1,
            title: "Local Title".to_string(),
            description: "".to_string(),
            status: "in_progress".to_string(),
            priority: "high".to_string(),
            due_date: None,
            tags: "[]".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-03-01T00:00:00Z".to_string(),
        };

        let data1 = SyncData {
            tasks: vec![local_task],
            courses: vec![],
            exams: vec![],
            pomodoro_sessions: vec![],
            exported_at: "2024-06-01T10:00:00Z".to_string(),
        };
        import_all(&conn, &data1).expect("Failed initial import");

        let older_remote = Task {
            id: 1,
            title: "Older Remote".to_string(),
            description: "".to_string(),
            status: "todo".to_string(),
            priority: "low".to_string(),
            due_date: None,
            tags: "[]".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-02-01T00:00:00Z".to_string(),
        };

        let data2 = SyncData {
            tasks: vec![older_remote],
            courses: vec![],
            exams: vec![],
            pomodoro_sessions: vec![],
            exported_at: "2024-07-01T10:00:00Z".to_string(),
        };
        let stats = import_all(&conn, &data2).expect("Failed merge import");
        assert_eq!(stats.tasks_merged, 0);
        assert_eq!(stats.conflicts, 1);

        let exported = export_all(&conn).expect("Failed to re-export");
        assert_eq!(exported.tasks[0].title, "Local Title");
        assert_eq!(exported.tasks[0].status, "in_progress");
        assert_eq!(exported.tasks[0].priority, "high");
    }

    #[test]
    fn test_import_courses_and_exams() {
        let conn = setup_db();

        let course = Course {
            id: 1,
            name: "Math 101".to_string(),
            day_of_week: 1,
            start_time: "08:00".to_string(),
            end_time: "09:30".to_string(),
            location: "Room 101".to_string(),
            teacher: "Prof. Smith".to_string(),
            color: "#3B82F6".to_string(),
            semester: "2024S1".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let exam = Exam {
            id: 1,
            course_name: "Math 101".to_string(),
            exam_datetime: "2024-06-15T10:00:00Z".to_string(),
            location: "Hall A".to_string(),
            notes: "".to_string(),
            course_id: Some(1),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let data = SyncData {
            tasks: vec![],
            courses: vec![course],
            exams: vec![exam],
            pomodoro_sessions: vec![],
            exported_at: "2024-06-01T10:00:00Z".to_string(),
        };

        let stats = import_all(&conn, &data).expect("Failed to import");
        assert_eq!(stats.courses_merged, 1);
        assert_eq!(stats.exams_merged, 1);

        let exported = export_all(&conn).expect("Failed to re-export");
        assert_eq!(exported.courses.len(), 1);
        assert_eq!(exported.courses[0].name, "Math 101");
        assert_eq!(exported.exams.len(), 1);
        assert_eq!(exported.exams[0].course_name, "Math 101");
    }
}
