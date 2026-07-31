//! LZU 课表字段到 Kairos 本地课程模型的映射。

use std::collections::BTreeSet;

use chrono::NaiveDate;

use crate::db::models::{CreateCourseRequest, UpsertSemesterContextRequest};
use crate::db::semester::LZU_SEMESTER_CONTEXT_SOURCE;
use crate::importers::infer_color;
use crate::lzu::models::{CourseInfo, XlxxData};

const LZU_CLASS_TIMES: [(&str, &str); 14] = [
    ("08:30", "09:15"),
    ("09:25", "10:10"),
    ("10:30", "11:15"),
    ("11:25", "12:10"),
    ("12:30", "13:15"),
    ("13:25", "14:00"),
    ("14:30", "15:15"),
    ("15:25", "16:10"),
    ("16:30", "17:15"),
    ("17:25", "18:10"),
    ("19:00", "19:45"),
    ("19:55", "20:40"),
    ("21:00", "21:45"),
    ("21:55", "22:40"),
];

pub fn map_course(info: &CourseInfo, xlxx: &XlxxData) -> Result<CreateCourseRequest, String> {
    let name = required_text(info.kcmc.as_deref(), "课程名称 kcmc")?;
    let day_of_week = required_text(info.skxql.as_deref(), "星期 skxql")?
        .parse::<i64>()
        .map_err(|_| "无法解析星期 skxql".to_string())?;
    if !(1..=7).contains(&day_of_week) {
        return Err(format!("星期 skxql 超出范围: {day_of_week}"));
    }

    let (start_time, end_time) = parse_class_bitmap(required_text(info.jc.as_deref(), "节次 jc")?)?;
    let weeks = effective_weeks(info.week.as_deref(), info.week_fb.as_deref())?;
    let week_pattern = format_week_pattern(&weeks)?;
    let semester = infer_semester(info, xlxx)?;
    let semester_start_date = normalize_date(
        required_text(xlxx.ksrq.as_deref(), "学期开始日期 ksrq")?,
        "学期开始日期 ksrq",
    )?;

    Ok(CreateCourseRequest {
        name: name.to_string(),
        day_of_week,
        start_time,
        end_time,
        week_pattern,
        semester_start_date: semester_start_date.to_string(),
        location: info.skjsl.clone().unwrap_or_default(),
        teacher: info.jsxm.clone().unwrap_or_default(),
        color: infer_color(&format!(
            "{}:{name}",
            info.kch.as_deref().unwrap_or_default()
        )),
        semester,
    })
}

pub fn map_semester_context(
    xlxx: &XlxxData,
    semester_hint: Option<&str>,
) -> Result<UpsertSemesterContextRequest, String> {
    let start_date = normalize_date(
        required_text(xlxx.ksrq.as_deref(), "学期开始日期 ksrq")?,
        "学期开始日期 ksrq",
    )?;

    let academic_year = normalized_optional(&xlxx.xn);
    let term = normalized_optional(&xlxx.xqm).or_else(|| normalized_optional(&xlxx.xq));
    let term_label = normalized_str(semester_hint.unwrap_or_default())
        .or_else(|| derive_term_label(academic_year.as_deref(), term.as_deref()))
        .ok_or_else(|| "缺少本地学期标识".to_string())?;

    Ok(UpsertSemesterContextRequest {
        source: LZU_SEMESTER_CONTEXT_SOURCE.to_string(),
        academic_year,
        term,
        term_label,
        start_date: start_date.to_string(),
        current_week: parse_optional_positive_week(xlxx.dqrqszzc.as_deref(), "当前周次 dqrqszzc")?,
        total_weeks: parse_optional_positive_week(xlxx.zzx.as_deref(), "总周次 zzx")?,
    })
}

pub(crate) fn parse_positive_week(raw: &str, field: &str) -> Result<i64, String> {
    let week = raw
        .trim()
        .parse::<i64>()
        .map_err(|_| format!("无法解析 LZU {field}: {raw}"))?;
    if week < 1 {
        return Err(format!("LZU {field} 无效: {week}"));
    }
    Ok(week)
}

fn parse_optional_positive_week(raw: Option<&str>, field: &str) -> Result<Option<i64>, String> {
    raw.map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| parse_positive_week(value, field))
        .transpose()
}

fn required_text<'a>(value: Option<&'a str>, field: &str) -> Result<&'a str, String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("缺少{field}"))
}

/// 归一化日期到 `YYYY-MM-DD`。兼容 LZU API 的 `YYYYMMDD` 与标准 `YYYY-MM-DD` 两种格式，
/// 避免无效日期导致学期上下文持久化被静默跳过。
fn normalize_date(value: &str, field: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if NaiveDate::parse_from_str(trimmed, "%Y-%m-%d").is_ok() {
        return Ok(trimmed.to_string());
    }
    if NaiveDate::parse_from_str(trimmed, "%Y%m%d").is_ok() {
        return Ok(format!(
            "{}-{}-{}",
            &trimmed[0..4],
            &trimmed[4..6],
            &trimmed[6..8]
        ));
    }
    Err(format!("无法解析{field}: {value}"))
}

