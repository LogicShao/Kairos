# Schedule Import and Week View Contracts

> Backend contracts for importing teaching-system table text and returning weekly schedule items.

---

## Scenario: Clipboard Schedule Import And Week View

### 1. Scope / Trigger

- Trigger: course/exam import commands, database schema changes, and a cross-layer week schedule response.
- Applies to Rust backend files under `src-tauri/src/importers.rs`, `commands/{courses,exams,schedule}.rs`, `db/{courses,exams,migrations,models}.rs`, `schedule.rs`, and `sync/exporter.rs`.

### 2. Signatures

- `commands::courses::import_courses_from_text(db, cmd: ImportCoursesCmd) -> Result<ImportTextResult, String>`
- `commands::exams::import_exams_from_text(db, cmd: ImportExamsCmd) -> Result<ImportTextResult, String>`
- `commands::schedule::get_week_schedule(db, cmd: WeekScheduleCmd) -> Result<WeekScheduleResponse, String>`
- `commands::schedule::get_calendar_week(db, cmd: CalendarWeekCmd) -> Result<CalendarWeekResponse, String>`
- Migration v3 `course_week_and_exam_range` adds:
  - `courses.week_pattern TEXT NOT NULL DEFAULT ''`
  - `courses.semester_start_date TEXT NOT NULL DEFAULT ''`
  - `exams.exam_end_datetime TEXT NOT NULL DEFAULT ''`
  - `exams.semester TEXT NOT NULL DEFAULT ''`

### 3. Contracts

- `ImportCoursesCmd` fields:
  - `text: String` - raw tab-separated clipboard text copied from the teaching-system course table.
  - `semester: String` - stored on every imported course.
  - `semester_start_date: String` - `YYYY-MM-DD`, stored on every imported course and used as week 1 anchor.
- `ImportExamsCmd` fields:
  - `text: String` - raw tab-separated clipboard text copied from the teaching-system exam table.
  - `semester: String` - stored on every imported exam.
- `ImportTextResult` fields:
  - `parsed: usize` - rows parsed from clipboard text.
  - `imported: usize` - rows inserted into SQLite.
  - `skipped: usize` - parsed rows skipped as obvious duplicates.
  - `message: String` - user-displayable summary.
- `WeekScheduleCmd` fields:
  - `semester: String` - filters courses and exams.
  - `week_index: i64` - one-based week number, must be `>= 1`.
  - `semester_start_date: Option<String>` - optional `YYYY-MM-DD` override. If absent, backend uses the first course's `semester_start_date`.
- `WeekScheduleResponse.items[]` returns a unified list:
  - `kind: "course" | "exam"`
  - `day_of_week: 1..=7`
  - `start_time` and `end_time` as local `HH:mm`
  - `course_id: Some(course.id)` for courses, exam-linked course id for exams
  - `week_pattern` populated for courses and empty for exams
- `CalendarWeekCmd` supports a date-first calendar query:
  - `semester: String` filters courses only.
  - `week_index: i64` remains accepted for semester-week compatibility.
  - `semester_start_date: Option<String>` optionally anchors course week calculation.
  - `week_start_date: Option<String>` optionally selects a real `YYYY-MM-DD` week for vacation and cross-semester browsing.
- `CalendarWeekResponse.events[]` returns `kind: "course" | "exam" | "task"`.
  - Courses follow semester and week-pattern rules.
  - Exams are date-scoped calendar events and must not be dropped only because `exam.semester` is empty or differs from the current course semester.
  - Tasks with `due_date` inside the week appear as all-day/date-level events.

### 4. Validation & Error Matrix

