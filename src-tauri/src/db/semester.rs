use rusqlite::{params, Connection, Result, Row};

use super::models::{SemesterContext, UpsertSemesterContextRequest};

pub const LZU_SEMESTER_CONTEXT_SOURCE: &str = "lzu";

pub fn upsert_semester_context(
    conn: &Connection,
    req: &UpsertSemesterContextRequest,
) -> Result<i64> {
    let now = super::chrono_now();
    conn.execute(
        "INSERT INTO semester_context
            (source, academic_year, term, term_label, start_date, current_week, total_weeks, refreshed_at, created_at, updated_at)
         VALUES
            (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8, ?8)
         ON CONFLICT(source, term_label) DO UPDATE SET
            academic_year = excluded.academic_year,
            term = excluded.term,
            start_date = excluded.start_date,
            current_week = excluded.current_week,
            total_weeks = excluded.total_weeks,
            refreshed_at = excluded.refreshed_at,
            updated_at = excluded.updated_at",
        params![
            req.source,
            req.academic_year,
            req.term,
            req.term_label,
            req.start_date,
            req.current_week,
            req.total_weeks,
            now,
        ],
    )?;

    conn.query_row(
        "SELECT id FROM semester_context WHERE source = ?1 AND term_label = ?2",
        params![req.source, req.term_label],
        |row| row.get(0),
    )
}

pub fn get_semester_context_by_source_and_term_label(
    conn: &Connection,
    source: &str,
    term_label: &str,
) -> Result<Option<SemesterContext>> {
    optional_context(
        conn.query_row(
            "SELECT id, source, academic_year, term, term_label, start_date, current_week, total_weeks, refreshed_at, created_at, updated_at
             FROM semester_context
             WHERE source = ?1 AND term_label = ?2",
            params![source, term_label],
            row_to_semester_context,
        ),
    )
}

pub fn get_latest_semester_context_by_source(
    conn: &Connection,
    source: &str,
) -> Result<Option<SemesterContext>> {
    optional_context(
        conn.query_row(
            "SELECT id, source, academic_year, term, term_label, start_date, current_week, total_weeks, refreshed_at, created_at, updated_at
             FROM semester_context
             WHERE source = ?1
             ORDER BY refreshed_at DESC, id DESC
             LIMIT 1",
            params![source],
            row_to_semester_context,
        ),
    )
}

pub fn find_semester_context(
    conn: &Connection,
    source: &str,
    semester: &str,
) -> Result<Option<SemesterContext>> {
    let term_label = semester.trim();
    if term_label.is_empty() {
        return get_latest_semester_context_by_source(conn, source);
    }

    get_semester_context_by_source_and_term_label(conn, source, term_label)
}

pub fn get_all_semester_contexts(conn: &Connection) -> Result<Vec<SemesterContext>> {
    let mut stmt = conn.prepare(
        "SELECT id, source, academic_year, term, term_label, start_date, current_week, total_weeks, refreshed_at, created_at, updated_at
         FROM semester_context
         ORDER BY source, term_label",
    )?;

    let rows = stmt.query_map([], row_to_semester_context)?;
    rows.collect()
}

/// 手动统一课程学期开始日期时，同步校准上下文，避免后续视图绕过用户覆盖。
pub fn update_all_semester_context_start_dates(conn: &Connection, date: &str) -> Result<usize> {
    let now = super::chrono_now();
    conn.execute(
        "UPDATE semester_context SET start_date = ?1, updated_at = ?2",
        params![date, now],
    )
}

