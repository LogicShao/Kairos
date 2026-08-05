use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use chrono::{Datelike, FixedOffset, NaiveDate, NaiveTime};
use rusqlite::Connection;
use serde::Serialize;
use tauri::State;

use crate::db::models::{Course, Exam, SemesterContext, Task};
use crate::db::semester::LZU_SEMESTER_CONTEXT_SOURCE;
use crate::schedule::{current_week_index_from_start, matches_week_pattern};
use crate::term_phase::CurrentPhaseStatus;
use crate::timer::PomodoroEngine;

// ─── Sub-structs ───────────────────────────────────────────────────────────────

/// 下一节课信息。
#[derive(Debug, Clone, Serialize)]
pub struct NextCourse {
    pub title: String,
    pub start_time: String,
    pub end_time: String,
    pub location: String,
}

/// 今日课程摘要。
#[derive(Debug, Clone, Serialize)]
pub struct TodayCourses {
    pub today_count: i64,
    pub current_course: Option<NextCourse>,
    pub next_course: Option<NextCourse>,
}

/// 待办 spotlight 条目。
#[derive(Debug, Clone, Serialize)]
pub struct TaskSpotlight {
    pub id: i64,
    pub title: String,
    pub priority: String,
    pub due_date: Option<String>,
}

/// 今日待办摘要。
#[derive(Debug, Clone, Serialize)]
pub struct TodayTasks {
    pub overdue_count: i64,
    pub due_today_count: i64,
    pub spotlight: Vec<TaskSpotlight>,
    /// 今日未完成的每日任务数（习惯类任务，每天需完成）。
    pub daily_unfinished_count: i64,
    /// 今日未完成的每日任务（按优先级/标题排序，最多 3 条用于展示）。
    pub daily_spotlight: Vec<TaskSpotlight>,
}

/// 最近一场考试。
#[derive(Debug, Clone, Serialize)]
pub struct UpcomingExam {
    pub id: i64,
    pub course_name: String,
    pub exam_datetime: String,
    pub days_until: i64,
    pub location: String,
}

/// 番茄钟当前状态摘要。
#[derive(Debug, Clone, Serialize)]
pub struct PomodoroBriefing {
    pub is_running: bool,
    pub phase: String,
    pub remaining_seconds: i64,
    pub completed_sessions: i64,
}

/// 今日所处学期阶段摘要。
#[derive(Debug, Clone, Serialize)]
pub struct PhaseBriefing {
    pub phase_type: String,
    pub term_label: Option<String>,
    pub current_week: Option<i64>,
    pub courses_visible: bool,
    pub exam_notifications_enabled: bool,
    pub pomodoro_profile: String,
}

/// 今日概览聚合响应，供前端首页卡片渲染及后续 AI Morning Brief 复用。
#[derive(Debug, Clone, Serialize)]
pub struct TodayBriefingResponse {
    /// 今日日期，YYYY-MM-DD。
    pub date: String,
    /// 今日星期中文标签，如"周一"。
    pub weekday_label: String,
    /// 今日课程摘要。
    pub courses: TodayCourses,
    /// 今日待办摘要。
    pub tasks: TodayTasks,
    /// 最近一场考试；None 表示没有未来考试。
    pub exam: Option<UpcomingExam>,
    /// 番茄钟当前状态摘要。
    pub pomodoro: PomodoroBriefing,
    /// 当前学期阶段摘要。
    pub phase: PhaseBriefing,
}

// ─── Tauri command ──────────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_today_briefing(
    db: State<'_, Arc<Mutex<Connection>>>,
    engine: State<'_, Arc<Mutex<PomodoroEngine>>>,
) -> Result<TodayBriefingResponse, String> {
    let conn = db.lock().map_err(|e| e.to_string())?;
    let engine = engine.lock().map_err(|e| e.to_string())?;
    collect_today_briefing(&conn, &engine)
}

