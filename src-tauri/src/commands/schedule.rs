use std::sync::{Arc, Mutex};

use rusqlite::Connection;
use serde::Deserialize;
use tauri::State;

use crate::db::semester::LZU_SEMESTER_CONTEXT_SOURCE;

#[derive(Debug, Deserialize)]
pub struct WeekScheduleCmd {
    pub semester: String,
    pub week_index: i64,
    #[serde(default)]
    pub semester_start_date: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CalendarWeekCmd {
    pub semester: String,
    pub week_index: i64,
    #[serde(default)]
    pub semester_start_date: Option<String>,
    #[serde(default)]
    pub week_start_date: Option<String>,
}

fn effective_semester_start_date(
    conn: &Connection,
    semester: &str,
    requested: Option<&String>,
) -> Result<Option<String>, String> {
    if let Some(value) = requested.map(String::as_str).map(str::trim) {
        if !value.is_empty() {
            return Ok(Some(value.to_string()));
        }
    }

    crate::db::semester::find_semester_context(conn, LZU_SEMESTER_CONTEXT_SOURCE, semester)
        .map_err(|e| e.to_string())
        .map(|context| context.map(|value| value.start_date))
}

#[tauri::command]
pub fn get_week_schedule(
    db: State<'_, Arc<Mutex<Connection>>>,
    cmd: WeekScheduleCmd,
) -> Result<crate::schedule::WeekScheduleResponse, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let courses = crate::db::courses::get_all_courses(&conn, Some(&cmd.semester))
        .map_err(|e| e.to_string())?;
    let exams = crate::db::exams::get_all_exams(&conn).map_err(|e| e.to_string())?;
    let semester_start_date =
        effective_semester_start_date(&conn, &cmd.semester, cmd.semester_start_date.as_ref())?;
    let phase_status = crate::term_phase::get_phase_status_for_term_week(
        &conn,
        LZU_SEMESTER_CONTEXT_SOURCE,
        &cmd.semester,
        cmd.week_index,
    )?;

    crate::schedule::build_week_schedule(
        &courses,
        &exams,
        &cmd.semester,
        cmd.week_index,
        semester_start_date.as_deref(),
        Some(&phase_status),
    )
}

#[tauri::command]
pub fn get_calendar_week(
    db: State<'_, Arc<Mutex<Connection>>>,
    cmd: CalendarWeekCmd,
) -> Result<crate::schedule::CalendarWeekResponse, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let courses = crate::db::courses::get_all_courses(&conn, Some(&cmd.semester))
        .map_err(|e| e.to_string())?;
    let exams = crate::db::exams::get_all_exams(&conn).map_err(|e| e.to_string())?;
    let tasks = crate::db::tasks::get_all_tasks(&conn, None, None, "due_date", "ASC")
        .map_err(|e| e.to_string())?;
    let semester_start_date =
        effective_semester_start_date(&conn, &cmd.semester, cmd.semester_start_date.as_ref())?;
    let phase_status = crate::term_phase::get_phase_status_for_term_week(
        &conn,
        LZU_SEMESTER_CONTEXT_SOURCE,
        &cmd.semester,
        cmd.week_index,
    )?;

    crate::schedule::build_calendar_week(
        &courses,
        &exams,
        &tasks,
        &cmd.semester,
        cmd.week_index,
        semester_start_date.as_deref(),
        cmd.week_start_date.as_deref(),
        Some(&phase_status),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::UpsertSemesterContextRequest;
    use crate::db::setup_db;

    fn sample_context() -> UpsertSemesterContextRequest {
        UpsertSemesterContextRequest {
            source: LZU_SEMESTER_CONTEXT_SOURCE.to_string(),
            academic_year: Some("2026".to_string()),
            term: Some("1".to_string()),
            term_label: "2026S1".to_string(),
            start_date: "2026-02-24".to_string(),
            current_week: Some(3),
            total_weeks: None,
        }
    }

    #[test]
    fn test_effective_semester_start_date_prefers_explicit_request() {
        let conn = setup_db();
        crate::db::semester::upsert_semester_context(&conn, &sample_context())
            .expect("upsert context");
        let requested = "2026-03-02".to_string();

        let start_date =
            effective_semester_start_date(&conn, "2026S1", Some(&requested)).expect("resolve date");

        assert_eq!(start_date.as_deref(), Some("2026-03-02"));
    }

    #[test]
    fn test_effective_semester_start_date_uses_context_when_request_missing() {
        let conn = setup_db();
        crate::db::semester::upsert_semester_context(&conn, &sample_context())
            .expect("upsert context");

        let start_date =
            effective_semester_start_date(&conn, "2026S1", None).expect("resolve date");

        assert_eq!(start_date.as_deref(), Some("2026-02-24"));
    }

    #[test]
    fn test_effective_semester_start_date_returns_none_without_context() {
        let conn = setup_db();

        let start_date =
            effective_semester_start_date(&conn, "2026S1", None).expect("resolve date");

        assert_eq!(start_date, None);
    }
}