fn optional_context(result: Result<SemesterContext>) -> Result<Option<SemesterContext>> {
    match result {
        Ok(context) => Ok(Some(context)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(err) => Err(err),
    }
}

fn row_to_semester_context(row: &Row<'_>) -> Result<SemesterContext> {
    Ok(SemesterContext {
        id: row.get(0)?,
        source: row.get(1)?,
        academic_year: row.get(2)?,
        term: row.get(3)?,
        term_label: row.get(4)?,
        start_date: row.get(5)?,
        current_week: row.get(6)?,
        total_weeks: row.get(7)?,
        refreshed_at: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
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

    fn sample_context() -> UpsertSemesterContextRequest {
        UpsertSemesterContextRequest {
            source: LZU_SEMESTER_CONTEXT_SOURCE.to_string(),
            academic_year: Some("2026".to_string()),
            term: Some("1".to_string()),
            term_label: "2026S1".to_string(),
            start_date: "2026-02-24".to_string(),
            current_week: Some(3),
            total_weeks: Some(16),
        }
    }

    fn find_required_context(conn: &Connection) -> SemesterContext {
        find_semester_context(conn, LZU_SEMESTER_CONTEXT_SOURCE, "2026S1")
            .expect("find context")
            .expect("context exists")
    }

    #[test]
    fn test_find_semester_context_returns_none_when_empty() {
        let conn = setup_db();
        let context = find_semester_context(&conn, LZU_SEMESTER_CONTEXT_SOURCE, "2026S1")
            .expect("find context");
        assert!(context.is_none());
    }

    #[test]
    fn test_upsert_and_find_semester_context() {
        let conn = setup_db();
        let req = sample_context();
        let id = upsert_semester_context(&conn, &req).expect("upsert context");
        assert!(id > 0);

        let context = find_required_context(&conn);
        assert_eq!(context.id, id);
        assert_eq!(context.start_date, "2026-02-24");
        assert_eq!(context.current_week, Some(3));
        assert_eq!(context.total_weeks, Some(16));
    }

    #[test]
    fn test_upsert_replaces_existing_context() {
        let conn = setup_db();
        let mut req = sample_context();
        let first_id = upsert_semester_context(&conn, &req).expect("first upsert");

        req.start_date = "2026-03-02".to_string();
        req.current_week = Some(4);
        req.total_weeks = None;
        let second_id = upsert_semester_context(&conn, &req).expect("second upsert");

        assert_eq!(first_id, second_id);
        let context = find_required_context(&conn);
        assert_eq!(context.start_date, "2026-03-02");
        assert_eq!(context.current_week, Some(4));
        assert_eq!(context.total_weeks, None);
    }

    #[test]
    fn test_total_weeks_can_be_unknown() {
        let conn = setup_db();
        let mut req = sample_context();
        req.total_weeks = None;

        upsert_semester_context(&conn, &req).expect("upsert context");

        let context = find_required_context(&conn);
        assert_eq!(context.total_weeks, None);
    }

    #[test]
    fn test_week_constraints_reject_non_positive_values() {
        let conn = setup_db();
        let mut req = sample_context();
        req.current_week = Some(0);
        assert!(upsert_semester_context(&conn, &req).is_err());

        req.current_week = Some(1);
        req.total_weeks = Some(0);
        assert!(upsert_semester_context(&conn, &req).is_err());
    }

    #[test]
    fn test_find_empty_semester_uses_latest_context() {
        let conn = setup_db();
        let mut old = sample_context();
        old.term_label = "2026S1".to_string();
        upsert_semester_context(&conn, &old).expect("upsert old");

        let mut latest = sample_context();
        latest.term = Some("2".to_string());
        latest.term_label = "2026S2".to_string();
        latest.start_date = "2026-09-01".to_string();
        upsert_semester_context(&conn, &latest).expect("upsert latest");

        let context = find_semester_context(&conn, LZU_SEMESTER_CONTEXT_SOURCE, "")
            .expect("find latest")
            .expect("latest exists");
        assert_eq!(context.term_label, "2026S2");
    }

    #[test]
    fn test_manual_start_date_update_updates_all_contexts() {
        let conn = setup_db();
        let mut first = sample_context();
        first.term_label = "2026S1".to_string();
        upsert_semester_context(&conn, &first).expect("upsert first");

        let mut second = sample_context();
        second.term_label = "2026S2".to_string();
        second.start_date = "2026-09-01".to_string();
        upsert_semester_context(&conn, &second).expect("upsert second");

        let updated =
            update_all_semester_context_start_dates(&conn, "2026-03-02").expect("update contexts");
        assert_eq!(updated, 2);

        let contexts = get_all_semester_contexts(&conn).expect("get contexts");
        assert_eq!(contexts.len(), 2);
        assert!(contexts
            .iter()
            .all(|context| context.start_date == "2026-03-02"));
    }
}