/// 聚合今日概览数据快照，供 `get_today_briefing` 命令与 AI Morning Brief 复用。
///
/// 调用方负责持有 DB 与 PomodoroEngine 锁；本函数不做网络等长阻塞操作。
pub(crate) fn collect_today_briefing(
    conn: &Connection,
    engine: &PomodoroEngine,
) -> Result<TodayBriefingResponse, String> {
    let china_offset = FixedOffset::east_opt(8 * 3600).expect("china utc offset");
    let now = chrono::Utc::now().with_timezone(&china_offset);
    let today = now.date_naive();

    let date = today.format("%Y-%m-%d").to_string();
    let weekday_label = weekday_label(today.weekday());
    let phase_status = crate::term_phase::latest_lzu_phase_status(conn)?;
    let phase = phase_briefing(&phase_status);

    // 1. Courses
    let all_courses = crate::db::courses::get_all_courses(conn, None).map_err(|e| e.to_string())?;
    let semester_contexts =
        crate::db::semester::get_all_semester_contexts(conn).map_err(|e| e.to_string())?;
    let courses = build_today_courses(
        &all_courses,
        &semester_contexts,
        today,
        &now.time(),
        &phase_status,
    );

    // 2. Tasks
    let all_tasks = crate::db::tasks::get_all_tasks(conn, None, None, "created_at", "DESC")
        .map_err(|e| e.to_string())?;
    let tasks = build_today_tasks(&all_tasks, today);

    // 3. Exam
    let all_exams = crate::db::exams::get_all_exams(conn).map_err(|e| e.to_string())?;
    let exam = build_upcoming_exam(&all_exams, &now);

    // 4. Pomodoro — 使用 DB 今日完成数以保证与番茄钟页一致
    let completed_sessions = crate::db::pomodoro::count_completed_work_sessions_for_date(conn)
        .map_err(|e| e.to_string())?;
    let state = engine.get_state();
    let pomodoro = PomodoroBriefing {
        is_running: state.is_running,
        phase: state.phase,
        remaining_seconds: state.remaining_seconds as i64,
        completed_sessions,
    };

    Ok(TodayBriefingResponse {
        date,
        weekday_label,
        courses,
        tasks,
        exam,
        pomodoro,
        phase,
    })
}

// ─── Rule engine helpers ────────────────────────────────────────────────────────

fn weekday_label(weekday: chrono::Weekday) -> String {
    match weekday {
        chrono::Weekday::Mon => "周一",
        chrono::Weekday::Tue => "周二",
        chrono::Weekday::Wed => "周三",
        chrono::Weekday::Thu => "周四",
        chrono::Weekday::Fri => "周五",
        chrono::Weekday::Sat => "周六",
        chrono::Weekday::Sun => "周日",
    }
    .to_string()
}

/// 规则引擎：今日课程数 + 下一节未开始课程。
fn build_today_courses(
    courses: &[Course],
    semester_contexts: &[SemesterContext],
    today: NaiveDate,
    now_time: &NaiveTime,
    phase_status: &CurrentPhaseStatus,
) -> TodayCourses {
    // 无学期上下文（unknown）或非教学阶段时不把课程计入今日概览：
    // 避免假期仍显示旧学期遗留课程（见 07-31 假期误报排查）。
    if !phase_status.courses_visible || phase_status.phase_type == crate::term_phase::PHASE_UNKNOWN
    {
        return TodayCourses {
            today_count: 0,
            current_course: None,
            next_course: None,
        };
    }

    // ISO weekday: 1 = 周一，7 = 周日。
    let today_weekday = today.weekday().num_days_from_monday() as i64 + 1;
    let context_start_dates = semester_context_start_dates(semester_contexts);

    let today_courses: Vec<&Course> = courses
        .iter()
        .filter(|c| {
            if c.day_of_week != today_weekday {
                return false;
            }
            let week_index = match course_week_index(c, &context_start_dates, today) {
                Ok(idx) => idx,
                Err(_) => return false,
            };
            matches_week_pattern(&c.week_pattern, week_index)
        })
        .collect();

    let today_count = today_courses.len() as i64;

    let current_course = today_courses
        .iter()
        .filter(|c| {
            let Some(start) = NaiveTime::parse_from_str(&c.start_time, "%H:%M").ok() else {
                return false;
            };
            let Some(end) = NaiveTime::parse_from_str(&c.end_time, "%H:%M").ok() else {
                return false;
            };
            start <= *now_time && *now_time < end
        })
        .min_by_key(|c| NaiveTime::parse_from_str(&c.start_time, "%H:%M").ok())
        .map(course_brief);

    // 下一节课：在今天课程中找 start_time > now 的最早一节
    let next_course = today_courses
        .iter()
        .filter(|c| {
            NaiveTime::parse_from_str(&c.start_time, "%H:%M")
                .ok()
                .is_some_and(|t| t > *now_time)
        })
        .min_by_key(|c| NaiveTime::parse_from_str(&c.start_time, "%H:%M").ok())
        .map(course_brief);

    TodayCourses {
        today_count,
        current_course,
        next_course,
    }
}

