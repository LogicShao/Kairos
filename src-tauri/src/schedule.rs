use std::collections::HashMap;

use chrono::{DateTime, Datelike, Duration, FixedOffset, NaiveDate};
use serde::Serialize;

use crate::db::models::{Course, Exam, Task};

/// 未绑定课程的考试使用固定警示色（Red-600），避免前端自行猜测考试默认颜色。
const EXAM_FALLBACK_COLOR: &str = "#DC2626";

/// 周课表聚合项，跨越后端课程/考试模型和前端课程周视图。
#[derive(Debug, Clone, Serialize)]
pub struct WeekScheduleItem {
    /// 聚合来源: "course" 或 "exam"。前端用它决定点击和展示语义。
    pub kind: String,
    pub id: i64,
    pub title: String,
    /// ISO weekday: 1 = 周一，7 = 周日。
    pub day_of_week: i64,
    /// 当天开始时间，格式 HH:mm。
    pub start_time: String,
    /// 当天结束时间，格式 HH:mm。
    pub end_time: String,
    pub location: String,
    pub teacher: String,
    /// 展示颜色。课程取自身颜色；考试优先取关联课程颜色，否则用 EXAM_FALLBACK_COLOR。
    pub color: String,
    pub notes: String,
    /// 课程周次规则；考试项为空字符串。
    pub week_pattern: String,
    /// 关联课程本地 id。课程项为自身 id，未绑定课程的考试为 None。
    pub course_id: Option<i64>,
}

/// 周课表响应，日期均为 YYYY-MM-DD。
#[derive(Debug, Clone, Serialize)]
pub struct WeekScheduleResponse {
    /// 请求的教学周序号，最小为 1。
    pub week_index: i64,
    /// 学期标识，例如 2026S1。
    pub semester: String,
    /// 学期锚点日期，用于从教学周推导自然周。
    pub semester_start_date: String,
    /// 当前周周一日期。
    pub week_start_date: String,
    /// 当前周周日日期。
    pub week_end_date: String,
    pub items: Vec<WeekScheduleItem>,
}

/// 日历视图聚合事件，跨越 course/exam/task 三种来源。
#[derive(Debug, Clone, Serialize)]
pub struct CalendarEvent {
    /// 聚合来源: "course"、"exam" 或 "task"。
    pub kind: String,
    pub id: i64,
    pub title: String,
    /// ISO weekday: 1 = 周一，7 = 周日。
    pub day_of_week: i64,
    /// 当天开始时间，格式 HH:mm。
    pub start_time: String,
    /// 当天结束时间，格式 HH:mm。
    pub end_time: String,
    pub location: String,
    /// 展示颜色由后端统一按来源决定，前端只消费。
    pub color: String,
    /// 展示标签，任务从 tags JSON 解析，考试包含"考试"和 notes。
    pub tags: Vec<String>,
    /// 前端点击后跳转目标: "todo"、"courses" 或 "exams"。
    pub source_link: String,
}

