use rusqlite::{params, Connection, Result};

use super::models::{CreateExamRequest, Exam, UpdateExamRequest};

pub fn create_exam(conn: &Connection, req: &CreateExamRequest) -> Result<i64> {
    conn.execute(
        "INSERT INTO exams (course_name, exam_datetime, location, notes, course_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
        params![
            req.course_name,
            req.exam_datetime,
            req.location,
            req.notes,
            req.course_id,
            super::chrono_now(),
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_exam(conn: &Connection, id: i64) -> Result<Exam> {
    conn.query_row(
        "SELECT id, course_name, exam_datetime, location, notes, course_id, created_at, updated_at
         FROM exams WHERE id = ?1",
        params![id],
        |row| {
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
        },
    )
}

pub fn get_all_exams(conn: &Connection) -> Result<Vec<Exam>> {
    let mut stmt = conn.prepare(
        "SELECT id, course_name, exam_datetime, location, notes, course_id, created_at, updated_at
         FROM exams
         ORDER BY exam_datetime ASC",
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

pub fn update_exam(conn: &Connection, id: i64, req: &UpdateExamRequest) -> Result<()> {
    conn.execute(
        "UPDATE exams
         SET course_name = ?1, exam_datetime = ?2, location = ?3, notes = ?4, course_id = ?5, updated_at = ?6
         WHERE id = ?7",
        params![
            req.course_name,
            req.exam_datetime,
            req.location,
            req.notes,
            req.course_id,
            super::chrono_now(),
            id,
        ],
    )?;
    Ok(())
}

pub fn delete_exam(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM exams WHERE id = ?1", params![id])?;
    Ok(())
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

    fn sample_exam(name: &str) -> CreateExamRequest {
        CreateExamRequest {
            course_name: name.to_string(),
            exam_datetime: String::from("2024-12-15T09:00:00Z"),
            location: String::from("Hall A"),
            notes: String::new(),
            course_id: None,
        }
    }

    #[test]
    fn test_create_and_get_exam() {
        let conn = setup_db();

        let req = sample_exam("Calculus Final");
        let id = create_exam(&conn, &req).expect("Failed to create exam");
        assert!(id > 0);

        let exam = get_exam(&conn, id).expect("Failed to get exam");
        assert_eq!(exam.course_name, "Calculus Final");
        assert_eq!(exam.exam_datetime, "2024-12-15T09:00:00Z");
        assert_eq!(exam.location, "Hall A");
        assert_eq!(exam.course_id, None);
    }

    #[test]
    fn test_get_nonexistent_exam() {
        let conn = setup_db();
        let result = get_exam(&conn, 999);
        assert!(result.is_err());
    }

    #[test]
    fn test_update_exam() {
        let conn = setup_db();

        let course_req = crate::db::models::CreateCourseRequest {
            name: String::from("Physics"),
            day_of_week: 2,
            start_time: String::from("10:00"),
            end_time: String::from("11:30"),
            location: String::new(),
            teacher: String::new(),
            color: String::new(),
            semester: String::from("2024S1"),
        };
        let course_id = crate::db::courses::create_course(&conn, &course_req)
            .expect("Failed to create course");

        let req = sample_exam("Physics Final");
        let id = create_exam(&conn, &req).expect("Failed to create exam");

        let update = UpdateExamRequest {
            course_name: String::from("Physics Final (Updated)"),
            exam_datetime: String::from("2024-12-20T14:00:00Z"),
            location: String::from("Hall B"),
            notes: String::from("Bring calculator"),
            course_id: Some(course_id),
        };
        update_exam(&conn, id, &update).expect("Failed to update exam");

        let exam = get_exam(&conn, id).expect("Failed to get updated exam");
        assert_eq!(exam.course_name, "Physics Final (Updated)");
        assert_eq!(exam.location, "Hall B");
        assert_eq!(exam.notes, "Bring calculator");
        assert_eq!(exam.course_id, Some(course_id));
    }

    #[test]
    fn test_delete_exam() {
        let conn = setup_db();

        let req = sample_exam("Chemistry Final");
        let id = create_exam(&conn, &req).expect("Failed to create exam");

        delete_exam(&conn, id).expect("Failed to delete exam");

        let result = get_exam(&conn, id);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_all_exams() {
        let conn = setup_db();

        let e1 = CreateExamRequest {
            course_name: String::from("Exam A"),
            exam_datetime: String::from("2024-12-15T09:00:00Z"),
            location: String::new(),
            notes: String::new(),
            course_id: None,
        };
        let e2 = CreateExamRequest {
            course_name: String::from("Exam B"),
            exam_datetime: String::from("2024-12-10T09:00:00Z"),
            location: String::new(),
            notes: String::new(),
            course_id: None,
        };

        create_exam(&conn, &e1).expect("Failed to create e1");
        create_exam(&conn, &e2).expect("Failed to create e2");

        let exams = get_all_exams(&conn).expect("Failed to get all exams");
        assert_eq!(exams.len(), 2);
        assert_eq!(exams[0].course_name, "Exam B");
        assert_eq!(exams[1].course_name, "Exam A");
    }

    #[test]
    fn test_get_all_exams_empty() {
        let conn = setup_db();
        let exams = get_all_exams(&conn).expect("Failed to get exams");
        assert!(exams.is_empty());
    }
}
