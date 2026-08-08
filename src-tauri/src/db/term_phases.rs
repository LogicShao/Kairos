use rusqlite::{params, Connection, OptionalExtension, Result, Row};

use super::models::{CreateTermPhaseRequest, TermPhase, UpdateTermPhaseRequest};

pub fn get_term_phases(conn: &Connection, term_label: &str) -> Result<Vec<TermPhase>> {
    let mut stmt = conn.prepare(
        "SELECT id, sync_id, term_label, phase_type, start_week, end_week,
                affects_courses, affects_exam_notifications, pomodoro_profile,
                notification_rules_json, sort_order, deleted_at, created_at, updated_at
         FROM term_phases
         WHERE term_label = ?1 AND deleted_at IS NULL
         ORDER BY sort_order ASC, start_week ASC, id ASC",
    )?;
    let rows = stmt.query_map(params![term_label], row_to_term_phase)?;
    rows.collect()
}

pub fn get_all_term_phases_for_sync(conn: &Connection) -> Result<Vec<TermPhase>> {
    let mut stmt = conn.prepare(
        "SELECT id, sync_id, term_label, phase_type, start_week, end_week,
                affects_courses, affects_exam_notifications, pomodoro_profile,
                notification_rules_json, sort_order, deleted_at, created_at, updated_at
         FROM term_phases
         ORDER BY id",
    )?;
    let rows = stmt.query_map([], row_to_term_phase)?;
    rows.collect()
}

pub fn get_term_phase(conn: &Connection, id: i64) -> Result<TermPhase> {
    conn.query_row(
        "SELECT id, sync_id, term_label, phase_type, start_week, end_week,
                affects_courses, affects_exam_notifications, pomodoro_profile,
                notification_rules_json, sort_order, deleted_at, created_at, updated_at
         FROM term_phases
         WHERE id = ?1",
        params![id],
        row_to_term_phase,
    )
}

pub fn get_term_phase_by_week(
    conn: &Connection,
    term_label: &str,
    week_index: i64,
) -> Result<Option<TermPhase>> {
    conn.query_row(
        "SELECT id, sync_id, term_label, phase_type, start_week, end_week,
                affects_courses, affects_exam_notifications, pomodoro_profile,
                notification_rules_json, sort_order, deleted_at, created_at, updated_at
         FROM term_phases
         WHERE term_label = ?1
           AND deleted_at IS NULL
           AND start_week <= ?2
           AND end_week >= ?2
         ORDER BY sort_order ASC, start_week DESC, id DESC
         LIMIT 1",
        params![term_label, week_index],
        row_to_term_phase,
    )
    .optional()
}

