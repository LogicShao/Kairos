use rusqlite::{params, Connection, Result};

use super::models::{Course, CreateCourseRequest, UpdateCourseRequest};

pub fn create_course(conn: &Connection, req: &CreateCourseRequest) -> Result<i64> {
    conn.execute(
        "INSERT INTO courses (name, day_of_week, start_time, end_time, location, teacher, color, semester, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
        params![
            req.name,
            req.day_of_week,
            req.start_time,
            req.end_time,
            req.location,
            req.teacher,
            req.color,
            req.semester,
            super::chrono_now(),
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_course(conn: &Connection, id: i64) -> Result<Course> {
    conn.query_row(
        "SELECT id, name, day_of_week, start_time, end_time, location, teacher, color, semester, created_at, updated_at
         FROM courses WHERE id = ?1",
        params![id],
        |row| {
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
        },
    )
}

pub fn get_all_courses(conn: &Connection, semester: Option<&str>) -> Result<Vec<Course>> {
    let mut sql = String::from(
        "SELECT id, name, day_of_week, start_time, end_time, location, teacher, color, semester, created_at, updated_at
         FROM courses",
    );
    let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(sem) = semester {
        sql.push_str(" WHERE semester = ?1");
        params_vec.push(Box::new(sem.to_string()));
    }
    sql.push_str(" ORDER BY day_of_week, start_time");

    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
    let mut stmt = conn.prepare(&sql)?;

    let rows = stmt.query_map(param_refs.as_slice(), |row| {
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

pub fn update_course(conn: &Connection, id: i64, req: &UpdateCourseRequest) -> Result<()> {
    conn.execute(
        "UPDATE courses
         SET name = ?1, day_of_week = ?2, start_time = ?3, end_time = ?4,
             location = ?5, teacher = ?6, color = ?7, semester = ?8, updated_at = ?9
         WHERE id = ?10",
        params![
            req.name,
            req.day_of_week,
            req.start_time,
            req.end_time,
            req.location,
            req.teacher,
            req.color,
            req.semester,
            super::chrono_now(),
            id,
        ],
    )?;
    Ok(())
}

pub fn delete_course(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM courses WHERE id = ?1", params![id])?;
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

    fn sample_course(name: &str) -> CreateCourseRequest {
        CreateCourseRequest {
            name: name.to_string(),
            day_of_week: 1,
            start_time: String::from("08:00"),
            end_time: String::from("09:30"),
            location: String::from("Room 101"),
            teacher: String::from("Prof. Smith"),
            color: String::from("#3B82F6"),
            semester: String::from("2024S1"),
        }
    }

    #[test]
    fn test_create_and_get_course() {
        let conn = setup_db();

        let req = sample_course("Math 101");
        let id = create_course(&conn, &req).expect("Failed to create course");
        assert!(id > 0);

        let course = get_course(&conn, id).expect("Failed to get course");
        assert_eq!(course.name, "Math 101");
        assert_eq!(course.day_of_week, 1);
        assert_eq!(course.start_time, "08:00");
        assert_eq!(course.color, "#3B82F6");
    }

    #[test]
    fn test_get_nonexistent_course() {
        let conn = setup_db();
        let result = get_course(&conn, 999);
        assert!(result.is_err());
    }

    #[test]
    fn test_update_course() {
        let conn = setup_db();

        let req = sample_course("Physics");
        let id = create_course(&conn, &req).expect("Failed to create course");

        let update = UpdateCourseRequest {
            name: String::from("Physics 201"),
            day_of_week: 3,
            start_time: String::from("10:00"),
            end_time: String::from("11:30"),
            location: String::from("Lab B"),
            teacher: String::from("Dr. Jones"),
            color: String::from("#EF4444"),
            semester: String::from("2024S2"),
        };
        update_course(&conn, id, &update).expect("Failed to update course");

        let course = get_course(&conn, id).expect("Failed to get updated course");
        assert_eq!(course.name, "Physics 201");
        assert_eq!(course.day_of_week, 3);
        assert_eq!(course.color, "#EF4444");
    }

    #[test]
    fn test_delete_course() {
        let conn = setup_db();

        let req = sample_course("Biology");
        let id = create_course(&conn, &req).expect("Failed to create course");

        delete_course(&conn, id).expect("Failed to delete course");

        let result = get_course(&conn, id);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_all_courses_with_semester_filter() {
        let conn = setup_db();

        let c1 = CreateCourseRequest {
            name: String::from("Course A"),
            day_of_week: 1,
            start_time: String::from("08:00"),
            end_time: String::from("09:00"),
            location: String::new(),
            teacher: String::new(),
            color: String::new(),
            semester: String::from("2024S1"),
        };
        let c2 = CreateCourseRequest {
            name: String::from("Course B"),
            day_of_week: 2,
            start_time: String::from("10:00"),
            end_time: String::from("11:00"),
            location: String::new(),
            teacher: String::new(),
            color: String::new(),
            semester: String::from("2024S2"),
        };

        create_course(&conn, &c1).expect("Failed to create c1");
        create_course(&conn, &c2).expect("Failed to create c2");

        let all = get_all_courses(&conn, None).expect("Failed to get all courses");
        assert_eq!(all.len(), 2);

        let s1 = get_all_courses(&conn, Some("2024S1"))
            .expect("Failed to filter by semester");
        assert_eq!(s1.len(), 1);
        assert_eq!(s1[0].name, "Course A");

        let s2 = get_all_courses(&conn, Some("2024S2"))
            .expect("Failed to filter by semester");
        assert_eq!(s2.len(), 1);
        assert_eq!(s2[0].name, "Course B");
    }

    #[test]
    fn test_get_all_courses_empty() {
        let conn = setup_db();
        let courses = get_all_courses(&conn, None).expect("Failed to get courses");
        assert!(courses.is_empty());
    }
}
