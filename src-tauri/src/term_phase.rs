use chrono::{Datelike, Duration, FixedOffset, NaiveDate};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::db::models::{SemesterContext, TermPhase};
use crate::db::semester::LZU_SEMESTER_CONTEXT_SOURCE;

pub const PHASE_UNKNOWN: &str = "unknown";
pub const PHASE_TEACHING: &str = "teaching";
pub const PHASE_EXAM: &str = "exam";
pub const PHASE_BREAK: &str = "break";

const FALLBACK_TOTAL_WEEKS: i64 = 20;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CurrentPhaseStatus {
    pub source: String,
    pub term_label: Option<String>,
    pub phase_type: String,
    pub current_week: Option<i64>,
    pub start_week: Option<i64>,
    pub end_week: Option<i64>,
    pub courses_visible: bool,
    pub exam_notifications_enabled: bool,
    pub pomodoro_profile: String,
    pub inferred: bool,
}

pub fn get_current_phase_status(
    conn: &Connection,
    source: &str,
) -> Result<CurrentPhaseStatus, String> {
    let china_offset = FixedOffset::east_opt(8 * 3600).expect("china utc offset");
    let today = chrono::Utc::now().with_timezone(&china_offset).date_naive();
    get_phase_status_for_date(conn, source, today)
}

pub fn get_phase_status_for_date(
    conn: &Connection,
    source: &str,
    today: NaiveDate,
) -> Result<CurrentPhaseStatus, String> {
    let context = crate::db::semester::get_latest_semester_context_by_source(conn, source)
        .map_err(|e| e.to_string())?;
    let Some(context) = context else {
        return Ok(unknown_status(source));
    };

    let current_week = current_week_from_context(&context, today)?;
    if let Some(phase) =
        crate::db::term_phases::get_term_phase_by_week(conn, &context.term_label, current_week)
            .map_err(|e| e.to_string())?
    {
        return Ok(downgrade_teaching_if_no_courses(
            conn,
            source,
            current_week,
            phase,
        ));
    }

    Ok(downgrade_inferred_teaching_if_no_courses(
        conn,
        inferred_status(source, &context, current_week),
        &context.term_label,
    ))
}

pub fn get_phase_status_for_term_week(
    conn: &Connection,
    source: &str,
    term_label: &str,
    week_index: i64,
) -> Result<CurrentPhaseStatus, String> {
    if week_index < 1 {
        return Err("week_index 必须大于等于 1".to_string());
    }

    if let Some(phase) =
        crate::db::term_phases::get_term_phase_by_week(conn, term_label, week_index)
            .map_err(|e| e.to_string())?
    {
        return Ok(downgrade_teaching_if_no_courses(
            conn,
            source,
            week_index,
            phase,
        ));
    }

    let context = crate::db::semester::find_semester_context(conn, source, term_label)
        .map_err(|e| e.to_string())?;
    let total_weeks = context
        .as_ref()
        .and_then(|value| value.total_weeks)
        .unwrap_or(FALLBACK_TOTAL_WEEKS);

    Ok(downgrade_inferred_teaching_if_no_courses(
        conn,
        inferred_status_from_parts(
            source,
            Some(term_label.to_string()),
            week_index,
            total_weeks,
        ),
        term_label,
    ))
}

pub fn current_week_from_context(
    context: &SemesterContext,
    today: NaiveDate,
) -> Result<i64, String> {
    let anchor = NaiveDate::parse_from_str(&context.start_date, "%Y-%m-%d")
        .map_err(|_| format!("无法解析学期开始日期: {}", context.start_date))?;
    let week_start = today - Duration::days(today.weekday().num_days_from_monday() as i64);
    let anchor_week_start = anchor - Duration::days(anchor.weekday().num_days_from_monday() as i64);
    let diff_days = week_start
        .signed_duration_since(anchor_week_start)
        .num_days();
    Ok(diff_days.div_euclid(7) + 1).map(|week| week.max(1))
}

pub fn latest_lzu_phase_status(conn: &Connection) -> Result<CurrentPhaseStatus, String> {
    get_current_phase_status(conn, LZU_SEMESTER_CONTEXT_SOURCE)
}

fn unknown_status(source: &str) -> CurrentPhaseStatus {
    CurrentPhaseStatus {
        source: source.to_string(),
        term_label: None,
        phase_type: PHASE_UNKNOWN.to_string(),
        current_week: None,
        start_week: None,
        end_week: None,
        courses_visible: true,
        exam_notifications_enabled: true,
        pomodoro_profile: "default".to_string(),
        inferred: true,
    }
}

fn status_from_phase(source: &str, current_week: i64, phase: TermPhase) -> CurrentPhaseStatus {
    CurrentPhaseStatus {
        source: source.to_string(),
        term_label: Some(phase.term_label),
        phase_type: phase.phase_type,
        current_week: Some(current_week),
        start_week: Some(phase.start_week),
        end_week: Some(phase.end_week),
        courses_visible: phase.affects_courses,
        exam_notifications_enabled: phase.affects_exam_notifications,
        pomodoro_profile: phase.pomodoro_profile,
        inferred: false,
    }
}