pub fn create_term_phase(conn: &Connection, req: &CreateTermPhaseRequest) -> Result<i64> {
    let now = super::chrono_now();
    conn.execute(
        "INSERT INTO term_phases
            (sync_id, term_label, phase_type, start_week, end_week, affects_courses,
             affects_exam_notifications, pomodoro_profile, notification_rules_json,
             sort_order, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11)",
        params![
            crate::sync::ids::new_sync_id(),
            req.term_label.trim(),
            req.phase_type,
            req.start_week,
            req.end_week,
            req.affects_courses,
            req.affects_exam_notifications,
            normalized_profile(&req.pomodoro_profile),
            normalized_rules(&req.notification_rules_json),
            req.sort_order,
            now,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update_term_phase(conn: &Connection, id: i64, req: &UpdateTermPhaseRequest) -> Result<()> {
    let now = super::chrono_now();
    conn.execute(
        "UPDATE term_phases
         SET term_label = ?1, phase_type = ?2, start_week = ?3, end_week = ?4,
             affects_courses = ?5, affects_exam_notifications = ?6,
             pomodoro_profile = ?7, notification_rules_json = ?8,
             sort_order = ?9, updated_at = ?10
         WHERE id = ?11 AND deleted_at IS NULL",
        params![
            req.term_label.trim(),
            req.phase_type,
            req.start_week,
            req.end_week,
            req.affects_courses,
            req.affects_exam_notifications,
            normalized_profile(&req.pomodoro_profile),
            normalized_rules(&req.notification_rules_json),
            req.sort_order,
            now,
            id,
        ],
    )?;
    Ok(())
}

pub fn soft_delete_term_phase(conn: &Connection, id: i64) -> Result<()> {
    let now = super::chrono_now();
    conn.execute(
        "UPDATE term_phases
         SET deleted_at = ?1, updated_at = ?1
         WHERE id = ?2 AND deleted_at IS NULL",
        params![now, id],
    )?;
    Ok(())
}

pub fn ensure_default_phases(
    conn: &Connection,
    term_label: &str,
    total_weeks: i64,
) -> Result<usize> {
    if total_weeks < 1 || !get_term_phases(conn, term_label)?.is_empty() {
        return Ok(0);
    }

    let exam_start = (total_weeks - 1).max(1);
    let teaching_end = (exam_start - 1).max(1);

    let mut created = 0usize;
    if teaching_end >= 1 {
        let teaching = CreateTermPhaseRequest {
            term_label: term_label.to_string(),
            phase_type: "teaching".to_string(),
            start_week: 1,
            end_week: teaching_end,
            affects_courses: true,
            affects_exam_notifications: true,
            pomodoro_profile: "default".to_string(),
            notification_rules_json: "{}".to_string(),
            sort_order: 0,
        };
        create_term_phase(conn, &teaching)?;
        created += 1;
    }

    let exam = CreateTermPhaseRequest {
        term_label: term_label.to_string(),
        phase_type: "exam".to_string(),
        start_week: exam_start,
        end_week: total_weeks,
        affects_courses: true,
        affects_exam_notifications: true,
        pomodoro_profile: "intense".to_string(),
        notification_rules_json: "{}".to_string(),
        sort_order: 1,
    };
    create_term_phase(conn, &exam)?;
    created += 1;

    Ok(created)
}

pub fn upsert_term_phase_from_sync(conn: &Connection, phase: &TermPhase) -> Result<bool> {
    let normalized = normalized_phase(phase);
    let local = conn
        .query_row(
            "SELECT id, updated_at, deleted_at FROM term_phases WHERE sync_id = ?1",
            params![normalized.sync_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            },
        )
        .optional()?;

    match local {
        None => {
            conn.execute(
                "INSERT INTO term_phases
                    (sync_id, term_label, phase_type, start_week, end_week, affects_courses,
                     affects_exam_notifications, pomodoro_profile, notification_rules_json,
                     sort_order, deleted_at, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    normalized.sync_id,
                    normalized.term_label,
                    normalized.phase_type,
                    normalized.start_week,
                    normalized.end_week,
                    normalized.affects_courses,
                    normalized.affects_exam_notifications,
                    normalized.pomodoro_profile,
                    normalized.notification_rules_json,
                    normalized.sort_order,
                    normalized.deleted_at,
                    normalized.created_at,
                    normalized.updated_at,
                ],
            )?;
            Ok(true)
        }
        Some((id, updated_at, deleted_at)) => {
            let remote_ts = normalized
                .deleted_at
                .as_deref()
                .unwrap_or(&normalized.updated_at);
            let local_ts = deleted_at.as_deref().unwrap_or(&updated_at);
            if remote_ts <= local_ts {
                return Ok(false);
            }

            conn.execute(
                "UPDATE term_phases
                 SET term_label = ?1, phase_type = ?2, start_week = ?3, end_week = ?4,
                     affects_courses = ?5, affects_exam_notifications = ?6,
                     pomodoro_profile = ?7, notification_rules_json = ?8,
                     sort_order = ?9, deleted_at = ?10, created_at = ?11, updated_at = ?12
                 WHERE id = ?13",
                params![
                    normalized.term_label,
                    normalized.phase_type,
                    normalized.start_week,
                    normalized.end_week,
                    normalized.affects_courses,
                    normalized.affects_exam_notifications,
                    normalized.pomodoro_profile,
                    normalized.notification_rules_json,
                    normalized.sort_order,
                    normalized.deleted_at,
                    normalized.created_at,
                    normalized.updated_at,
                    id,
                ],
            )?;
            Ok(true)
        }
    }
}

fn normalized_phase(phase: &TermPhase) -> TermPhase {
    let mut normalized = phase.clone();
    if normalized.sync_id.is_empty() {
        normalized.sync_id = format!("legacy-term-phase-{}", normalized.id);
    }
    normalized
}

fn normalized_profile(value: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        "default".to_string()
    } else {
        value.to_string()
    }
}

fn normalized_rules(value: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        "{}".to_string()
    } else {
        value.to_string()
    }
}

fn row_to_term_phase(row: &Row<'_>) -> Result<TermPhase> {
    Ok(TermPhase {
        id: row.get(0)?,
        sync_id: row.get(1)?,
        term_label: row.get(2)?,
        phase_type: row.get(3)?,
        start_week: row.get(4)?,
        end_week: row.get(5)?,
        affects_courses: row.get(6)?,
        affects_exam_notifications: row.get(7)?,
        pomodoro_profile: row.get(8)?,
        notification_rules_json: row.get(9)?,
        sort_order: row.get(10)?,
        deleted_at: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::setup_db;

    fn sample_phase(phase_type: &str, start_week: i64, end_week: i64) -> CreateTermPhaseRequest {
        CreateTermPhaseRequest {
            term_label: "2026S1".to_string(),
            phase_type: phase_type.to_string(),
            start_week,
            end_week,
            affects_courses: phase_type != "break",
            affects_exam_notifications: phase_type != "break",
            pomodoro_profile: "default".to_string(),
            notification_rules_json: "{}".to_string(),
            sort_order: start_week,
        }
    }

    #[test]
    fn test_create_get_update_delete_term_phase() {
        let conn = setup_db();
        let id = create_term_phase(&conn, &sample_phase("teaching", 1, 16)).expect("create phase");

        let update = UpdateTermPhaseRequest {
            term_label: "2026S1".to_string(),
            phase_type: "break".to_string(),
            start_week: 1,
            end_week: 16,
            affects_courses: false,
            affects_exam_notifications: false,
            pomodoro_profile: "default".to_string(),
            notification_rules_json: "{}".to_string(),
            sort_order: 1,
        };
        update_term_phase(&conn, id, &update).expect("update phase");

        let phase = get_term_phase(&conn, id).expect("get phase");
        assert_eq!(phase.phase_type, "break");
        assert!(!phase.affects_courses);

        soft_delete_term_phase(&conn, id).expect("delete phase");
        assert!(get_term_phases(&conn, "2026S1")
            .expect("list phases")
            .is_empty());
        assert!(get_term_phase(&conn, id)
            .expect("get tombstone")
            .deleted_at
            .is_some());
    }

    #[test]
    fn test_get_term_phase_by_week_prefers_matching_range() {
        let conn = setup_db();
        create_term_phase(&conn, &sample_phase("teaching", 1, 16)).expect("teaching");
        create_term_phase(&conn, &sample_phase("exam", 17, 18)).expect("exam");

        let phase = get_term_phase_by_week(&conn, "2026S1", 17)
            .expect("query phase")
            .expect("phase exists");

        assert_eq!(phase.phase_type, "exam");
    }

    #[test]
    fn test_ensure_default_phases_is_idempotent() {
        let conn = setup_db();
        assert_eq!(
            ensure_default_phases(&conn, "2026S1", 16).expect("ensure defaults"),
            2
        );
        assert_eq!(
            ensure_default_phases(&conn, "2026S1", 16).expect("ensure again"),
            0
        );

        let phases = get_term_phases(&conn, "2026S1").expect("list phases");
        assert_eq!(phases.len(), 2);
        assert_eq!(phases[0].phase_type, "teaching");
        assert_eq!(phases[1].phase_type, "exam");
    }

    #[test]
    fn test_sync_newer_tombstone_wins() {
        let conn = setup_db();
        let id = create_term_phase(&conn, &sample_phase("teaching", 1, 16)).expect("create phase");
        let mut phase = get_term_phase(&conn, id).expect("get phase");
        phase.deleted_at = Some("2999-01-01T00:00:00Z".to_string());
        phase.updated_at = "2026-01-01T00:00:00Z".to_string();

        assert!(upsert_term_phase_from_sync(&conn, &phase).expect("sync phase"));
        assert!(get_term_phase(&conn, id)
            .expect("get phase")
            .deleted_at
            .is_some());
    }
}
