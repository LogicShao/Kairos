use chrono::{FixedOffset, NaiveDateTime, TimeZone, Utc};
use serde::Serialize;

use crate::db::models::{CreateCourseRequest, CreateExamRequest};

const IMPORT_COLORS: [&str; 8] = [
    "#3B82F6", "#10B981", "#F59E0B", "#EC4899", "#06B6D4", "#EF4444", "#7C8CC0", "#8B5CF6",
];

#[derive(Debug, Clone, Serialize)]
pub struct ImportTextResult {
    pub parsed: usize,
    pub imported: usize,
    pub skipped: usize,
    pub message: String,
}

impl ImportTextResult {
    pub fn from_counts(parsed: usize, imported: usize, skipped: usize) -> Self {
        Self {
            parsed,
            imported,
            skipped,
            message: format!("已解析 {parsed} 条，导入 {imported} 条，跳过 {skipped} 条重复记录。"),
        }
    }
}

fn normalize_line(raw_line: &str) -> String {
    raw_line.replace('\r', "").trim().to_string()
}

fn is_course_header_line(line: &str) -> bool {
    line.contains("课程号")
        || line.contains("课程名称")
        || line.contains("上课时间、地点")
        || line.contains("考试性质")
        || line.contains("缓考")
}

fn is_exam_header_line(line: &str) -> bool {
    line.contains("课程号") || line.contains("考试时间") || line.contains("考试地点")
}

fn is_course_start_line(line: &str) -> bool {
    let cells: Vec<&str> = line.split('\t').map(str::trim).collect();
    cells.len() >= 4
        && cells[0]
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_digit())
        && cells[1].chars().all(|ch| ch.is_ascii_digit())
        && !cells[2].is_empty()
}

fn is_course_metadata_line(line: &str) -> bool {
    let cells: Vec<&str> = line.split('\t').map(str::trim).collect();
    cells.len() >= 5 && cells[0].parse::<f64>().is_ok()
}

fn is_time_location_line(line: &str) -> bool {
    let cells: Vec<&str> = line.split('\t').map(str::trim).collect();
    cells.len() >= 4 && cells[0].contains('周') && matches_day_label(cells[1]).is_some()
}

fn is_terminator_line(line: &str) -> bool {
    line.split('\t').map(str::trim).any(|cell| cell == "查看")
}

fn matches_day_label(label: &str) -> Option<i64> {
    match label {
        "星期一" => Some(1),
        "星期二" => Some(2),
        "星期三" => Some(3),
        "星期四" => Some(4),
        "星期五" => Some(5),
        "星期六" => Some(6),
        "星期日" | "星期天" => Some(7),
        _ => None,
    }
}

fn infer_color(seed: &str) -> String {
    let mut hash: u32 = 0;
    for ch in seed.chars() {
        hash = hash.wrapping_mul(31).wrapping_add(ch as u32);
    }
    IMPORT_COLORS[(hash as usize) % IMPORT_COLORS.len()].to_string()
}

fn normalize_teacher(lines: &[String]) -> String {
    lines
        .iter()
        .map(|line| normalize_line(line))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" / ")
}

fn parse_time_range(slot_text: &str) -> Option<(String, String)> {
    let normalized = slot_text.replace(char::is_whitespace, "");

    match normalized.as_str() {
        "上午12节" => Some(("08:00".to_string(), "09:40".to_string())),
        "上午34节" => Some(("10:00".to_string(), "11:40".to_string())),
        "下午56节" => Some(("14:00".to_string(), "15:40".to_string())),
        "下午78节" => Some(("16:00".to_string(), "17:40".to_string())),
        "晚9-10节" => Some(("19:00".to_string(), "20:40".to_string())),
        "晚9-11节" => Some(("19:00".to_string(), "21:30".to_string())),
        "中午第1节" => Some(("12:10".to_string(), "12:55".to_string())),
        "下午5-7节" => Some(("14:00".to_string(), "16:25".to_string())),
        _ => {
            let (period, digits) = if let Some(rest) = normalized.strip_prefix("上午") {
                ("上午", rest)
            } else if let Some(rest) = normalized.strip_prefix("下午") {
                ("下午", rest)
            } else if let Some(rest) = normalized.strip_prefix("晚") {
                ("晚", rest)
            } else {
                return None;
            };

            let digits = digits.strip_suffix('节')?;
            let mut parts = digits.split('-');
            let start_index = parts.next()?.parse::<i64>().ok()?;
            let end_index = parts
                .next()
                .unwrap_or_default()
                .parse::<i64>()
                .ok()
                .unwrap_or(start_index);

            let start_minutes = lesson_index_to_minutes(period, start_index)?;
            let end_minutes = lesson_index_to_minutes(period, end_index + 1)?;
            if end_minutes <= start_minutes {
                return None;
            }

            Some((minutes_to_time(start_minutes), minutes_to_time(end_minutes)))
        }
    }
}

