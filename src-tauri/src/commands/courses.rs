use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

use rusqlite::Connection;
use serde::Deserialize;
use tauri::State;

use crate::db::models::{Course, CreateCourseRequest, UpdateCourseRequest};
use crate::importers::ImportTextResult;

#[derive(Debug, Deserialize)]
pub struct CreateCourseCmd {
    pub name: String,
    pub day_of_week: i64,
    pub start_time: String,
    pub end_time: String,
    #[serde(default)]
    pub week_pattern: Option<String>,
    #[serde(default)]
    pub semester_start_date: Option<String>,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub teacher: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub semester: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCourseCmd {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub day_of_week: Option<i64>,
    #[serde(default)]
    pub start_time: Option<String>,
    #[serde(default)]
    pub end_time: Option<String>,
    #[serde(default)]
    pub week_pattern: Option<String>,
    #[serde(default)]
    pub semester_start_date: Option<String>,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub teacher: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub semester: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CourseFilterParams {
    #[serde(default)]
    pub semester: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ImportCoursesCmd {
    pub text: String,
    pub semester: String,
    pub semester_start_date: String,
}

#[tauri::command]
pub fn get_all_courses(
    db: State<'_, Arc<Mutex<Connection>>>,
    filters: CourseFilterParams,
) -> Result<Vec<Course>, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    crate::db::courses::get_all_courses(&conn, filters.semester.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_course(
    db: State<'_, Arc<Mutex<Connection>>>,
    cmd: CreateCourseCmd,
) -> Result<i64, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let req = CreateCourseRequest {
        name: cmd.name,
        day_of_week: cmd.day_of_week,
        start_time: cmd.start_time,
        end_time: cmd.end_time,
        week_pattern: cmd.week_pattern.unwrap_or_default(),
        semester_start_date: cmd.semester_start_date.unwrap_or_default(),
        location: cmd.location.unwrap_or_default(),
        teacher: cmd.teacher.unwrap_or_default(),
        color: cmd.color.unwrap_or_else(|| String::from("#7C8CC0")),
        semester: cmd.semester.unwrap_or_default(),
    };
    crate::db::courses::create_course(&conn, &req).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_course(
    db: State<'_, Arc<Mutex<Connection>>>,
    id: i64,
    cmd: UpdateCourseCmd,
) -> Result<(), String> {
    let conn = db.lock().map_err(|e| e.to_string())?;

    let existing = crate::db::courses::get_course(&conn, id).map_err(|e| e.to_string())?;

    let req = UpdateCourseRequest {
        name: cmd.name.unwrap_or(existing.name),
        day_of_week: cmd.day_of_week.unwrap_or(existing.day_of_week),
        start_time: cmd.start_time.unwrap_or(existing.start_time),
        end_time: cmd.end_time.unwrap_or(existing.end_time),
        week_pattern: cmd.week_pattern.unwrap_or(existing.week_pattern),
        semester_start_date: cmd
            .semester_start_date
            .unwrap_or(existing.semester_start_date),
        location: cmd.location.unwrap_or(existing.location),
        teacher: cmd.teacher.unwrap_or(existing.teacher),
        color: cmd.color.unwrap_or(existing.color),
        semester: cmd.semester.unwrap_or(existing.semester),
    };
    crate::db::courses::update_course(&conn, id, &req).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_course(db: State<'_, Arc<Mutex<Connection>>>, id: i64) -> Result<(), String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    crate::db::courses::delete_course(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn import_courses_from_text(
    db: State<'_, Arc<Mutex<Connection>>>,
    cmd: ImportCoursesCmd,
) -> Result<ImportTextResult, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let courses = crate::importers::parse_course_import_text(
        &cmd.text,
        &cmd.semester,
        &cmd.semester_start_date,
    )?;

    import_new_courses(&conn, &courses, &cmd.semester)
}

fn import_new_courses(
    conn: &Connection,
    courses: &[CreateCourseRequest],
    semester: &str,
) -> Result<ImportTextResult, String> {
    let existing =
        crate::db::courses::get_all_courses(conn, Some(semester)).map_err(|e| e.to_string())?;
    let mut seen: HashSet<String> = existing.iter().map(course_import_key).collect();

    let mut imported = 0usize;
    let mut skipped = 0usize;

    for course in courses {
        let key = course_request_import_key(course);
        if !seen.insert(key) {
            skipped += 1;
            continue;
        }

        crate::db::courses::create_course(conn, course).map_err(|e| e.to_string())?;
        imported += 1;
    }

    Ok(ImportTextResult::from_counts(
        courses.len(),
        imported,
        skipped,
    ))
}

fn course_import_key(course: &Course) -> String {
    [
        course.semester.as_str(),
        course.name.as_str(),
        &course.day_of_week.to_string(),
        course.start_time.as_str(),
        course.end_time.as_str(),
        course.week_pattern.as_str(),
        course.semester_start_date.as_str(),
        course.location.as_str(),
        course.teacher.as_str(),
    ]
    .join("\t")
}

fn course_request_import_key(course: &CreateCourseRequest) -> String {
    [
        course.semester.as_str(),
        course.name.as_str(),
        &course.day_of_week.to_string(),
        course.start_time.as_str(),
        course.end_time.as_str(),
        course.week_pattern.as_str(),
        course.semester_start_date.as_str(),
        course.location.as_str(),
        course.teacher.as_str(),
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

    fn sample_course() -> CreateCourseRequest {
        CreateCourseRequest {
            name: "自动控制原理".to_string(),
            day_of_week: 3,
            start_time: "10:00".to_string(),
            end_time: "11:40".to_string(),
            week_pattern: "1-17周全周".to_string(),
            semester_start_date: "2026-02-24".to_string(),
            location: "秦岭堂A114".to_string(),
            teacher: "李红信".to_string(),
            color: "#3B82F6".to_string(),
            semester: "2026S1".to_string(),
        }
    }

    #[test]
    fn test_import_new_courses_skips_duplicates() {
        let conn = setup_db();
        let course = sample_course();

        let first = import_new_courses(&conn, std::slice::from_ref(&course), "2026S1")
            .expect("first import");
        assert_eq!(first.parsed, 1);
        assert_eq!(first.imported, 1);
        assert_eq!(first.skipped, 0);

        let second = import_new_courses(&conn, &[course], "2026S1").expect("second import");
        assert_eq!(second.parsed, 1);
        assert_eq!(second.imported, 0);
        assert_eq!(second.skipped, 1);

        let courses =
            crate::db::courses::get_all_courses(&conn, Some("2026S1")).expect("get courses");
        assert_eq!(courses.len(), 1);
    }
}
