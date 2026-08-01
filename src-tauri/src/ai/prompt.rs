//! 系统提示词、成本常量、user 消息构建与 AI 输出结构校验。
//!
//! 系统提示词是编译期固定的中文常量，不来自用户输入、不入库，保证 token 数稳定可测算。

use serde::Serialize;

use crate::commands::briefing::{
    PhaseBriefing, PomodoroBriefing, TodayBriefingResponse, TodayCourses,
};

/// 默认模型名（配置页可改）。成本预算按 deepseek-v4-flash 官方价估算，见 design.md。
pub const MODEL_DEFAULT: &str = "deepseek-v4-flash";
/// 单次生成 token 上限（成本护栏，硬编码，不信任前端）。
pub const MAX_TOKENS: u32 = 400;
/// 生成温度。
pub const TEMPERATURE: f32 = 0.7;
/// user 消息 JSON 字符数兜底上限；超限直接降级规则引擎，绝不中途截断 JSON。
pub const MAX_CONTEXT_CHARS: usize = 4000;

/// AI 输出 footer（强制结尾行，反幻觉标注）。
pub const AI_FOOTER: &str = "*AI生成，请核实*";
/// 规则引擎输出 footer（区分来源）。
pub const RULE_FOOTER: &str = "*本地生成*";

/// 固定 5 个节标题，结构校验依赖它们逐字出现。
pub const SECTION_TITLES: [&str; 5] = ["## 今日重点", "## 课程", "## 待办", "## 考试", "## 专注"];

/// 系统提示词（中文，编译期固定）。
pub const SYSTEM_PROMPT: &str = r#"你是「Kairos」学生的中文晨间学习助手。每天早晨根据系统提供的 JSON 数据，生成一份简洁自然的 Markdown 晨间摘要，帮学生快速规划当天。

## 硬性规则
1. 只基于给定 JSON 总结，绝不编造或外推任何信息。JSON 里没有的一律不写。
2. 课程名称、起止时间、地点；考试课程、考试时间、剩余天数、地点；任务标题；所有数字计数（课程节数、逾期数、今日到期数、已完成番茄数、当前周次），必须逐字引用，不得改写、不得计算、不得补全。
3. 考试一律用 JSON 的 days_until 表述（如"还有 5 天"），不要自行换算日期或时区；如需日期可原样引用 exam_datetime。
4. 空节必须明确写"暂无"。无考试→"暂无"；无课程安排→"暂无课程安排"；无待办→"暂无"。
5. 语气简洁口语化，适合学生早晨快读。每节 1-3 个要点，全文不超过 350 字。
6. 标题使用 JSON 里的 date 与 weekday_label 原样拼出，格式：# {date} {weekday_label} 晨间摘要。
7. 输出必须使用下方固定结构，节标题一字不差；最后一行必须是：*AI生成，请核实*

## 允许与禁止
- 允许：基于已有数据给出学习优先级、番茄钟建议，用"建议…"口吻，不得写成事实陈述。
- 禁止：引用 JSON 之外字段（如教师姓名、教室未给出时编造地点）；猜测或补全考试时间/地点；杜撰任务；虚构课程。

## 字段含义（助你理解，不要照抄）
- courses.current_course/next_course 是正在上/下一节未开始的课；两者为 null 分别表示无进行中课程/无后续课程。
- tasks.priority 取值 high|medium|low；spotlight 已按 逾期>高优先级>早截止 排序。
- pomodoro.phase 取值 work|short_break|long_break。
- phase.courses_visible 为 false 时表示非教学周，课程节必须写"暂无课程安排"。

## 输出结构（必须完全一致）
# {date} {weekday_label} 晨间摘要

## 今日重点
（1-2 句综合建议，可含优先级引导）

## 课程
（今日共 N 节；进行中/下一节课程；无课写"暂无课程安排"）

## 待办
（逾期/今日到期；列出最要紧的 1-3 项）

## 考试
（最近考试信息；无则写"暂无"）

## 专注
（番茄钟状态与建议）

---
*AI生成，请核实*
"#;

// ─── user 消息构建（裁剪 id 减 token）──────────────────────────────────────────

/// 裁剪掉对生成无用的 id 字段（TaskSpotlight.id / UpcomingExam.id）。
#[derive(Serialize)]
struct TrimmedSpotlight<'a> {
    title: &'a str,
    priority: &'a str,
    due_date: Option<&'a str>,
}

#[derive(Serialize)]
struct TrimmedExam<'a> {
    course_name: &'a str,
    exam_datetime: &'a str,
    days_until: i64,
    location: &'a str,
}

#[derive(Serialize)]
struct TrimmedTasks<'a> {
    overdue_count: i64,
    due_today_count: i64,
    spotlight: Vec<TrimmedSpotlight<'a>>,
}

#[derive(Serialize)]
struct TrimmedBriefing<'a> {
    date: &'a str,
    weekday_label: &'a str,
    courses: &'a TodayCourses,
    tasks: TrimmedTasks<'a>,
    exam: Option<TrimmedExam<'a>>,
    pomodoro: &'a PomodoroBriefing,
    phase: &'a PhaseBriefing,
}

/// 构建 user 消息：把 `TodayBriefingResponse` 序列化为裁剪后的 JSON。
///
/// 超 `MAX_CONTEXT_CHARS` 时返回 `Err`，调用方应降级规则引擎（绝不中途截断 JSON）。
pub fn build_user_message(b: &TodayBriefingResponse) -> Result<String, String> {
    let trimmed = TrimmedBriefing {
        date: &b.date,
        weekday_label: &b.weekday_label,
        courses: &b.courses,
        tasks: TrimmedTasks {
            overdue_count: b.tasks.overdue_count,
            due_today_count: b.tasks.due_today_count,
            spotlight: b
                .tasks
                .spotlight
                .iter()
                .map(|s| TrimmedSpotlight {
                    title: &s.title,
                    priority: &s.priority,
                    due_date: s.due_date.as_deref(),
                })
                .collect(),
        },
        exam: b.exam.as_ref().map(|e| TrimmedExam {
            course_name: &e.course_name,
            exam_datetime: &e.exam_datetime,
            days_until: e.days_until,
            location: &e.location,
        }),
        pomodoro: &b.pomodoro,
        phase: &b.phase,
    };

    let json = serde_json::to_string(&trimmed).map_err(|e| format!("序列化简报失败: {e}"))?;
    if json.chars().count() > MAX_CONTEXT_CHARS {
        return Err(format!(
            "今日数据超过 {MAX_CONTEXT_CHARS} 字符上限，降级本地规则生成"
        ));
    }
    Ok(format!("以下是今日数据，请按系统提示生成摘要：\n{json}"))
}

// ─── AI 输出结构校验（反幻觉/成本兜底）────────────────────────────────────────

/// 校验 AI 响应是否符合固定结构：必含 5 个节标题且以 AI footer 结尾。
///
/// 失败时调用方降级规则引擎，**不发起第二次 API 调用**。
pub fn validate_ai_output(markdown: &str) -> bool {
    SECTION_TITLES.iter().all(|title| markdown.contains(title))
        && markdown.trim_end().ends_with(AI_FOOTER)
}