fn lesson_index_to_minutes(period: &str, index: i64) -> Option<i64> {
    match (period, index) {
        ("上午", 1) => Some(8 * 60),
        ("上午", 2) => Some(8 * 60 + 50),
        ("上午", 3) => Some(10 * 60),
        ("上午", 4) => Some(10 * 60 + 50),
        ("下午", 5) => Some(14 * 60),
        ("下午", 6) => Some(14 * 60 + 50),
        ("下午", 7) => Some(16 * 60),
        ("下午", 8) => Some(16 * 60 + 50),
        ("晚", 9) => Some(19 * 60),
        ("晚", 10) => Some(19 * 60 + 50),
        ("晚", 11) => Some(20 * 60 + 40),
        _ => None,
    }
}

fn minutes_to_time(total_minutes: i64) -> String {
    let hours = total_minutes / 60;
    let minutes = total_minutes % 60;
    format!("{hours:02}:{minutes:02}")
}

pub fn parse_course_import_text(
    text: &str,
    semester: &str,
    semester_start_date: &str,
) -> Result<Vec<CreateCourseRequest>, String> {
    let lines: Vec<String> = text
        .lines()
        .map(normalize_line)
        .filter(|line| !line.is_empty())
        .collect();

    let mut courses = Vec::new();
    let mut index = 0usize;

    while index < lines.len() {
        let line = &lines[index];
        if is_course_header_line(line) || is_terminator_line(line) {
            index += 1;
            continue;
        }

        if !is_course_start_line(line) {
            index += 1;
            continue;
        }

        let start_cells: Vec<&str> = line.split('\t').map(str::trim).collect();
        let course_code = start_cells[0];
        let course_name = start_cells[2];
        let mut teacher_lines = vec![start_cells[3].to_string()];
        index += 1;

        while index < lines.len() && !is_course_metadata_line(&lines[index]) {
            let current_line = &lines[index];
            if is_time_location_line(current_line)
                || is_course_start_line(current_line)
                || is_terminator_line(current_line)
            {
                break;
            }
            teacher_lines.push(current_line.clone());
            index += 1;
        }

        if index < lines.len() && is_course_metadata_line(&lines[index]) {
            index += 1;
        }

        let teacher = normalize_teacher(&teacher_lines);
        while index < lines.len() && is_time_location_line(&lines[index]) {
            let cells: Vec<&str> = lines[index].split('\t').map(str::trim).collect();
            let week_pattern = cells[0].to_string();
            let day_of_week = matches_day_label(cells[1])
                .ok_or_else(|| format!("无法解析星期字段: {}", cells[1]))?;
            let (start_time, end_time) = parse_time_range(cells[2])
                .ok_or_else(|| format!("无法解析节次字段: {}", cells[2]))?;

            courses.push(CreateCourseRequest {
                name: course_name.to_string(),
                day_of_week,
                start_time,
                end_time,
                week_pattern,
                semester_start_date: semester_start_date.to_string(),
                location: cells.get(3).copied().unwrap_or_default().to_string(),
                teacher: teacher.clone(),
                color: infer_color(&format!("{course_code}:{course_name}")),
                semester: semester.to_string(),
            });
            index += 1;
        }

        while index < lines.len() && is_terminator_line(&lines[index]) {
            index += 1;
        }
    }

    if courses.is_empty() {
        return Err("未识别到可导入的课程，请确认复制内容来自教务系统课表表格。".to_string());
    }

    Ok(courses)
}

pub fn parse_exam_import_text(
    text: &str,
    semester: &str,
) -> Result<Vec<CreateExamRequest>, String> {
    let lines: Vec<String> = text
        .lines()
        .map(normalize_line)
        .filter(|line| !line.is_empty())
        .collect();

    let mut exams = Vec::new();
    for line in lines {
        if is_exam_header_line(&line) {
            continue;
        }

        let cells: Vec<&str> = line.split('\t').map(str::trim).collect();
        if cells.len() < 4 || cells[1].is_empty() || cells[2].is_empty() {
            continue;
        }

        let (exam_datetime, exam_end_datetime) = parse_exam_time_range(cells[2])?;
        exams.push(CreateExamRequest {
            course_name: cells[1].to_string(),
            exam_datetime,
            exam_end_datetime,
            location: cells.get(3).copied().unwrap_or_default().to_string(),
            notes: cells.get(4).copied().unwrap_or_default().to_string(),
            course_id: None,
            semester: semester.to_string(),
        });
    }

    if exams.is_empty() {
        return Err("未识别到可导入的考试，请确认复制内容来自教务系统考试表格。".to_string());
    }

    Ok(exams)
}