/// 教学周但该学期无任何活跃课程 → 降级为假期。
///
/// 覆盖 LZU 返回暑期小学期等无课学期上下文的场景：学期锚点日期落在暑假，
/// 但用户没有该学期课程时不应显示"教学周"（见 08-01 暑假误报排查）。
fn downgrade_teaching_if_no_courses(
    conn: &Connection,
    source: &str,
    current_week: i64,
    phase: TermPhase,
) -> CurrentPhaseStatus {
    if phase.phase_type != PHASE_TEACHING {
        return status_from_phase(source, current_week, phase);
    }

    // 查询失败时保守视为有课（不降级），避免 DB 异常导致真实教学周被误判假期。
    let has_courses = conn
        .query_row(
            "SELECT EXISTS(
                SELECT 1 FROM courses WHERE semester = ?1 AND deleted_at IS NULL
             )",
            params![phase.term_label],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(1);

    if has_courses > 0 {
        return status_from_phase(source, current_week, phase);
    }

    let mut status = status_from_phase(source, current_week, phase);
    status.phase_type = PHASE_BREAK.to_string();
    status.current_week = None;
    status.start_week = None;
    status.end_week = None;
    status.courses_visible = false;
    status.exam_notifications_enabled = false;
    status.pomodoro_profile = "relaxed".to_string();
    status.inferred = true;
    status
}

fn downgrade_inferred_teaching_if_no_courses(
    conn: &Connection,
    status: CurrentPhaseStatus,
    term_label: &str,
) -> CurrentPhaseStatus {
    if status.phase_type != PHASE_TEACHING || term_label.is_empty() {
        return status;
    }

    let has_courses = conn
        .query_row(
            "SELECT EXISTS(
                SELECT 1 FROM courses WHERE semester = ?1 AND deleted_at IS NULL
             )",
            params![term_label],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(1);

    if has_courses > 0 {
        return status;
    }

    CurrentPhaseStatus {
        source: status.source,
        term_label: status.term_label,
        phase_type: PHASE_BREAK.to_string(),
        current_week: None,
        start_week: None,
        end_week: None,
        courses_visible: false,
        exam_notifications_enabled: false,
        pomodoro_profile: "relaxed".to_string(),
        inferred: true,
    }
}

fn inferred_status(
    source: &str,
    context: &SemesterContext,
    current_week: i64,
) -> CurrentPhaseStatus {
    inferred_status_from_parts(
        source,
        Some(context.term_label.clone()),
        current_week,
        context.total_weeks.unwrap_or(FALLBACK_TOTAL_WEEKS),
    )
}