fn phase_briefing(status: &CurrentPhaseStatus) -> PhaseBriefing {
    PhaseBriefing {
        phase_type: status.phase_type.clone(),
        term_label: status.term_label.clone(),
        current_week: status.current_week,
        courses_visible: status.courses_visible,
        exam_notifications_enabled: status.exam_notifications_enabled,
        pomodoro_profile: status.pomodoro_profile.clone(),
    }
}

fn semester_context_start_dates(contexts: &[SemesterContext]) -> HashMap<String, String> {
    contexts
        .iter()
        .filter(|context| context.source == LZU_SEMESTER_CONTEXT_SOURCE)
        .filter(|context| !context.start_date.trim().is_empty())
        .map(|context| (context.term_label.clone(), context.start_date.clone()))
        .collect()
}

fn course_week_index(
    course: &Course,
    context_start_dates: &HashMap<String, String>,
    today: NaiveDate,
) -> Result<i64, String> {
    if let Some(start_date) = context_start_dates.get(&course.semester) {
        if let Ok(week_index) = current_week_index_from_start(start_date, today) {
            return Ok(week_index);
        }
    }

    current_week_index_from_start(&course.semester_start_date, today)
}

fn course_brief(course: &&Course) -> NextCourse {
    NextCourse {
        title: course.name.clone(),
        start_time: course.start_time.clone(),
        end_time: course.end_time.clone(),
        location: course.location.clone(),
    }
}

fn priority_score(priority: &str) -> i64 {
    match priority {
        "high" => 3,
        "medium" => 2,
        "low" => 1,
        _ => 0,
    }
}

fn task_due_rank(task: &Task, today: NaiveDate) -> i64 {
    let Some(due_date) = task
        .due_date
        .as_deref()
        .and_then(|date| NaiveDate::parse_from_str(date.trim(), "%Y-%m-%d").ok())
    else {
        return 0;
    };

    if due_date < today {
        2
    } else if due_date == today {
        1
    } else {
        0
    }
}

