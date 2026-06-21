use std::collections::HashMap;

use chrono::{DateTime, Datelike, Duration, FixedOffset, NaiveDate};
use serde::Serialize;

use crate::db::models::{Course, Exam};

const EXAM_FALLBACK_COLOR: &str = "#DC2626";

#[derive(Debug, Clone, Serialize)]
pub struct WeekScheduleItem {
    pub kind: String,
    pub id: i64,
    pub title: String,
    pub day_of_week: i64,
    pub start_time: String,
    pub end_time: String,
    pub location: String,
    pub teacher: String,
    pub color: String,
    pub notes: String,
    pub week_pattern: String,
    pub course_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WeekScheduleResponse {
    pub week_index: i64,
    pub semester: String,
    pub semester_start_date: String,
    pub week_start_date: String,
    pub week_end_date: String,
    pub items: Vec<WeekScheduleItem>,
}

pub fn build_week_schedule(
    courses: &[Course],
    exams: &[Exam],
    semester: &str,
    week_index: i64,
    requested_semester_start_date: Option<&str>,
) -> Result<WeekScheduleResponse, String> {
    if week_index < 1 {
        return Err("week_index 必须大于等于 1".to_string());
    }

    let semester_courses: Vec<&Course> = courses
        .iter()
        .filter(|course| semester.is_empty() || course.semester == semester)
        .collect();
    let semester_exams: Vec<&Exam> = exams
        .iter()
        .filter(|exam| semester.is_empty() || exam.semester == semester)
        .collect();

    let anchor = resolve_semester_start_date(&semester_courses, requested_semester_start_date)?;
    let raw_start = anchor + Duration::days((week_index - 1) * 7);
    let days_from_monday = raw_start.weekday().num_days_from_monday() as i64;
    let week_start = raw_start - Duration::days(days_from_monday);
    let week_end = week_start + Duration::days(6);

    let course_colors: HashMap<i64, String> = courses
        .iter()
        .map(|course| (course.id, course.color.clone()))
        .collect();

    let mut items: Vec<WeekScheduleItem> = semester_courses
        .into_iter()
        .filter(|course| matches_week_pattern(&course.week_pattern, week_index))
        .map(|course| WeekScheduleItem {
            kind: "course".to_string(),
            id: course.id,
            title: course.name.clone(),
            day_of_week: course.day_of_week,
            start_time: course.start_time.clone(),
            end_time: course.end_time.clone(),
            location: course.location.clone(),
            teacher: course.teacher.clone(),
            color: course.color.clone(),
            notes: String::new(),
            week_pattern: course.week_pattern.clone(),
            course_id: Some(course.id),
        })
        .collect();

    let china_offset = FixedOffset::east_opt(8 * 3600).expect("china utc offset");
    for exam in semester_exams {
        let start = DateTime::parse_from_rfc3339(&exam.exam_datetime)
            .map_err(|_| format!("无法解析考试开始时间: {}", exam.exam_datetime))?
            .with_timezone(&china_offset);
        let end = if exam.exam_end_datetime.is_empty() {
            start
        } else {
            DateTime::parse_from_rfc3339(&exam.exam_end_datetime)
                .map_err(|_| format!("无法解析考试结束时间: {}", exam.exam_end_datetime))?
                .with_timezone(&china_offset)
        };

        let exam_date = start.date_naive();
        if exam_date < week_start || exam_date > week_end {
            continue;
        }

        let color = exam
            .course_id
            .and_then(|course_id| course_colors.get(&course_id))
            .cloned()
            .unwrap_or_else(|| EXAM_FALLBACK_COLOR.to_string());

        items.push(WeekScheduleItem {
            kind: "exam".to_string(),
            id: exam.id,
            title: exam.course_name.clone(),
            day_of_week: exam_date.weekday().number_from_monday() as i64,
            start_time: start.format("%H:%M").to_string(),
            end_time: end.format("%H:%M").to_string(),
            location: exam.location.clone(),
            teacher: String::new(),
            color,
            notes: exam.notes.clone(),
            week_pattern: String::new(),
            course_id: exam.course_id,
        });
    }

    items.sort_by(|left, right| {
        left.day_of_week
            .cmp(&right.day_of_week)
            .then(left.start_time.cmp(&right.start_time))
            .then(left.kind.cmp(&right.kind))
    });

    Ok(WeekScheduleResponse {
        week_index,
        semester: semester.to_string(),
        semester_start_date: anchor.format("%Y-%m-%d").to_string(),
        week_start_date: week_start.format("%Y-%m-%d").to_string(),
        week_end_date: week_end.format("%Y-%m-%d").to_string(),
        items,
    })
}

fn resolve_semester_start_date(
    courses: &[&Course],
    requested: Option<&str>,
) -> Result<NaiveDate, String> {
    let candidate = requested
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            courses
                .iter()
                .find(|course| !course.semester_start_date.trim().is_empty())
                .map(|course| course.semester_start_date.trim().to_string())
        })
        .ok_or_else(|| "缺少学期开始日期，无法计算周视图。".to_string())?;

    NaiveDate::parse_from_str(&candidate, "%Y-%m-%d")
        .map_err(|_| format!("无法解析学期开始日期: {candidate}"))
}