fn normalized_optional(value: &Option<String>) -> Option<String> {
    value.as_deref().and_then(normalized_str)
}

fn normalized_str(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn derive_term_label(academic_year: Option<&str>, term: Option<&str>) -> Option<String> {
    Some(format!("{}S{}", academic_year?, term?))
}

fn parse_class_bitmap(bitmap: &str) -> Result<(String, String), String> {
    let active: Vec<usize> = bitmap
        .chars()
        .enumerate()
        .filter_map(|(index, ch)| (ch == '1').then_some(index))
        .collect();
    let first = active
        .first()
        .copied()
        .ok_or_else(|| "节次 jc 未包含任何上课节次".to_string())?;
    let last = active
        .last()
        .copied()
        .ok_or_else(|| "节次 jc 未包含任何上课节次".to_string())?;

    let (start_time, _) = LZU_CLASS_TIMES
        .get(first)
        .ok_or_else(|| format!("节次 jc 索引超出范围: {first}"))?;
    let (_, end_time) = LZU_CLASS_TIMES
        .get(last)
        .ok_or_else(|| format!("节次 jc 索引超出范围: {last}"))?;
    Ok((start_time.to_string(), end_time.to_string()))
}

fn effective_weeks(week: Option<&str>, week_fb: Option<&str>) -> Result<Vec<i64>, String> {
    let mut weeks = parse_week_list(required_text(week, "周次 week")?)?;
    if let Some(filter) = week_fb.map(str::trim).filter(|value| !value.is_empty()) {
        let filter_set: BTreeSet<i64> = parse_week_list(filter)?.into_iter().collect();
        weeks.retain(|week| filter_set.contains(week));
    }
    weeks.sort_unstable();
    weeks.dedup();
    if weeks.is_empty() {
        return Err("周次 week 与 week_fb 交集为空".to_string());
    }
    Ok(weeks)
}

fn parse_week_list(raw: &str) -> Result<Vec<i64>, String> {
    raw.split([',', '，', ';', '；', '、', ' '])
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| {
            part.parse::<i64>()
                .map_err(|_| format!("无法解析周次: {part}"))
        })
        .collect()
}

fn format_week_pattern(weeks: &[i64]) -> Result<String, String> {
    let start = *weeks.first().ok_or_else(|| "缺少周次".to_string())?;
    let end = *weeks.last().ok_or_else(|| "缺少周次".to_string())?;

    if weeks.len() == 1 {
        return Ok(format!("{start}周"));
    }
    if is_contiguous(weeks, 1) {
        return Ok(format!("{start}-{end}周全周"));
    }
    if start % 2 == 1 && is_contiguous(weeks, 2) {
        return Ok(format!("{start}-{end}周单周"));
    }
    if start % 2 == 0 && is_contiguous(weeks, 2) {
        return Ok(format!("{start}-{end}周双周"));
    }

    Ok(weeks
        .iter()
        .map(|week| format!("{week}周"))
        .collect::<Vec<_>>()
        .join(","))
}

fn is_contiguous(weeks: &[i64], step: i64) -> bool {
    weeks
        .windows(2)
        .all(|pair| pair[1].saturating_sub(pair[0]) == step)
}