/// 规则引擎：逾期数 + 今日到期数 + spotlight（高优先级优先、有截止日期优先）。
fn build_today_tasks(tasks: &[Task], today: NaiveDate) -> TodayTasks {
    let mut overdue_count = 0i64;
    let mut due_today_count = 0i64;
    let mut spotlight_candidates: Vec<&Task> = Vec::new();

    for task in tasks {
        if task.status == "done" || task.deleted_at.is_some() {
            continue;
        }

        let due = match &task.due_date {
            Some(d) if !d.trim().is_empty() => NaiveDate::parse_from_str(d.trim(), "%Y-%m-%d").ok(),
            _ => None,
        };

        match due {
            Some(d) if d < today => {
                overdue_count += 1;
                spotlight_candidates.push(task);
            }
            Some(d) if d == today => {
                due_today_count += 1;
                spotlight_candidates.push(task);
            }
            _ => {}
        }
    }

    // Spotlight 排序：逾期优先，然后高优先级优先，再按较早截止日期优先。
    spotlight_candidates.sort_by(|a, b| {
        let a_due_rank = task_due_rank(a, today);
        let b_due_rank = task_due_rank(b, today);
        let a_score = priority_score(&a.priority);
        let b_score = priority_score(&b.priority);
        b_due_rank
            .cmp(&a_due_rank)
            .then_with(|| b_score.cmp(&a_score))
            .then_with(|| {
                let a_due = a.due_date.as_deref().unwrap_or("9999-99-99");
                let b_due = b.due_date.as_deref().unwrap_or("9999-99-99");
                a_due.cmp(b_due)
            })
    });

    let spotlight: Vec<TaskSpotlight> = spotlight_candidates
        .into_iter()
        .take(3)
        .map(|t| TaskSpotlight {
            id: t.id,
            title: t.title.clone(),
            priority: t.priority.clone(),
            due_date: t.due_date.clone(),
        })
        .collect();

    // 每日任务：今日未完成（last_completed_date != today）的项，按优先级/标题排序。
    let today_str = today.format("%Y-%m-%d").to_string();
    let mut daily: Vec<&Task> = tasks
        .iter()
        .filter(|t| {
            t.is_daily
                && t.deleted_at.is_none()
                && t.last_completed_date.as_deref() != Some(today_str.as_str())
        })
        .collect();
    daily.sort_by(|a, b| {
        let a_score = priority_score(&a.priority);
        let b_score = priority_score(&b.priority);
        b_score.cmp(&a_score).then_with(|| a.title.cmp(&b.title))
    });
    let daily_spotlight: Vec<TaskSpotlight> = daily
        .iter()
        .take(3)
        .map(|t| TaskSpotlight {
            id: t.id,
            title: t.title.clone(),
            priority: t.priority.clone(),
            due_date: None,
        })
        .collect();
    let daily_unfinished_count = daily.len() as i64;

    TodayTasks {
        overdue_count,
        due_today_count,
        spotlight,
        daily_unfinished_count,
        daily_spotlight,
    }
}

/// 规则引擎：最近一场未开始的考试。
fn build_upcoming_exam(
    exams: &[Exam],
    now: &chrono::DateTime<FixedOffset>,
) -> Option<UpcomingExam> {
    let mut upcoming: Vec<&Exam> = exams
        .iter()
        .filter(|exam| {
            chrono::DateTime::parse_from_rfc3339(&exam.exam_datetime)
                .ok()
                .is_some_and(|start| start > *now)
        })
        .collect();

    upcoming.sort_by_key(|exam| chrono::DateTime::parse_from_rfc3339(&exam.exam_datetime).ok());

    upcoming.into_iter().next().map(|exam| {
        let start =
            chrono::DateTime::parse_from_rfc3339(&exam.exam_datetime).expect("already validated");
        let exam_date = start.date_naive();
        let days_until = (exam_date - now.date_naive()).num_days();

        UpcomingExam {
            id: exam.id,
            course_name: exam.course_name.clone(),
            exam_datetime: exam.exam_datetime.clone(),
            days_until,
            location: exam.location.clone(),
        }
    })
}

