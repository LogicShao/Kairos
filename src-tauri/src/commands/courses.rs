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
pub fn reset_all_semester_start_dates(
    db: State<'_, Arc<Mutex<Connection>>>,
    date: String,
) -> Result<usize, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    reset_semester_start_dates(&conn, &date)
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

pub(crate) fn import_new_courses(
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

fn reset_semester_start_dates(conn: &Connection, date: &str) -> Result<usize, String> {
    let course_count = crate::db::courses::update_all_semester_start_dates(conn, date)
        .map_err(|e| e.to_string())?;
    crate::db::semester::update_all_semester_context_start_dates(conn, date)
        .map_err(|e| e.to_string())?;
    Ok(course_count)
}

/// 课程导入去重键的公共字段拼接。参数多但均为课程判定键的必要字段。
#[allow(clippy::too_many_arguments)]
fn course_key_parts(
    semester: &str,
    name: &str,
    day_of_week: i64,
    start_time: &str,
    end_time: &str,
    week_pattern: &str,
    semester_start_date: &str,
    location: &str,
    teacher: &str,
) -> String {
    [
        semester,
        name,
        &day_of_week.to_string(),
        start_time,
        end_time,
        week_pattern,
        semester_start_date,
        location,
        teacher,
    ]
    .join("\t")
}

fn course_import_key(course: &Course) -> String {
    course_key_parts(
        &course.semester,
        &course.name,
        course.day_of_week,
        &course.start_time,
        &course.end_time,
        &course.week_pattern,
        &course.semester_start_date,
        &course.location,
        &course.teacher,
    )
}

fn course_request_import_key(course: &CreateCourseRequest) -> String {
    course_key_parts(
        &course.semester,
        &course.name,
        course.day_of_week,
        &course.start_time,
        &course.end_time,
        &course.week_pattern,
        &course.semester_start_date,
        &course.location,
        &course.teacher,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::UpsertSemesterContextRequest;
    use crate::db::semester::LZU_SEMESTER_CONTEXT_SOURCE;
    use crate::db::setup_db;

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

    #[test]
    fn test_reset_semester_start_dates_updates_contexts() {
        let conn = setup_db();
        let course = sample_course();
        crate::db::courses::create_course(&conn, &course).expect("create course");
        crate::db::semester::upsert_semester_context(
            &conn,
            &UpsertSemesterContextRequest {
                source: LZU_SEMESTER_CONTEXT_SOURCE.to_string(),
                academic_year: Some("2026".to_string()),
                term: Some("1".to_string()),
                term_label: "2026S1".to_string(),
                start_date: "2026-02-24".to_string(),
                current_week: Some(3),
                total_weeks: None,
            },
        )
        .expect("upsert context");

        let updated =
            reset_semester_start_dates(&conn, "2026-03-02").expect("reset semester dates");

        assert_eq!(updated, 1);
        let courses =
            crate::db::courses::get_all_courses(&conn, Some("2026S1")).expect("get courses");
        assert_eq!(courses[0].semester_start_date, "2026-03-02");

        let context = crate::db::semester::find_semester_context(
            &conn,
            LZU_SEMESTER_CONTEXT_SOURCE,
            "2026S1",
        )
        .expect("find context")
        .expect("context exists");
        assert_eq!(context.start_date, "2026-03-02");
    }
}