/// 日历周响应，支持按教学周或自然周定位。
#[derive(Debug, Clone, Serialize)]
pub struct CalendarWeekResponse {
    /// 实际日历周对应的教学周序号，可能由 week_start_date 反推。
    pub week_index: i64,
    /// 学期标识，例如 2026S1。
    pub semester: String,
    /// 学期锚点日期，格式 YYYY-MM-DD。
    pub semester_start_date: String,
    /// 当前周周一日期，格式 YYYY-MM-DD。
    pub week_start_date: String,
    /// 当前周周日日期，格式 YYYY-MM-DD。
    pub week_end_date: String,
    pub events: Vec<CalendarEvent>,
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

/// 未完成任务在日历视图中的固定颜色（Amber-600），语义为"待处理/需关注"。
const TASK_COLOR: &str = "#D97706";
/// 已完成任务在日历视图中的固定颜色（Teal-500），语义为"已完成"。
/// 区别于未完成任务但不改变数据库 task 状态，仅影响日历渲染。
const TASK_COMPLETED_COLOR: &str = "#14B8A6";

pub fn build_calendar_week(
    courses: &[Course],
    exams: &[Exam],
    tasks: &[Task],
    semester: &str,
    week_index: i64,
    requested_semester_start_date: Option<&str>,
    requested_week_start_date: Option<&str>,
) -> Result<CalendarWeekResponse, String> {
    if week_index < 1 {
        return Err("week_index 必须大于等于 1".to_string());
    }

    let semester_courses: Vec<&Course> = courses
        .iter()
        .filter(|course| semester.is_empty() || course.semester == semester)
        .collect();
    let anchor = resolve_calendar_start_date(
        &semester_courses,
        requested_semester_start_date,
        requested_week_start_date,
    )?;
    let week_start = match requested_week_start_date {
        Some(value) if !value.trim().is_empty() => normalize_week_start(value)?,
        _ => {
            let raw_start = anchor + Duration::days((week_index - 1) * 7);
            raw_start - Duration::days(raw_start.weekday().num_days_from_monday() as i64)
        }
    };
    let week_end = week_start + Duration::days(6);
    let effective_week_index = compute_week_index(anchor, week_start);

    let course_colors: HashMap<i64, String> = courses
        .iter()
        .map(|course| (course.id, course.color.clone()))
        .collect();

    let mut events: Vec<CalendarEvent> = semester_courses
        .into_iter()
        .filter(|course| {
            effective_week_index >= 1
                && matches_week_pattern(&course.week_pattern, effective_week_index)
        })
        .map(|course| CalendarEvent {
            kind: "course".to_string(),
            id: course.id,
            title: course.name.clone(),
            day_of_week: course.day_of_week,
            start_time: course.start_time.clone(),
            end_time: course.end_time.clone(),
            location: course.location.clone(),
            color: course.color.clone(),
            tags: Vec::new(),
            source_link: "courses".to_string(),
        })
        .collect();

    let china_offset = FixedOffset::east_opt(8 * 3600).expect("china utc offset");
    for exam in exams {
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

        let mut tags = vec!["考试".to_string()];
        if !exam.notes.is_empty() {
            tags.push(exam.notes.clone());
        }

        events.push(CalendarEvent {
            kind: "exam".to_string(),
            id: exam.id,
            title: exam.course_name.clone(),
            day_of_week: exam_date.weekday().number_from_monday() as i64,
            start_time: start.format("%H:%M").to_string(),
            end_time: end.format("%H:%M").to_string(),
            location: exam.location.clone(),
            color,
            tags,
            source_link: "exams".to_string(),
        });
    }

    for task in tasks {
        let due_date_str = match &task.due_date {
            Some(d) if !d.trim().is_empty() => d.trim(),
            _ => continue,
        };

        let due_date = match NaiveDate::parse_from_str(due_date_str, "%Y-%m-%d") {
            Ok(d) => d,
            Err(_) => continue,
        };

        if due_date < week_start || due_date > week_end {
            continue;
        }

        let is_done = task.status == "done";
        let color = if is_done {
            TASK_COMPLETED_COLOR.to_string()
        } else {
            TASK_COLOR.to_string()
        };

        let mut tags: Vec<String> = Vec::new();
        if is_done {
            tags.push("完成".to_string());
        }

        let today = chrono::Utc::now().with_timezone(&china_offset).date_naive();
        if !is_done && due_date == today {
            tags.push("截止".to_string());
        }

        if !task.tags.is_empty() && task.tags != "[]" {
            if let Ok(parsed) = serde_json::from_str::<Vec<String>>(&task.tags) {
                for t in parsed {
                    if !t.is_empty() {
                        tags.push(t);
                    }
                }
            }
        }

        events.push(CalendarEvent {
            kind: "task".to_string(),
            id: task.id,
            title: task.title.clone(),
            day_of_week: due_date.weekday().number_from_monday() as i64,
            start_time: "00:00".to_string(),
            end_time: "00:00".to_string(),
            location: String::new(),
            color,
            tags,
            source_link: "todo".to_string(),
        });
    }

    events.sort_by(|left, right| {
        left.day_of_week
            .cmp(&right.day_of_week)
            .then(left.start_time.cmp(&right.start_time))
            .then(left.kind.cmp(&right.kind))
    });

    Ok(CalendarWeekResponse {
        week_index: effective_week_index,
        semester: semester.to_string(),
        semester_start_date: anchor.format("%Y-%m-%d").to_string(),
        week_start_date: week_start.format("%Y-%m-%d").to_string(),
        week_end_date: week_end.format("%Y-%m-%d").to_string(),
        events,
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

fn resolve_calendar_start_date(
    courses: &[&Course],
    requested: Option<&str>,
    requested_week_start_date: Option<&str>,
) -> Result<NaiveDate, String> {
    let has_requested = requested
        .map(str::trim)
        .is_some_and(|value| !value.is_empty());

    match resolve_semester_start_date(courses, requested) {
        Ok(date) => Ok(date),
        Err(err)
            if has_requested
                || courses
                    .iter()
                    .any(|course| !course.semester_start_date.trim().is_empty()) =>
        {
            Err(err)
        }
        Err(_) => {
            if let Some(value) = requested_week_start_date {
                if !value.trim().is_empty() {
                    return normalize_week_start(value);
                }
            }

            let china_offset = FixedOffset::east_opt(8 * 3600).expect("china utc offset");
            let today = chrono::Utc::now().with_timezone(&china_offset).date_naive();
            Ok(today - Duration::days(today.weekday().num_days_from_monday() as i64))
        }
    }
}

fn normalize_week_start(value: &str) -> Result<NaiveDate, String> {
    let date = NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d")
        .map_err(|_| format!("无法解析周起始日期: {}", value.trim()))?;
    Ok(date - Duration::days(date.weekday().num_days_from_monday() as i64))
}

fn compute_week_index(anchor: NaiveDate, week_start: NaiveDate) -> i64 {
    let anchor_week_start = anchor - Duration::days(anchor.weekday().num_days_from_monday() as i64);
    let diff_days = week_start
        .signed_duration_since(anchor_week_start)
        .num_days();
    diff_days.div_euclid(7) + 1
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
            sync_id: "course-sync-1".to_string(),
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
            deleted_at: None,
        }
    }

    fn sample_exam() -> Exam {
        Exam {
            id: 10,
            sync_id: "exam-sync-10".to_string(),
            course_name: "自动控制原理".to_string(),
            exam_datetime: "2026-02-25T00:00:00Z".to_string(),
            exam_end_datetime: "2026-02-25T02:00:00Z".to_string(),
            location: "天山堂A409".to_string(),
            notes: "正常考试".to_string(),
            course_id: Some(1),
            semester: "2026S1".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
            deleted_at: None,
        }
    }

    fn sample_task(id: i64, title: &str, due_date: &str, status: &str) -> Task {
        Task {
            id,
            sync_id: format!("task-sync-{id}"),
            title: title.to_string(),
            description: String::new(),
            status: status.to_string(),
            priority: "medium".to_string(),
            due_date: Some(due_date.to_string()),
            tags: "[]".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
            deleted_at: None,
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

    #[test]
    fn test_build_calendar_week_course_and_exam() {
        let response = build_calendar_week(
            &[sample_course()],
            &[sample_exam()],
            &[],
            "2026S1",
            1,
            Some("2026-02-24"),
            None,
        )
        .expect("build calendar week");

        assert_eq!(response.week_start_date, "2026-02-23");
        assert_eq!(response.events.len(), 2);
        assert_eq!(response.events[0].kind, "exam");
        assert_eq!(response.events[1].kind, "course");
    }

    #[test]
    fn test_build_calendar_week_includes_exam_by_date_even_when_semester_differs() {
        let mut exam = sample_exam();
        exam.semester = String::new();

        let response = build_calendar_week(
            &[sample_course()],
            &[exam],
            &[],
            "2026S1",
            1,
            Some("2026-02-24"),
            None,
        )
        .expect("build calendar week");

        let exam_events: Vec<_> = response
            .events
            .iter()
            .filter(|event| event.kind == "exam")
            .collect();
        assert_eq!(exam_events.len(), 1);
        assert_eq!(exam_events[0].title, "自动控制原理");
    }

    #[test]
    fn test_build_calendar_week_with_tasks() {
        // 2026-02-24 is Tuesday, Monday of week 1 is 2026-02-23
        // due_date 2026-02-24 (Tue) falls within the week
        let response = build_calendar_week(
            &[sample_course()],
            &[],
            &[sample_task(100, "提交作业", "2026-02-24", "todo")],
            "2026S1",
            1,
            Some("2026-02-24"),
            None,
        )
        .expect("build calendar week");

        assert_eq!(response.week_start_date, "2026-02-23");
        let task_events: Vec<_> = response
            .events
            .iter()
            .filter(|e| e.kind == "task")
            .collect();
        assert_eq!(task_events.len(), 1);
        assert_eq!(task_events[0].title, "提交作业");
        assert_eq!(task_events[0].color, TASK_COLOR);
        assert_eq!(task_events[0].source_link, "todo");
    }

    #[test]
    fn test_build_calendar_week_completed_task() {
        let response = build_calendar_week(
            &[],
            &[],
            &[sample_task(200, "已完成作业", "2026-02-25", "done")],
            "2026S1",
            1,
            Some("2026-02-24"),
            None,
        )
        .expect("build calendar week");

        let task = response.events.iter().find(|e| e.kind == "task").unwrap();
        assert_eq!(task.color, TASK_COMPLETED_COLOR);
        assert!(task.tags.contains(&"完成".to_string()));
    }

    #[test]
    fn test_build_calendar_week_task_outside_week() {
        // 2026-03-10 is outside week 1 (week 1 Monday is 2026-02-23)
        let response = build_calendar_week(
            &[],
            &[],
            &[sample_task(300, "远期任务", "2026-03-10", "todo")],
            "2026S1",
            1,
            Some("2026-02-24"),
            None,
        )
        .expect("build calendar week");

        let task_events: Vec<_> = response
            .events
            .iter()
            .filter(|e| e.kind == "task")
            .collect();
        assert!(task_events.is_empty());
    }

    #[test]
    fn test_build_calendar_week_task_no_due_date() {
        let task_no_date = Task {
            id: 400,
            sync_id: "task-sync-400".to_string(),
            title: "无截止日期".to_string(),
            description: String::new(),
            status: "todo".to_string(),
            priority: "medium".to_string(),
            due_date: None,
            tags: "[]".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
            deleted_at: None,
        };

        let response = build_calendar_week(
            &[],
            &[],
            &[task_no_date],
            "2026S1",
            1,
            Some("2026-02-24"),
            None,
        )
        .expect("build calendar week");

        let task_events: Vec<_> = response
            .events
            .iter()
            .filter(|e| e.kind == "task")
            .collect();
        assert!(task_events.is_empty());
    }

    #[test]
    fn test_build_calendar_week_without_semester_anchor_uses_current_week() {
        let china_offset = FixedOffset::east_opt(8 * 3600).expect("china utc offset");
        let today = chrono::Utc::now().with_timezone(&china_offset).date_naive();
        let monday = today - Duration::days(today.weekday().num_days_from_monday() as i64);
        let due_date = monday.format("%Y-%m-%d").to_string();

        let response = build_calendar_week(
            &[],
            &[],
            &[sample_task(500, "本周待办", &due_date, "todo")],
            "",
            1,
            None,
            None,
        )
        .expect("build calendar week");

        assert_eq!(response.week_start_date, due_date);
        let task_events: Vec<_> = response
            .events
            .iter()
            .filter(|e| e.kind == "task")
            .collect();
        assert_eq!(task_events.len(), 1);
    }

    #[test]
    fn test_build_calendar_week_vacation_week_uses_real_dates_without_repeating_courses() {
        let response = build_calendar_week(
            &[sample_course()],
            &[],
            &[sample_task(600, "暑假读书计划", "2026-07-20", "todo")],
            "2026S1",
            1,
            Some("2026-02-24"),
            Some("2026-07-20"),
        )
        .expect("build calendar week");

        assert_eq!(response.week_start_date, "2026-07-20");
        assert!(response.week_index > 16);

        let course_events: Vec<_> = response
            .events
            .iter()
            .filter(|e| e.kind == "course")
            .collect();
        let task_events: Vec<_> = response
            .events
            .iter()
            .filter(|e| e.kind == "task")
            .collect();
        assert!(course_events.is_empty());
        assert_eq!(task_events.len(), 1);
        assert_eq!(task_events[0].title, "暑假读书计划");
    }
}