pub fn matches_week_pattern(week_pattern: &str, week_index: i64) -> bool {
    let normalized = week_pattern.replace(' ', "");
    if normalized.is_empty() {
        return true;
    }

    normalized
        .split([',', '，', ';', '；', '、', '/'])
        .filter(|segment| !segment.is_empty())
        .any(|segment| segment_matches(segment, week_index))
}

fn segment_matches(segment: &str, week_index: i64) -> bool {
    let (range_part, tail) = segment.split_once('周').unwrap_or((segment, ""));
    let range_part = range_part.trim_start_matches('第');

    let (start, end) = if let Some((start_raw, end_raw)) = range_part.split_once('-') {
        let start = start_raw.parse::<i64>().ok();
        let end = end_raw.parse::<i64>().ok();
        match (start, end) {
            (Some(start), Some(end)) => (start, end),
            _ => return false,
        }
    } else if let Ok(single_week) = range_part.parse::<i64>() {
        (single_week, single_week)
    } else {
        return false;
    };

    if week_index < start || week_index > end {
        return false;
    }

    if tail.contains('单') {
        return week_index % 2 == 1;
    }
    if tail.contains('双') {
        return week_index % 2 == 0;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_course() -> Course {
        Course {
            id: 1,
            name: "自动控制原理".to_string(),
            day_of_week: 3,
            start_time: "10:00".to_string(),
            end_time: "11:40".to_string(),
            week_pattern: "1-16周单周".to_string(),
            semester_start_date: "2026-02-24".to_string(),
            location: "秦岭堂A114".to_string(),
            teacher: "李红信".to_string(),
            color: "#3B82F6".to_string(),
            semester: "2026S1".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    fn sample_exam() -> Exam {
        Exam {
            id: 10,
            course_name: "自动控制原理".to_string(),
            exam_datetime: "2026-02-25T00:00:00Z".to_string(),
            exam_end_datetime: "2026-02-25T02:00:00Z".to_string(),
            location: "天山堂A409".to_string(),
            notes: "正常考试".to_string(),
            course_id: Some(1),
            semester: "2026S1".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn test_matches_week_pattern() {
        assert!(matches_week_pattern("1-16周全周", 8));
        assert!(matches_week_pattern("1-16周单周", 9));
        assert!(!matches_week_pattern("1-16周单周", 10));
        assert!(matches_week_pattern("2-18周双周", 10));
    }

    #[test]
    fn test_build_week_schedule() {
        let response = build_week_schedule(
            &[sample_course()],
            &[sample_exam()],
            "2026S1",
            1,
            Some("2026-02-24"),
        )
        .expect("build schedule");

        // 2026-02-24 is Tuesday, so Monday of week 1 is 2026-02-23
        assert_eq!(response.week_start_date, "2026-02-23");
        assert_eq!(response.items.len(), 2);
        assert_eq!(response.items[0].kind, "exam");
        assert_eq!(response.items[1].kind, "course");
    }
}