fn inferred_status_from_parts(
    source: &str,
    term_label: Option<String>,
    current_week: i64,
    total_weeks: i64,
) -> CurrentPhaseStatus {
    let is_break = current_week > total_weeks;
    CurrentPhaseStatus {
        source: source.to_string(),
        term_label,
        phase_type: if is_break {
            PHASE_BREAK
        } else {
            PHASE_TEACHING
        }
        .to_string(),
        current_week: Some(current_week),
        start_week: None,
        end_week: None,
        courses_visible: !is_break,
        exam_notifications_enabled: !is_break,
        pomodoro_profile: if is_break { "relaxed" } else { "default" }.to_string(),
        inferred: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations;
    use crate::db::models::{CreateTermPhaseRequest, UpsertSemesterContextRequest};

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().expect("Failed to open in-memory DB");
        conn.pragma_update(None, "foreign_keys", "ON")
            .expect("Failed to enable foreign keys");
        migrations::run_migrations(&conn).expect("Migrations failed");
        conn
    }

    fn insert_course(conn: &Connection, semester: &str) {
        conn.execute(
            "INSERT INTO courses
                (name, day_of_week, start_time, end_time, week_pattern, semester_start_date, semester, created_at, updated_at)
             VALUES ('测试课程', 1, '08:00', '09:40', '1-17周全周', '2026-02-24', ?1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            params![semester],
        )
        .expect("insert course");
    }

    fn sample_context(total_weeks: Option<i64>) -> UpsertSemesterContextRequest {
        UpsertSemesterContextRequest {
            source: LZU_SEMESTER_CONTEXT_SOURCE.to_string(),
            academic_year: Some("2026".to_string()),
            term: Some("1".to_string()),
            term_label: "2026S1".to_string(),
            start_date: "2026-02-24".to_string(),
            current_week: Some(1),
            total_weeks,
        }
    }

    fn sample_phase(phase_type: &str, start_week: i64, end_week: i64) -> CreateTermPhaseRequest {
        CreateTermPhaseRequest {
            term_label: "2026S1".to_string(),
            phase_type: phase_type.to_string(),
            start_week,
            end_week,
            affects_courses: phase_type != PHASE_BREAK,
            affects_exam_notifications: phase_type != PHASE_BREAK,
            pomodoro_profile: if phase_type == PHASE_EXAM {
                "intense".to_string()
            } else {
                "default".to_string()
            },
            notification_rules_json: "{}".to_string(),
            sort_order: start_week,
        }
    }

    #[test]
    fn test_unknown_without_semester_context() {
        let conn = setup_db();

        let status = get_phase_status_for_date(
            &conn,
            LZU_SEMESTER_CONTEXT_SOURCE,
            NaiveDate::from_ymd_opt(2026, 2, 24).unwrap(),
        )
        .expect("phase status");

        assert_eq!(status.phase_type, PHASE_UNKNOWN);
        assert!(status.current_week.is_none());
    }

    #[test]
    fn test_fallback_teaching_and_break() {
        let conn = setup_db();
        crate::db::semester::upsert_semester_context(&conn, &sample_context(Some(16)))
            .expect("upsert context");
        insert_course(&conn, "2026S1");

        let teaching = get_phase_status_for_date(
            &conn,
            LZU_SEMESTER_CONTEXT_SOURCE,
            NaiveDate::from_ymd_opt(2026, 2, 26).unwrap(),
        )
        .expect("teaching status");
        assert_eq!(teaching.phase_type, PHASE_TEACHING);
        assert!(teaching.courses_visible);

        let break_phase = get_phase_status_for_date(
            &conn,
            LZU_SEMESTER_CONTEXT_SOURCE,
            NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
        )
        .expect("break status");
        assert_eq!(break_phase.phase_type, PHASE_BREAK);
        assert!(!break_phase.exam_notifications_enabled);
    }

    #[test]
    fn test_explicit_phase_overrides_fallback() {
        let conn = setup_db();
        crate::db::semester::upsert_semester_context(&conn, &sample_context(Some(16)))
            .expect("upsert context");
        crate::db::term_phases::create_term_phase(&conn, &sample_phase(PHASE_EXAM, 1, 1))
            .expect("create phase");
        insert_course(&conn, "2026S1");

        let status = get_phase_status_for_date(
            &conn,
            LZU_SEMESTER_CONTEXT_SOURCE,
            NaiveDate::from_ymd_opt(2026, 2, 26).unwrap(),
        )
        .expect("phase status");

        assert_eq!(status.phase_type, PHASE_EXAM);
        assert_eq!(status.pomodoro_profile, "intense");
        assert!(!status.inferred);
    }

    #[test]
    fn test_explicit_teaching_without_courses_downgrades_to_break() {
        // 回归：LZU 暑期小学期上下文（teaching 阶段）但无任何课程时，应显示假期而非教学周。
        let conn = setup_db();
        crate::db::semester::upsert_semester_context(&conn, &sample_context(Some(16)))
            .expect("upsert context");
        crate::db::term_phases::create_term_phase(&conn, &sample_phase(PHASE_TEACHING, 1, 16))
            .expect("create phase");
        // 不插入任何课程。

        let status = get_phase_status_for_date(
            &conn,
            LZU_SEMESTER_CONTEXT_SOURCE,
            NaiveDate::from_ymd_opt(2026, 2, 26).unwrap(),
        )
        .expect("phase status");

        assert_eq!(status.phase_type, PHASE_BREAK);
        assert!(!status.courses_visible);
        assert!(!status.exam_notifications_enabled);
        assert_eq!(status.pomodoro_profile, "relaxed");
        assert!(status.inferred);
    }

    #[test]
    fn test_inferred_teaching_without_courses_downgrades_to_break() {
        // 回归：无显式 term_phase、由周数推断 teaching 但该学期无课，同样降级为假期。
        let conn = setup_db();
        crate::db::semester::upsert_semester_context(&conn, &sample_context(Some(16)))
            .expect("upsert context");

        let status = get_phase_status_for_date(
            &conn,
            LZU_SEMESTER_CONTEXT_SOURCE,
            NaiveDate::from_ymd_opt(2026, 2, 26).unwrap(),
        )
        .expect("phase status");

        assert_eq!(status.phase_type, PHASE_BREAK);
        assert!(!status.courses_visible);
    }

    #[test]
    fn test_term_week_teaching_without_courses_downgrades_to_break() {
        // 课表页入口同样降级：查看无课学期某教学周时应为假期。
        let conn = setup_db();
        crate::db::semester::upsert_semester_context(&conn, &sample_context(Some(16)))
            .expect("upsert context");
        crate::db::term_phases::create_term_phase(&conn, &sample_phase(PHASE_TEACHING, 1, 16))
            .expect("create phase");

        let status = get_phase_status_for_term_week(
            &conn,
            LZU_SEMESTER_CONTEXT_SOURCE,
            "2026S1",
            2,
        )
        .expect("phase status");

        assert_eq!(status.phase_type, PHASE_BREAK);
        assert!(!status.courses_visible);
    }
}