fn infer_semester(info: &CourseInfo, xlxx: &XlxxData) -> Result<String, String> {
    let year = info
        .xn
        .as_deref()
        .or(xlxx.xn.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "缺少学年 xn".to_string())?;
    let term = info
        .xqm
        .map(|value| value.to_string())
        .or_else(|| xlxx.xqm.clone())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "缺少学期 xqm".to_string())?;
    Ok(format!("{year}S{term}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_xlxx() -> XlxxData {
        XlxxData {
            dqrqszzc: Some("1".to_string()),
            zzx: Some("16".to_string()),
            ksrq: Some("2026-02-24".to_string()),
            xqm: Some("1".to_string()),
            xn: Some("2026".to_string()),
            xq: None,
        }
    }

    fn sample_course() -> CourseInfo {
        CourseInfo {
            kch: Some("105404003".to_string()),
            kcmc: Some("通信原理".to_string()),
            jsxm: Some("张冠茂".to_string()),
            jc: Some("11000000000000".to_string()),
            skjsl: Some("秦岭堂B404".to_string()),
            skxql: Some("2".to_string()),
            week: Some("1,2,3,4".to_string()),
            bs: None,
            xykh: None,
            xn: Some("2026".to_string()),
            xqm: Some(1),
            status: None,
            color: None,
            sksj: None,
            xf: None,
            week_fb: None,
            kcrq: None,
            create_time: None,
            create_user_id: None,
        }
    }

    #[test]
    fn test_map_course() {
        let mapped = map_course(&sample_course(), &sample_xlxx()).expect("map course");
        assert_eq!(mapped.name, "通信原理");
        assert_eq!(mapped.day_of_week, 2);
        assert_eq!(mapped.start_time, "08:30");
        assert_eq!(mapped.end_time, "10:10");
        assert_eq!(mapped.week_pattern, "1-4周全周");
        assert_eq!(mapped.semester_start_date, "2026-02-24");
        assert_eq!(mapped.semester, "2026S1");
    }

    #[test]
    fn test_map_semester_context_keeps_missing_total_weeks_unknown() {
        let mut xlxx = sample_xlxx();
        xlxx.zzx = None;

        let context = map_semester_context(&xlxx, Some("2026S1")).expect("map semester context");

        assert_eq!(context.source, LZU_SEMESTER_CONTEXT_SOURCE);
        assert_eq!(context.academic_year.as_deref(), Some("2026"));
        assert_eq!(context.term.as_deref(), Some("1"));
        assert_eq!(context.term_label, "2026S1");
        assert_eq!(context.start_date, "2026-02-24");
        assert_eq!(context.current_week, Some(1));
        assert_eq!(context.total_weeks, None);
    }

    #[test]
    fn test_map_semester_context_derives_term_label_without_hint() {
        let context = map_semester_context(&sample_xlxx(), None).expect("map semester context");
        assert_eq!(context.term_label, "2026S1");
        assert_eq!(context.total_weeks, Some(16));
    }

    #[test]
    fn test_map_semester_context_rejects_invalid_start_date() {
        let mut xlxx = sample_xlxx();
        xlxx.ksrq = Some("2026/02/24".to_string());

        assert!(map_semester_context(&xlxx, Some("2026S1")).is_err());
    }

    #[test]
    fn test_normalize_date_accepts_compact_format() {
        assert_eq!(
            normalize_date("20260309", "test").expect("normalize"),
            "2026-03-09"
        );
        assert_eq!(
            normalize_date("2026-03-09", "test").expect("normalize"),
            "2026-03-09"
        );
        assert!(normalize_date("not-a-date", "test").is_err());
    }

    #[test]
    fn test_map_course_normalizes_compact_ksrq() {
        let mut xlxx = sample_xlxx();
        xlxx.ksrq = Some("20260309".to_string());

        let mapped = map_course(&sample_course(), &xlxx).expect("map course");
        assert_eq!(mapped.semester_start_date, "2026-03-09");
    }

    #[test]
    fn test_map_semester_context_normalizes_compact_ksrq() {
        let mut xlxx = sample_xlxx();
        xlxx.ksrq = Some("20260309".to_string());

        let context = map_semester_context(&xlxx, Some("2026S1")).expect("map context");
        assert_eq!(context.start_date, "2026-03-09");
        assert_eq!(context.total_weeks, Some(16));
    }

    #[test]
    fn test_map_semester_context_rejects_invalid_weeks() {
        let mut xlxx = sample_xlxx();
        xlxx.dqrqszzc = Some("0".to_string());
        assert!(map_semester_context(&xlxx, Some("2026S1")).is_err());

        xlxx.dqrqszzc = Some("1".to_string());
        xlxx.zzx = Some("bad".to_string());
        assert!(map_semester_context(&xlxx, Some("2026S1")).is_err());
    }

    #[test]
    fn test_parse_class_bitmap_midday_and_evening() {
        assert_eq!(
            parse_class_bitmap("00001100000000"),
            Ok(("12:30".to_string(), "14:00".to_string()))
        );
        assert_eq!(
            parse_class_bitmap("00000000001111"),
            Ok(("19:00".to_string(), "22:40".to_string()))
        );
    }

    #[test]
    fn test_format_week_pattern() {
        assert_eq!(
            format_week_pattern(&[1, 2, 3, 4]),
            Ok("1-4周全周".to_string())
        );
        assert_eq!(format_week_pattern(&[1, 3, 5]), Ok("1-5周单周".to_string()));
        assert_eq!(format_week_pattern(&[2, 4, 6]), Ok("2-6周双周".to_string()));
        assert_eq!(
            format_week_pattern(&[1, 4, 7]),
            Ok("1周,4周,7周".to_string())
        );
    }

    #[test]
    fn test_effective_weeks_intersects_week_fb() {
        assert_eq!(
            effective_weeks(Some("1,2,3,4,5,6"), Some("2,4,6")),
            Ok(vec![2, 4, 6])
        );
    }
}