- Course text contains no recognizable course time rows -> `Err("未识别到可导入的课程...")`.
- Exam text contains no recognizable exam rows -> `Err("未识别到可导入的考试...")`.
- Exam time cannot parse as `YYYY-MM-DD HH:mm--HH:mm`, `YYYY-MM-DD HH:mm-HH:mm`, or full end datetime -> `Err("无法解析考试时间字段...")`.
- `week_index < 1` -> `Err("week_index 必须大于等于 1")`.
- Invalid `CalendarWeekCmd.week_start_date` -> `Err("无法解析周起始日期: ...")`.
- No usable semester start date from command or course data -> `Err("缺少学期开始日期，无法计算周视图。")`.
- Duplicate imported rows are not errors. They increment `ImportTextResult.skipped`.
- SQLite failures are converted at the command boundary with `.map_err(|e| e.to_string())`.

### 5. Good / Base / Bad Cases

- Good: importing `_TEMP/_import_class_ref.txt` creates separate course rows for each time/location block, preserving `week_pattern` and `semester_start_date`.
- Good: importing `_TEMP/_import_final_ref.txt` stores both UTC start and end datetimes and the selected semester.
- Base: repeated paste of the same course or exam returns `imported = 0`, `skipped = parsed`.
- Bad: deriving week rules from `location` text. Week rules must be stored in `courses.week_pattern`.
- Bad: letting the frontend merge exams into weekly events independently. The backend `get_week_schedule` command owns the unified event contract.

### 6. Tests Required

- Parser tests:
  - course table sample produces expected count, week pattern, day, start/end time, teacher, and semester start date.
  - exam table sample produces UTC start/end times and semester.
  - exam single-dash time separator does not split the date hyphens.
- Command helper tests:
  - duplicate course import skips the second identical row.
  - duplicate exam import skips the second identical row.
- Migration tests:
  - fresh migration creates v3 fields.
  - v3 tolerates preexisting columns when a development database already has the columns but not the migration record.
- Schedule tests:
  - odd/even/full week patterns match correctly.
  - weekly response includes both course and exam items for the target week.
  - calendar response includes date-matching exams even when `exam.semester` is empty or differs from the selected course semester.
  - calendar response can show vacation-week tasks/exams without repeating semester courses outside their week pattern.

### 7. Wrong vs Correct

#### Wrong

```rust
// Loses structured week data and makes weekly filtering fragile.
location: format!("{} {}", week_pattern, location)
```

#### Correct

```rust
CreateCourseRequest {
    week_pattern,
    semester_start_date,
    location,
    // ...
}
```

#### Wrong

```rust
// This splits the date in "2026-07-06 16:00-18:00".
let (start, end) = raw.split_once('-').unwrap();
```

#### Correct

```rust
let start_raw = raw.get(..16).ok_or_else(|| format!("无法解析考试时间字段: {raw}"))?;
let end_raw = raw.get(16..).unwrap_or_default().trim_start_matches(is_time_separator);
```

---

## Scenario: LZU API Schedule Import

### 1. Scope / Trigger

- Trigger: backend commands that import courses from LZU AppService APIs instead of clipboard text.
- Applies to Rust backend files under `src-tauri/src/lzu/` and `src-tauri/src/commands/lzu.rs`.

### 2. Contracts

- Treat third-party API fields as optional unless the import mapper cannot proceed without them.
- `getXlxx.data.zzx` is not guaranteed to exist. It must not be required before importing schedules.
- If `zzx` is present, use it as the authoritative total teaching-week count.
- If `zzx` is absent, fall back to a conservative maximum week range and continue importing.
- If fallback mode has already collected schedule rows and a later week returns a business error, stop fetching later weeks instead of failing the whole import.
- Course duplicate handling must reuse the existing course import deduplication helper so repeated LZU imports report skipped rows instead of inserting duplicates.

### 3. Error And Logging Rules

- Missing `zzx` is a recoverable compatibility case. Log it as `warn!`, not `error!`.
- Do not include LZU tokens, headers, encrypted payloads, or raw credentials in frontend errors or logs.
- Token-bearing query endpoints such as `getSt` must map transport errors to sanitized messages before returning them through Tauri commands.

### 4. Tests Required

- Unit tests must cover:
  - `zzx` present -> use `zzx`.
  - `zzx` absent -> fallback week limit.
  - fallback week limit keeps a larger `dqrqszzc` value when needed.
  - invalid `zzx` values are rejected.