fn parse_exam_time_range(raw: &str) -> Result<(String, String), String> {
    let normalized = raw.trim();
    let start_raw = normalized
        .get(..16)
        .ok_or_else(|| format!("无法解析考试时间字段: {raw}"))?;
    let end_raw = normalized
        .get(16..)
        .unwrap_or_default()
        .trim_start_matches(|ch: char| {
            ch.is_whitespace() || ch == '-' || ch == '—' || ch == '–' || ch == '－'
        })
        .trim();
    if end_raw.is_empty() {
        return Err(format!("无法解析考试时间字段: {raw}"));
    }

    let start_naive = NaiveDateTime::parse_from_str(start_raw.trim(), "%Y-%m-%d %H:%M")
        .map_err(|_| format!("无法解析考试开始时间: {raw}"))?;

    let end_naive = if end_raw.contains(' ') {
        NaiveDateTime::parse_from_str(end_raw, "%Y-%m-%d %H:%M")
            .map_err(|_| format!("无法解析考试结束时间: {raw}"))?
    } else {
        let date = start_naive.date();
        let end_datetime = format!("{} {}", date.format("%Y-%m-%d"), end_raw);
        NaiveDateTime::parse_from_str(&end_datetime, "%Y-%m-%d %H:%M")
            .map_err(|_| format!("无法解析考试结束时间: {raw}"))?
    };
    if end_naive < start_naive {
        return Err(format!("考试结束时间早于开始时间: {raw}"));
    }

    let offset = FixedOffset::east_opt(8 * 3600).expect("china utc offset");
    let start = offset
        .from_local_datetime(&start_naive)
        .single()
        .ok_or_else(|| format!("无法确定考试开始时间: {raw}"))?;
    let end = offset
        .from_local_datetime(&end_naive)
        .single()
        .ok_or_else(|| format!("无法确定考试结束时间: {raw}"))?;

    Ok((
        start
            .with_timezone(&Utc)
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string(),
        end.with_timezone(&Utc)
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_course_import_text() {
        let text = "课程号\t课程\n序号\t课程名称\t任课教师\t学 分\t选课属性\t考核方式\t考试\n性质\t是否\n缓考\t上课时间、地点\t教材\t教学记录\t过程性成绩\n2043056\t3\t自动控制原理\t李红信\n3\t必修\t未确定\t正常考试\t非缓考\n1-17周全周\t星期三\t上午34节\t秦岭堂A114\n1-17周单周\t星期二\t晚9-10节\t天山堂A312\n \t查看\t查看";

        let courses =
            parse_course_import_text(text, "2026S1", "2026-02-24").expect("parse course import");
        assert_eq!(courses.len(), 2);
        assert_eq!(courses[0].name, "自动控制原理");
        assert_eq!(courses[0].week_pattern, "1-17周全周");
        assert_eq!(courses[0].semester_start_date, "2026-02-24");
        assert_eq!(courses[1].day_of_week, 2);
        assert_eq!(courses[1].start_time, "19:00");
        assert_eq!(courses[1].end_time, "20:40");
    }

    #[test]
    fn test_parse_exam_import_text() {
        let text = "课程号\t课程名称\t考试时间\t考试地点\t考试性质\n2043056\t自动控制原理\t2026-07-06 16:00--18:00\t天山堂A409\t正常考试";
        let exams = parse_exam_import_text(text, "2026S1").expect("parse exam import");
        assert_eq!(exams.len(), 1);
        assert_eq!(exams[0].course_name, "自动控制原理");
        assert_eq!(exams[0].location, "天山堂A409");
        assert_eq!(exams[0].notes, "正常考试");
        assert_eq!(exams[0].semester, "2026S1");
        assert_eq!(exams[0].exam_datetime, "2026-07-06T08:00:00Z");
        assert_eq!(exams[0].exam_end_datetime, "2026-07-06T10:00:00Z");
    }

    #[test]
    fn test_parse_exam_import_text_with_single_dash_separator() {
        let text = "2043056\t自动控制原理\t2026-07-06 16:00-18:00\t天山堂A409\t正常考试";
        let exams = parse_exam_import_text(text, "2026S1").expect("parse exam import");
        assert_eq!(exams[0].exam_datetime, "2026-07-06T08:00:00Z");
        assert_eq!(exams[0].exam_end_datetime, "2026-07-06T10:00:00Z");
    }
}
