//! 本地规则引擎降级摘要。输出与 AI 版同构（固定 5 分节 + footer），零 AI 依赖。

use crate::ai::{AIService, AiBriefResult, AiError, BriefSource};
use crate::commands::briefing::TodayBriefingResponse;

/// 无 AI / 无 key / 离线 / AI 失败时的本地确定性摘要。
pub struct RuleBasedService;

impl AIService for RuleBasedService {
    fn generate_daily_brief(&self, b: &TodayBriefingResponse) -> Result<AiBriefResult, AiError> {
        Ok(AiBriefResult {
            summary: render_rule_briefing(b),
            source: BriefSource::Rule,
            generated_at: crate::db::chrono_now(),
            model: None,
        })
    }
}

/// 用本地模板逐节拼出与 AI 版同构的 5 分节 markdown。
pub fn render_rule_briefing(b: &TodayBriefingResponse) -> String {
    let mut out = String::new();
    out.push_str(&format!("# {} {} 晨间摘要\n\n", b.date, b.weekday_label));

    // 今日重点：综合逾期/到期/考试的最紧急提示。
    out.push_str("## 今日重点\n");
    let mut focus: Vec<String> = Vec::new();
    if b.tasks.overdue_count > 0 {
        focus.push(format!(
            "有 {} 项任务逾期，建议优先处理。",
            b.tasks.overdue_count
        ));
    } else if b.tasks.due_today_count > 0 {
        focus.push(format!("今天有 {} 项任务到期。", b.tasks.due_today_count));
    }
    if let Some(exam) = &b.exam {
        focus.push(format!(
            "{} 还有 {} 天，建议提前复习。",
            exam.course_name, exam.days_until
        ));
    }
    if let Some(term) = &b.phase.term_label {
        if let Some(week) = b.phase.current_week {
            focus.push(format!("当前{}第 {} 周。", term, week));
        }
    }
    if focus.is_empty() {
        focus.push("今天暂无紧急事项，按计划推进即可。".to_string());
    }
    out.push_str(&focus.join(" "));
    out.push_str("\n\n");

    // 课程
    out.push_str("## 课程\n");
    if !b.phase.courses_visible {
        out.push_str("暂无课程安排\n\n");
    } else if b.courses.today_count == 0 {
        out.push_str("今天没有课程\n\n");
    } else {
        out.push_str(&format!("今日共 {} 节。", b.courses.today_count));
        if let Some(cur) = &b.courses.current_course {
            out.push_str(&format!(
                " 当前：{} {}（{}）。",
                cur.title, cur.start_time, cur.location
            ));
        } else if let Some(next) = &b.courses.next_course {
            out.push_str(&format!(
                " 下一节：{} {} @ {}。",
                next.title, next.start_time, next.location
            ));
        }
        out.push_str("\n\n");
    }

    // 待办
    out.push_str("## 待办\n");
    if b.tasks.overdue_count == 0 && b.tasks.due_today_count == 0 {
        out.push_str("暂无\n\n");
    } else {
        out.push_str(&format!(
            "{} 项逾期、{} 项今日到期。",
            b.tasks.overdue_count, b.tasks.due_today_count
        ));
        if let Some(first) = b.tasks.spotlight.first() {
            out.push_str(&format!(" 最要紧：{}。", first.title));
        }
        out.push_str("\n\n");
    }

    // 考试
    out.push_str("## 考试\n");
    match &b.exam {
        Some(exam) => out.push_str(&format!(
            "{} 还有 {} 天（{}，{}）。",
            exam.course_name, exam.days_until, exam.exam_datetime, exam.location
        )),
        None => out.push_str("暂无\n\n"),
    }

    // 专注
    out.push_str("## 专注\n");
    if b.pomodoro.is_running {
        out.push_str(&format!(
            "当前{}进行中，已完成 {} 个番茄钟。",
            b.pomodoro.phase, b.pomodoro.completed_sessions
        ));
    } else {
        out.push_str(&format!(
            "今日已完成 {} 个番茄钟，可开始下一轮专注。",
            b.pomodoro.completed_sessions
        ));
    }
    out.push_str("\n\n");

    out.push_str("---\n");
    out.push_str(crate::ai::prompt::RULE_FOOTER);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::prompt::{RULE_FOOTER, SECTION_TITLES};
    use crate::ai::AIService;
    use crate::commands::briefing::TodayCourses;

    fn sample_briefing() -> TodayBriefingResponse {
        TodayBriefingResponse {
            date: "2026-07-31".to_string(),
            weekday_label: "周五".to_string(),
            courses: TodayCourses {
                today_count: 2,
                current_course: None,
                next_course: Some(crate::commands::briefing::NextCourse {
                    title: "操作系统".to_string(),
                    start_time: "10:10".to_string(),
                    end_time: "11:50".to_string(),
                    location: "天山堂A205".to_string(),
                }),
            },
            tasks: crate::commands::briefing::TodayTasks {
                overdue_count: 1,
                due_today_count: 2,
                spotlight: vec![crate::commands::briefing::TaskSpotlight {
                    id: 1,
                    title: "交操作系统实验报告".to_string(),
                    priority: "high".to_string(),
                    due_date: Some("2026-07-31".to_string()),
                }],
                daily_unfinished_count: 0,
                daily_spotlight: vec![],
            },
            exam: Some(crate::commands::briefing::UpcomingExam {
                id: 1,
                course_name: "高数期末".to_string(),
                exam_datetime: "2026-08-05T08:00:00+08:00".to_string(),
                days_until: 5,
                location: "天山堂B103".to_string(),
            }),
            pomodoro: crate::commands::briefing::PomodoroBriefing {
                is_running: false,
                phase: "work".to_string(),
                remaining_seconds: 0,
                completed_sessions: 2,
            },
            phase: crate::commands::briefing::PhaseBriefing {
                phase_type: "teaching".to_string(),
                term_label: Some("2026S1".to_string()),
                current_week: Some(17),
                courses_visible: true,
                exam_notifications_enabled: true,
                pomodoro_profile: "default".to_string(),
            },
        }
    }

    #[test]
    fn test_rule_briefing_has_all_sections_and_footer() {
        let md = render_rule_briefing(&sample_briefing());
        for title in SECTION_TITLES {
            assert!(md.contains(title), "缺节标题 {title}");
        }
        assert!(md.trim_end().ends_with(RULE_FOOTER));
        assert!(md.contains("操作系统"));
        assert!(md.contains("高数期末"));
        assert!(md.contains("1 项逾期"));
    }

    #[test]
    fn test_rule_briefing_empty_state() {
        let mut b = sample_briefing();
        b.courses.today_count = 0;
        b.courses.next_course = None;
        b.tasks.overdue_count = 0;
        b.tasks.due_today_count = 0;
        b.tasks.spotlight.clear();
        b.exam = None;

        let md = render_rule_briefing(&b);
        assert!(md.contains("今天没有课程"));
        assert!(md.contains("暂无"));
    }

    #[test]
    fn test_rule_service_source_is_rule() {
        let service = RuleBasedService;
        let result = service
            .generate_daily_brief(&sample_briefing())
            .expect("rule brief should never fail");
        assert_eq!(result.source, BriefSource::Rule);
        assert!(result.model.is_none());
        assert!(!result.summary.is_empty());
    }
}