// ─── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn sample_task(id: i64, title: &str, due_date: &str, status: &str, priority: &str) -> Task {
        Task {
            id,
            sync_id: format!("task-sync-{id}"),
            title: title.to_string(),
            description: String::new(),
            status: status.to_string(),
            priority: priority.to_string(),
            due_date: Some(due_date.to_string()),
            tags: "[]".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
            is_daily: false,
            last_completed_date: None,
            reminder_time: None,
            remind_at: None,
            deleted_at: None,
        }
    }

    fn task_no_date(id: i64, title: &str) -> Task {
        Task {
            id,
            sync_id: format!("task-sync-{id}"),
            title: title.to_string(),
            description: String::new(),
            status: "todo".to_string(),
            priority: "medium".to_string(),
            due_date: None,
            tags: "[]".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
            is_daily: false,
            last_completed_date: None,
            reminder_time: None,
            remind_at: None,
            deleted_at: None,
        }
    }

    fn sample_course(
        id: i64,
        name: &str,
        day_of_week: i64,
        start_time: &str,
        end_time: &str,
        semester_start_date: &str,
    ) -> Course {
        Course {
            id,
            sync_id: format!("course-{id}"),
            name: name.to_string(),
            day_of_week,
            start_time: start_time.to_string(),
            end_time: end_time.to_string(),
            week_pattern: "1-17周全周".to_string(),
            semester_start_date: semester_start_date.to_string(),
            location: "教室A".to_string(),
            teacher: "李老师".to_string(),
            color: "#3B82F6".to_string(),
            semester: "2026S1".to_string(),
            created_at: String::new(),
            updated_at: String::new(),
            deleted_at: None,
        }
    }

    fn sample_context(term_label: &str, start_date: &str) -> SemesterContext {
        SemesterContext {
            id: 1,
            source: LZU_SEMESTER_CONTEXT_SOURCE.to_string(),
            academic_year: Some("2026".to_string()),
            term: Some("1".to_string()),
            term_label: term_label.to_string(),
            start_date: start_date.to_string(),
            current_week: Some(1),
            total_weeks: None,
            refreshed_at: String::new(),
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    fn teaching_phase() -> CurrentPhaseStatus {
        CurrentPhaseStatus {
            source: LZU_SEMESTER_CONTEXT_SOURCE.to_string(),
            term_label: Some("2026S1".to_string()),
            phase_type: "teaching".to_string(),
            current_week: Some(17),
            start_week: None,
            end_week: None,
            courses_visible: true,
            exam_notifications_enabled: true,
            pomodoro_profile: "default".to_string(),
            inferred: true,
        }
    }

    #[test]
    fn test_weekday_label() {
        assert_eq!(weekday_label(chrono::Weekday::Mon), "周一");
        assert_eq!(weekday_label(chrono::Weekday::Sun), "周日");
    }

    #[test]
    fn test_build_today_tasks_counts() {
        let today = NaiveDate::from_ymd_opt(2026, 6, 28).unwrap();
        let tasks = vec![
            sample_task(1, "逾期任务", "2026-06-27", "todo", "high"),
            sample_task(2, "今日到期", "2026-06-28", "todo", "medium"),
            sample_task(3, "已完成", "2026-06-28", "done", "low"),
            task_no_date(4, "无截止日期"),
        ];
        let result = build_today_tasks(&tasks, today);
        assert_eq!(result.overdue_count, 1);
        assert_eq!(result.due_today_count, 1);
        assert_eq!(result.spotlight.len(), 2);
    }

    #[test]
    fn test_build_today_tasks_spotlight_ordering() {
        let today = NaiveDate::from_ymd_opt(2026, 6, 28).unwrap();
        let tasks = vec![
            sample_task(1, "低优先级逾期", "2026-06-27", "todo", "low"),
            sample_task(2, "高优先级今日", "2026-06-28", "todo", "high"),
            sample_task(3, "中优先级逾期", "2026-06-26", "todo", "medium"),
        ];
        let result = build_today_tasks(&tasks, today);
        assert_eq!(result.spotlight.len(), 3);
        assert_eq!(result.spotlight[0].title, "中优先级逾期");
        assert_eq!(result.spotlight[1].title, "低优先级逾期");
        assert_eq!(result.spotlight[2].title, "高优先级今日");
    }

    #[test]
    fn test_build_today_tasks_spotlight_capped_at_three() {
        let today = NaiveDate::from_ymd_opt(2026, 6, 28).unwrap();
        let tasks: Vec<Task> = (1..=5)
            .map(|i| sample_task(i as i64, &format!("任务{i}"), "2026-06-27", "todo", "low"))
            .collect();
        let result = build_today_tasks(&tasks, today);
        assert_eq!(result.overdue_count, 5);
        assert_eq!(result.spotlight.len(), 3);
    }

    #[test]
    fn test_build_upcoming_exam() {
        let _china_offset = FixedOffset::east_opt(8 * 3600).unwrap();
        let now = chrono::DateTime::parse_from_rfc3339("2026-06-28T10:00:00+08:00").unwrap();
        let exams = vec![
            Exam {
                id: 1,
                sync_id: "exam-1".to_string(),
                course_name: "高数期末".to_string(),
                exam_datetime: "2026-07-01T08:00:00+08:00".to_string(),
                exam_end_datetime: "2026-07-01T10:00:00+08:00".to_string(),
                location: "天山堂A409".to_string(),
                notes: String::new(),
                course_id: None,
                semester: "2026S1".to_string(),
                created_at: String::new(),
                updated_at: String::new(),
                deleted_at: None,
            },
            Exam {
                id: 2,
                sync_id: "exam-2".to_string(),
                course_name: "已过去考试".to_string(),
                exam_datetime: "2026-06-27T08:00:00+08:00".to_string(),
                exam_end_datetime: "2026-06-27T10:00:00+08:00".to_string(),
                location: "教室A".to_string(),
                notes: String::new(),
                course_id: None,
                semester: "2026S1".to_string(),
                created_at: String::new(),
                updated_at: String::new(),
                deleted_at: None,
            },
        ];
        let result = build_upcoming_exam(&exams, &now);
        assert!(result.is_some());
        let exam = result.unwrap();
        assert_eq!(exam.course_name, "高数期末");
        assert_eq!(exam.days_until, 3);
    }

    #[test]
    fn test_build_upcoming_exam_none_when_all_past() {
        let _china_offset = FixedOffset::east_opt(8 * 3600).unwrap();
        let now = chrono::DateTime::parse_from_rfc3339("2026-07-01T12:00:00+08:00").unwrap();
        let exams = vec![Exam {
            id: 1,
            sync_id: "exam-1".to_string(),
            course_name: "已过去考试".to_string(),
            exam_datetime: "2026-06-27T08:00:00+08:00".to_string(),
            exam_end_datetime: "2026-06-27T10:00:00+08:00".to_string(),
            location: "教室A".to_string(),
            notes: String::new(),
            course_id: None,
            semester: "2026S1".to_string(),
            created_at: String::new(),
            updated_at: String::new(),
            deleted_at: None,
        }];
        let result = build_upcoming_exam(&exams, &now);
        assert!(result.is_none());
    }

    #[test]
    fn test_build_today_courses_no_matching_day() {
        // 2026-06-28 is Sunday (ISO weekday 7)
        let today = NaiveDate::from_ymd_opt(2026, 6, 28).unwrap();
        let now_time = NaiveTime::from_hms_opt(10, 0, 0).unwrap();
        let courses = vec![sample_course(
            1,
            "周一课程",
            1,
            "10:00",
            "11:40",
            "2026-02-24",
        )];
        let result = build_today_courses(&courses, &[], today, &now_time, &teaching_phase());
        assert_eq!(result.today_count, 0);
        assert!(result.current_course.is_none());
        assert!(result.next_course.is_none());
    }

    #[test]
    fn test_build_today_courses_next_course() {
        // 2026-06-19 is Friday of week 17 (within 1-17周全周)
        let today = NaiveDate::from_ymd_opt(2026, 6, 19).unwrap();
        let now_time = NaiveTime::from_hms_opt(9, 0, 0).unwrap(); // 09:00
        let courses = vec![
            sample_course(1, "上午课程", 5, "08:00", "09:40", "2026-02-24"),
            sample_course(2, "下午课程", 5, "14:00", "15:40", "2026-02-24"),
        ];
        let result = build_today_courses(&courses, &[], today, &now_time, &teaching_phase());
        assert_eq!(result.today_count, 2);
        assert!(result.current_course.is_some());
        assert_eq!(result.current_course.as_ref().unwrap().title, "上午课程");
        // Next course should be the afternoon one (starts at 14:00, which is > 09:00)
        assert!(result.next_course.is_some());
        assert_eq!(result.next_course.as_ref().unwrap().title, "下午课程");
    }

    #[test]
    fn test_build_today_courses_uses_semester_context_start_date() {
        let today = NaiveDate::from_ymd_opt(2026, 6, 19).unwrap();
        let now_time = NaiveTime::from_hms_opt(7, 0, 0).unwrap();
        let courses = vec![sample_course(
            1,
            "上下文校准课程",
            5,
            "08:00",
            "09:40",
            "2026-02-03",
        )];
        let contexts = vec![sample_context("2026S1", "2026-02-24")];

        let result = build_today_courses(&courses, &contexts, today, &now_time, &teaching_phase());

        assert_eq!(result.today_count, 1);
        assert_eq!(
            result
                .next_course
                .as_ref()
                .map(|course| course.title.as_str()),
            Some("上下文校准课程")
        );
    }

    #[test]
    fn test_build_today_courses_falls_back_when_context_missing() {
        let today = NaiveDate::from_ymd_opt(2026, 6, 19).unwrap();
        let now_time = NaiveTime::from_hms_opt(7, 0, 0).unwrap();
        let courses = vec![sample_course(
            1,
            "课程字段锚点",
            5,
            "08:00",
            "09:40",
            "2026-02-24",
        )];

        let result = build_today_courses(&courses, &[], today, &now_time, &teaching_phase());

        assert_eq!(result.today_count, 1);
    }

    #[test]
    fn test_build_today_courses_hides_courses_when_phase_hidden() {
        let today = NaiveDate::from_ymd_opt(2026, 6, 19).unwrap();
        let now_time = NaiveTime::from_hms_opt(7, 0, 0).unwrap();
        let mut phase = teaching_phase();
        phase.phase_type = "break".to_string();
        phase.courses_visible = false;
        let courses = vec![sample_course(
            1,
            "假期隐藏课程",
            5,
            "08:00",
            "09:40",
            "2026-02-24",
        )];

        let result = build_today_courses(&courses, &[], today, &now_time, &phase);

        assert_eq!(result.today_count, 0);
        assert!(result.next_course.is_none());
    }

    #[test]
    fn test_build_today_courses_hides_courses_when_phase_unknown() {
        // 无学期上下文（unknown）时不应把课程计入今日概览，即使 courses_visible=true。
        let today = NaiveDate::from_ymd_opt(2026, 7, 31).unwrap();
        let now_time = NaiveTime::from_hms_opt(7, 0, 0).unwrap();
        let mut phase = teaching_phase();
        phase.phase_type = crate::term_phase::PHASE_UNKNOWN.to_string();
        phase.courses_visible = true;
        let courses = vec![sample_course(
            1,
            "无上下文课程",
            5,
            "08:00",
            "09:40",
            "2026-03-09",
        )];

        let result = build_today_courses(&courses, &[], today, &now_time, &phase);

        assert_eq!(result.today_count, 0);
        assert!(result.current_course.is_none());
        assert!(result.next_course.is_none());
    }

    #[test]
    fn test_build_today_tasks_daily_unfinished() {
        let today = NaiveDate::from_ymd_opt(2026, 8, 1).unwrap();
        // 今日已完成（last_completed_date == today）的每日任务不计入未完成。
        let mut daily_done = sample_task(1, "已完成的每日", "2026-08-01", "done", "medium");
        daily_done.is_daily = true;
        daily_done.last_completed_date = Some("2026-08-01".to_string());
        // 昨日完成（跨天自然重置）应计入今日未完成。
        let mut daily_yesterday = sample_task(2, "昨日完成的每日", "2026-08-01", "done", "medium");
        daily_yesterday.is_daily = true;
        daily_yesterday.last_completed_date = Some("2026-07-31".to_string());
        // 普通任务不计入每日。
        let normal = sample_task(3, "普通任务", "2026-08-01", "todo", "medium");

        let tasks = vec![daily_done, daily_yesterday, normal];
        let result = build_today_tasks(&tasks, today);

        assert_eq!(result.daily_unfinished_count, 1);
        assert_eq!(result.daily_spotlight.len(), 1);
        assert_eq!(result.daily_spotlight[0].title, "昨日完成的每日");
    }

    #[test]
    fn test_build_today_tasks_daily_spotlight_ordering() {
        let today = NaiveDate::from_ymd_opt(2026, 8, 1).unwrap();
        let mut low = sample_task(1, "低优先级", "2026-08-01", "todo", "low");
        low.is_daily = true;
        let mut high = sample_task(2, "高优先级", "2026-08-01", "todo", "high");
        high.is_daily = true;

        let result = build_today_tasks(&[low, high], today);

        assert_eq!(result.daily_unfinished_count, 2);
        // 高优先级排在前。
        assert_eq!(result.daily_spotlight[0].title, "高优先级");
    }

    #[test]
    fn test_priority_score() {
        assert_eq!(priority_score("high"), 3);
        assert_eq!(priority_score("medium"), 2);
        assert_eq!(priority_score("low"), 1);
        assert_eq!(priority_score("unknown"), 0);
    }
}
