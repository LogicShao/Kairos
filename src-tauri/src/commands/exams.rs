use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

use rusqlite::Connection;
use serde::Deserialize;
use tauri::State;

use crate::db::models::{CreateExamRequest, Exam, UpdateExamRequest};
use crate::importers::ImportTextResult;

#[derive(Debug, Deserialize)]
pub struct CreateExamCmd {
    pub course_name: String,
    pub exam_datetime: String,
    #[serde(default)]
    pub exam_end_datetime: Option<String>,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub course_id: Option<i64>,
    #[serde(default)]
    pub semester: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateExamCmd {
    #[serde(default)]
    pub course_name: Option<String>,
    #[serde(default)]
    pub exam_datetime: Option<String>,
    #[serde(default)]
    pub exam_end_datetime: Option<String>,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub course_id: Option<i64>,
    #[serde(default)]
    pub semester: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ImportExamsCmd {
    pub text: String,
    pub semester: String,
}

#[tauri::command]
pub fn get_all_exams(db: State<'_, Arc<Mutex<Connection>>>) -> Result<Vec<Exam>, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    crate::db::exams::get_all_exams(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_exam(
    db: State<'_, Arc<Mutex<Connection>>>,
    app_handle: tauri::AppHandle,
    cmd: CreateExamCmd,
) -> Result<i64, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let req = CreateExamRequest {
        course_name: cmd.course_name,
        exam_datetime: cmd.exam_datetime,
        exam_end_datetime: cmd.exam_end_datetime.unwrap_or_default(),
        location: cmd.location.unwrap_or_default(),
        notes: cmd.notes.unwrap_or_default(),
        course_id: cmd.course_id,
        semester: cmd.semester.unwrap_or_default(),
    };
    let id = crate::db::exams::create_exam(&conn, &req).map_err(|e| e.to_string())?;

    // Schedule notifications for the newly created exam
    if let Ok(exam) = crate::db::exams::get_exam(&conn, id) {
        if let Err(e) =
            crate::notifications::exam_scheduler::schedule_exam_for_one(&conn, &app_handle, &exam)
        {
            log::error!("failed to schedule notifications for new exam {id}: {e}");
        }
    }

    Ok(id)
}

#[tauri::command]
pub fn update_exam(
    db: State<'_, Arc<Mutex<Connection>>>,
    app_handle: tauri::AppHandle,
    id: i64,
    cmd: UpdateExamCmd,
) -> Result<(), String> {
    let conn = db.lock().map_err(|e| e.to_string())?;

    let existing = crate::db::exams::get_exam(&conn, id).map_err(|e| e.to_string())?;

    // Cancel notifications for the old exam data
    if let Err(e) = crate::notifications::exam_scheduler::cancel_exam_notifications(
        &conn,
        &app_handle,
        &existing,
    ) {
        log::error!("failed to cancel notifications for exam {id}: {e}");
    }

    let req = UpdateExamRequest {
        course_name: cmd.course_name.unwrap_or(existing.course_name),
        exam_datetime: cmd.exam_datetime.unwrap_or(existing.exam_datetime),
        exam_end_datetime: cmd.exam_end_datetime.unwrap_or(existing.exam_end_datetime),
        location: cmd.location.unwrap_or(existing.location),
        notes: cmd.notes.unwrap_or(existing.notes),
        course_id: match cmd.course_id {
            Some(val) => Some(val),
            None => existing.course_id,
        },
        semester: cmd.semester.unwrap_or(existing.semester),
    };
    crate::db::exams::update_exam(&conn, id, &req).map_err(|e| e.to_string())?;

    // Schedule notifications for the updated exam
    if let Ok(updated) = crate::db::exams::get_exam(&conn, id) {
        if let Err(e) = crate::notifications::exam_scheduler::schedule_exam_for_one(
            &conn,
            &app_handle,
            &updated,
        ) {
            log::error!("failed to schedule notifications for updated exam {id}: {e}");
        }
    }

    Ok(())
}

#[tauri::command]
pub fn delete_exam(
    db: State<'_, Arc<Mutex<Connection>>>,
    app_handle: tauri::AppHandle,
    id: i64,
) -> Result<(), String> {
    let conn = db.lock().map_err(|e| e.to_string())?;

    // Cancel notifications before deleting
    if let Ok(exam) = crate::db::exams::get_exam(&conn, id) {
        if let Err(e) = crate::notifications::exam_scheduler::cancel_exam_notifications(
            &conn,
            &app_handle,
            &exam,
        ) {
            log::error!("failed to cancel notifications for deleted exam {id}: {e}");
        }
    }

    crate::db::exams::delete_exam(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn import_exams_from_text(
    db: State<'_, Arc<Mutex<Connection>>>,
    cmd: ImportExamsCmd,
) -> Result<ImportTextResult, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let exams = crate::importers::parse_exam_import_text(&cmd.text, &cmd.semester)?;

    import_new_exams(&conn, &exams)
}

fn import_new_exams(
    conn: &Connection,
    exams: &[CreateExamRequest],
) -> Result<ImportTextResult, String> {
    let existing = crate::db::exams::get_all_exams(conn).map_err(|e| e.to_string())?;
    let mut seen: HashSet<String> = existing.iter().map(exam_import_key).collect();

    let mut imported = 0usize;
    let mut skipped = 0usize;

    for exam in exams {
        let key = exam_request_import_key(exam);
        if !seen.insert(key) {
            skipped += 1;
            continue;
        }

        crate::db::exams::create_exam(conn, exam).map_err(|e| e.to_string())?;
        imported += 1;
    }

    Ok(ImportTextResult::from_counts(
        exams.len(),
        imported,
        skipped,
    ))
}

fn exam_import_key(exam: &Exam) -> String {
    [
        exam.semester.as_str(),
        exam.course_name.as_str(),
        exam.exam_datetime.as_str(),
        exam.exam_end_datetime.as_str(),
        exam.location.as_str(),
    ]
    .join("\t")
}

fn exam_request_import_key(exam: &CreateExamRequest) -> String {
    [
        exam.semester.as_str(),
        exam.course_name.as_str(),
        exam.exam_datetime.as_str(),
        exam.exam_end_datetime.as_str(),
        exam.location.as_str(),
    ]
    .join("\t")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().expect("Failed to open in-memory DB");
        conn.pragma_update(None, "foreign_keys", "ON")
            .expect("Failed to enable foreign keys");
        migrations::run_migrations(&conn).expect("Migrations failed");
        conn
    }

    fn sample_exam() -> CreateExamRequest {
        CreateExamRequest {
            course_name: "自动控制原理".to_string(),
            exam_datetime: "2026-07-06T08:00:00Z".to_string(),
            exam_end_datetime: "2026-07-06T10:00:00Z".to_string(),
            location: "天山堂A409".to_string(),
            notes: "正常考试".to_string(),
            course_id: None,
            semester: "2026S1".to_string(),
        }
    }

    #[test]
    fn test_import_new_exams_skips_duplicates() {
        let conn = setup_db();
        let exam = sample_exam();

        let first = import_new_exams(&conn, std::slice::from_ref(&exam)).expect("first import");
        assert_eq!(first.parsed, 1);
        assert_eq!(first.imported, 1);
        assert_eq!(first.skipped, 0);

        let second = import_new_exams(&conn, &[exam]).expect("second import");
        assert_eq!(second.parsed, 1);
        assert_eq!(second.imported, 0);
        assert_eq!(second.skipped, 1);

        let exams = crate::db::exams::get_all_exams(&conn).expect("get exams");
        assert_eq!(exams.len(), 1);
    }
}
