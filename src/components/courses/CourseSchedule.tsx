import { useEffect, useRef, useState } from "react"
import { invoke } from "@tauri-apps/api/core"
import { readText } from "@tauri-apps/plugin-clipboard-manager"
import type { Course, CreateCourseRequest, UpdateCourseRequest, CourseFilterParams } from "@/types/course"
import type { ImportTextResult } from "@/types/course-import"
import type { WeekScheduleItem, WeekScheduleResponse } from "@/types/schedule"
import { Button } from "@/components/ui/button"
import { AcrylicPanel } from "@/components/shared/acrylic-panel"
import { Modal } from "@/components/shared/modal"
import { cn } from "@/lib/utils"
import {
  Plus,
  Pencil,
  Trash2,
  ArrowLeft,
  ChevronLeft,
  ChevronRight,
  GraduationCap,
  EllipsisVertical,
  Save,
} from "lucide-react"
import {
  DAY_LABELS,
  DAY_SHORT,
  HOUR_HEIGHT,
  START_HOUR,
  TOTAL_HOURS,
  type CourseFormData,
  emptyForm,
  courseToForm,
  timeToMinutes,
  minutesToTop,
  isDarkColor,
  hexToRgba,
  todayKey,
  computeCurrentWeek,
  dayCellKey,
  FIELD_CLASS,
} from "./utils"
import { CourseFormModal } from "./CourseFormModal"
import { ImportModal } from "./ImportModal"

export function CourseSchedule({ onNavigate }: { onNavigate: (key: string) => void }) {
  const [courses, setCourses] = useState<Course[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [semesterFilter, setSemesterFilter] = useState(() => {
    const now = new Date()
    const y = now.getFullYear()
    // Feb–Jul → S1 (春季), Aug–Jan → S2 (秋季)
    return now.getMonth() >= 1 && now.getMonth() <= 6 ? `${y}S1` : `${y}S2`
  })

  const [viewMode, setViewMode] = useState<"week" | "list">("week")
  const [weekIndex, setWeekIndex] = useState(1)
  const [weekData, setWeekData] = useState<WeekScheduleResponse | null>(null)
  const [weekLoading, setWeekLoading] = useState(true)
  const [weekError, setWeekError] = useState<string | null>(null)
  const [weekRefresh, setWeekRefresh] = useState(0)
  const [mobileDayIndex, setMobileDayIndex] = useState(new Date().getDay() === 0 ? 6 : new Date().getDay() - 1)
  const didAutoJumpRef = useRef(false)

  const [showForm, setShowForm] = useState(false)
  const [editingCourse, setEditingCourse] = useState<Course | null>(null)
  const [form, setForm] = useState<CourseFormData>(emptyForm())
  const [saving, setSaving] = useState(false)

  const [showImport, setShowImport] = useState(false)
  const [importText, setImportText] = useState("")
  const [importSemester, setImportSemester] = useState("")
  const [importStartDate, setImportStartDate] = useState("")
  const [importing, setImporting] = useState(false)
  const [importError, setImportError] = useState<string | null>(null)
  const [importFeedback, setImportFeedback] = useState<ImportTextResult | null>(null)

  const [showWeekPicker, setShowWeekPicker] = useState(false)
  const [showOverflowMenu, setShowOverflowMenu] = useState(false)
  const [showResetDate, setShowResetDate] = useState(false)
  const [resetDate, setResetDate] = useState("")
  const [resetting, setResetting] = useState(false)
  const [showDetail, setShowDetail] = useState(false)
  const [detailCourse, setDetailCourse] = useState<Course | null>(null)
  const weekPickerRef = useRef<HTMLDivElement>(null)
  const overflowMenuRef = useRef<HTMLDivElement>(null)

  // Close week picker on outside click
  useEffect(() => {
    if (!showWeekPicker) return
    function handleClick(e: MouseEvent) {
      if (weekPickerRef.current && !weekPickerRef.current.contains(e.target as Node)) {
        setShowWeekPicker(false)
      }
    }
    document.addEventListener("mousedown", handleClick)
    return () => document.removeEventListener("mousedown", handleClick)
  }, [showWeekPicker])

  // Close overflow menu on outside click
  useEffect(() => {
    if (!showOverflowMenu) return
    function handleClick(e: MouseEvent) {
      if (overflowMenuRef.current && !overflowMenuRef.current.contains(e.target as Node)) {
        setShowOverflowMenu(false)
      }
    }
    document.addEventListener("mousedown", handleClick)
    return () => document.removeEventListener("mousedown", handleClick)
  }, [showOverflowMenu])

  async function getCourses(targetSemester: string): Promise<Course[]> {
    const filters: CourseFilterParams = {}
    if (targetSemester) filters.semester = targetSemester
    return invoke<Course[]>("get_all_courses", { filters })
  }

  async function fetchCourses(targetSemester = semesterFilter) {
    const result = await getCourses(targetSemester)
    setCourses(result)
    setError(null)
  }

  useEffect(() => {
    let cancelled = false
    async function load() {
      try {
        const result = await getCourses(semesterFilter)
        if (!cancelled) {
          setCourses(result)
          setError(null)
        }
      } catch {
        if (!cancelled) setError("Tauri 不可用 — 展示离线 UI")
      } finally {
        if (!cancelled) setLoading(false)
      }
    }
    void load()
    return () => {
      cancelled = true
    }
  }, [semesterFilter])

  // 切换学期后允许重新自动跳转到当前周
  useEffect(() => {
    didAutoJumpRef.current = false
  }, [semesterFilter])

  // 周视图数据
  useEffect(() => {
    if (viewMode !== "week") return
    let cancelled = false
    async function loadWeek() {
      setWeekLoading(true)
      setWeekError(null)
      try {
        const res = await invoke<WeekScheduleResponse>("get_week_schedule", {
          cmd: { semester: semesterFilter, week_index: weekIndex },
        })
        if (cancelled) return
        setWeekData(res)
        if (!didAutoJumpRef.current && res.semester_start_date) {
          didAutoJumpRef.current = true
          const current = computeCurrentWeek(res.semester_start_date)
          if (current !== weekIndex) setWeekIndex(current)
        }
      } catch (e) {
        if (!cancelled) {
          setWeekData(null)
          setWeekError(typeof e === "string" ? e : "无法加载周课表")
        }
      } finally {
        if (!cancelled) setWeekLoading(false)
      }
    }
    void loadWeek()
    return () => {
      cancelled = true
    }
  }, [viewMode, semesterFilter, weekIndex, weekRefresh])

  function refreshAll() {
    void fetchCourses()
    setWeekRefresh((n) => n + 1)
  }

  function openCreateForm() {
    setForm({ ...emptyForm(), semester: semesterFilter })
    setEditingCourse(null)
    setShowForm(true)
  }

  function openEditForm(course: Course) {
    setForm(courseToForm(course))
    setEditingCourse(course)
    setShowForm(true)
  }

  function openImportPanel() {
    setImportError(null)
    setImportFeedback(null)
    setImportSemester(semesterFilter)
    setImportStartDate("")
    setShowImport(true)
  }

  async function handleSave() {
    if (!form.name.trim() || !form.start_time || !form.end_time) return

    setSaving(true)
    try {
      if (editingCourse) {
        const payload: UpdateCourseRequest = {
          name: form.name,
          day_of_week: form.day_of_week,
          start_time: form.start_time,
          end_time: form.end_time,
          week_pattern: form.week_pattern,
          semester_start_date: form.semester_start_date,
          location: form.location,
          teacher: form.teacher,
          color: form.color,
          semester: form.semester,
        }
        await invoke("update_course", { id: editingCourse.id, cmd: payload })
      } else {
        const payload: CreateCourseRequest = {
          name: form.name,
          day_of_week: form.day_of_week,
          start_time: form.start_time,
          end_time: form.end_time,
          week_pattern: form.week_pattern || undefined,
          semester_start_date: form.semester_start_date || undefined,
          location: form.location || undefined,
          teacher: form.teacher || undefined,
          color: form.color || undefined,
          semester: form.semester || undefined,
        }
        await invoke("create_course", { cmd: payload })
      }
      setShowForm(false)
      setEditingCourse(null)
      refreshAll()
    } finally {
      setSaving(false)
    }
  }

  async function handleDelete(id: number) {
    await invoke("delete_course", { id })
    refreshAll()
  }

  async function handleReadClipboard() {
    setImportError(null)
    try {
      // 给 Android 一点时间完成剪贴板权限切换
      await new Promise((r) => setTimeout(r, 150))
      let text: string
      try {
        text = await readText()
      } catch {
        text = await navigator.clipboard.readText()
      }
      if (!text.trim()) {
        setImportError("剪贴板为空，请先从教务系统复制课表表格。")
        return
      }
      setImportText(text)
    } catch {
      setImportError("读取剪贴板失败，请手动粘贴课表文本后再导入。")
    }
  }

  async function handleImport() {
    if (!importText.trim()) {
      setImportError("请先粘贴或读取课表文本。")
      return
    }

    setImporting(true)
    setImportError(null)
    setImportFeedback(null)
    try {
      const result = await invoke<ImportTextResult>("import_courses_from_text", {
        cmd: {
          text: importText,
          semester: importSemester.trim(),
          semester_start_date: importStartDate.trim(),
        },
      })
      setImportFeedback(result)

      const nextSemester = importSemester.trim()
      if (nextSemester && nextSemester !== semesterFilter) {
        setSemesterFilter(nextSemester)
      } else {
        await fetchCourses(nextSemester || semesterFilter)
        setWeekRefresh((n) => n + 1)
      }
    } catch (e) {
      setImportError(typeof e === "string" ? e : "导入失败，请检查课表文本格式。")
    } finally {
      setImporting(false)
    }
  }

  // ── Inline navigation handlers ──
  const handlePrevWeek = () => {
    setWeekIndex((w) => Math.max(1, w - 1))
    setViewMode("week")
  }

  const handleNextWeek = () => {
    setWeekIndex((w) => w + 1)
    setViewMode("week")
  }

  const handleSelectWeek = (w: number) => {
    setWeekIndex(w)
    setShowWeekPicker(false)
    setViewMode("week")
  }

  const handleDayClick = (dayIndex: number, dateKey: string) => {
    setMobileDayIndex(dayIndex)
    setViewMode("week")
    const today = todayKey()
    if (dateKey === today && weekData?.semester_start_date) {
      setWeekIndex(computeCurrentWeek(weekData.semester_start_date))
    }
  }

  const handleBlockClick = (item: WeekScheduleItem) => {
    if (item.kind !== "course") return
    const course = courses.find((c) => c.id === item.id)
    if (course) {
      setDetailCourse(course)
      setShowDetail(true)
    }
  }

  const handleShowAllCourses = () => {
    setViewMode("list")
    setShowOverflowMenu(false)
  }

  async function handleResetDate() {
    if (!resetDate.trim()) return
    setResetting(true)
    try {
      await invoke<number>("reset_all_semester_start_dates", { date: resetDate.trim() })
      setShowResetDate(false)
      setResetDate("")
      refreshAll()
    } catch {
      setWeekError("重置学期起始日失败")
    } finally {
      setResetting(false)
    }
  }

  const today = todayKey()
  const currentWeek = weekData?.semester_start_date
    ? computeCurrentWeek(weekData.semester_start_date)
    : null
  const isCurrentWeek = currentWeek !== null && currentWeek === weekIndex

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      {/* ─── Header: back + week nav + overflow ─── */}
      <div className="flex items-center gap-2 px-3 py-2 shrink-0">
        <Button
          size="icon-sm"
          variant="ghost"
          onClick={() => onNavigate("kairos")}
          aria-label="返回"
        >
          <ArrowLeft className="h-4 w-4" />
        </Button>
        <Button
          size="icon-sm"
          variant="ghost"
          onClick={handlePrevWeek}
          aria-label="上一周"
          className="shrink-0"
        >
          <ChevronLeft className="h-4 w-4" />
        </Button>

        {/* Week dropdown */}
        <div className="relative" ref={weekPickerRef}>
          <button
            type="button"
            onClick={() => setShowWeekPicker(!showWeekPicker)}
            className={cn(
              "inline-flex items-center gap-1 rounded-md px-2.5 py-1 text-sm font-medium transition-colors h-8",
              "text-foreground hover:bg-muted/60",
              showWeekPicker && "bg-muted",
            )}
          >
            <span className="tabular-nums">
              第 {weekIndex} 周
            </span>
            {isCurrentWeek && (
              <span className="text-[11px] text-muted-foreground font-normal">(本周)</span>
            )}
            <span className={cn("text-[10px] text-muted-foreground transition-transform ml-0.5", showWeekPicker && "rotate-180")}>▼</span>
          </button>
          {showWeekPicker && (
            <div className="absolute left-1/2 -translate-x-1/2 top-full mt-1.5 z-50 w-40 max-h-64 overflow-y-auto rounded-lg border border-border bg-card p-1 shadow-lg animate-in fade-in-0 zoom-in-95 origin-top">
              {Array.from({ length: 20 }, (_, i) => i + 1).map((w) => (
                <button
                  key={w}
                  type="button"
                  onClick={() => handleSelectWeek(w)}
                  className={cn(
                    "w-full text-left rounded-md px-3 py-1.5 text-sm transition-colors tabular-nums",
                    w === weekIndex
                      ? "bg-primary text-primary-foreground"
                      : "text-foreground hover:bg-muted/60",
                  )}
                >
                  第 {w} 周
                  {w === currentWeek && w !== weekIndex && (
                    <span className="ml-1 text-[11px] opacity-70">(本周)</span>
                  )}
                </button>
              ))}
            </div>
          )}
        </div>

        <Button
          size="icon-sm"
          variant="ghost"
          onClick={handleNextWeek}
          aria-label="下一周"
          className="shrink-0"
        >
          <ChevronRight className="h-4 w-4" />
        </Button>

        {!isCurrentWeek && currentWeek !== null && (
          <Button
            size="sm"
            variant="ghost"
            onClick={() => {
              setWeekIndex(currentWeek)
              setViewMode("week")
            }}
            className="shrink-0 h-7 text-[11px] font-medium text-primary hover:bg-primary/10"
          >
            今天
          </Button>
        )}

        <div className="flex-1" />

        {/* Overflow menu */}
        <div className="relative" ref={overflowMenuRef}>
          <Button
            size="icon-sm"
            variant="ghost"
            aria-label="更多选项"
            onClick={() => setShowOverflowMenu(!showOverflowMenu)}
            className={cn(showOverflowMenu && "bg-muted")}
          >
            <EllipsisVertical className="h-4 w-4" />
          </Button>
          {showOverflowMenu && (
            <div className="absolute right-0 top-full mt-1.5 z-50 w-44 rounded-lg border border-border bg-card p-2 shadow-lg animate-in fade-in-0 zoom-in-95 origin-top-right">
              <button
                type="button"
                onClick={() => {
                  openImportPanel()
                  setShowOverflowMenu(false)
                }}
                className="w-full text-left rounded-md px-3 py-2 text-sm text-foreground hover:bg-muted/60 transition-colors"
              >
                导入课表
              </button>

              <button
                type="button"
                onClick={() => {
                  setShowOverflowMenu(false)
                  setResetDate(weekData?.semester_start_date ?? "")
                  setShowResetDate(true)
                }}
                className="w-full text-left rounded-md px-3 py-2 text-sm text-foreground hover:bg-muted/60 transition-colors"
              >
                重置学期起始日
              </button>

              <button
                type="button"
                onClick={handleShowAllCourses}
                className={cn(
                  "w-full text-left rounded-md px-3 py-2 text-sm transition-colors",
                  viewMode === "list"
                    ? "bg-primary text-primary-foreground"
                    : "text-foreground hover:bg-muted/60",
                )}
              >
                全部课程
              </button>
            </div>
          )}
        </div>
      </div>

      {/* ─── Date strip (desktop: aligned with 48px + 7fr grid) ─── */}
      <div className="hidden md:block shrink-0 pb-1">
        <div
          className="grid select-none"
          style={{ gridTemplateColumns: "48px repeat(7, minmax(0, 1fr))", minWidth: "680px" }}
        >
          <div />
          {DAY_SHORT.map((day, i) => {
            const dateKey = weekData ? dayCellKey(weekData.week_start_date, i) : ""
            const dateNum = dateKey ? dateKey.slice(8) : ""
            const isToday = dateKey === today
            return (
              <button
                key={i}
                type="button"
                onClick={() => handleDayClick(i, dateKey)}
                className={cn(
                  "flex flex-col items-center justify-center h-12 text-center transition-colors",
                  isToday ? "bg-primary/10 text-primary font-semibold" : "text-foreground hover:bg-muted/40",
                )}
              >
                <span className="text-[10px] leading-tight font-medium">{day}</span>
                <span className="text-xs font-semibold leading-tight tabular-nums mt-0.5">{dateNum || "—"}</span>
              </button>
            )
          })}
        </div>
      </div>

      {/* ─── Date strip (mobile: fit all seven days inside the viewport) ─── */}
      <div className="shrink-0 overflow-x-hidden pb-1 md:hidden">
        <div
          className="grid w-full select-none"
          style={{ gridTemplateColumns: "28px repeat(7, minmax(0, 1fr))" }}
        >
          <div />
          {DAY_SHORT.map((day, i) => {
            const dateKey = weekData ? dayCellKey(weekData.week_start_date, i) : ""
            const dateNum = dateKey ? dateKey.slice(8) : ""
            const isToday = dateKey === today
            return (
              <button
                key={i}
                type="button"
                onClick={() => handleDayClick(i, dateKey)}
                className={cn(
                  "flex flex-col items-center justify-center h-12 text-center transition-colors",
                  isToday ? "bg-primary/10 text-primary font-semibold" : "text-foreground hover:bg-muted/40",
                )}
              >
                <span className="text-[10px] leading-tight font-medium">{day}</span>
                <span className="text-xs font-semibold leading-tight tabular-nums mt-0.5">{dateNum || "—"}</span>
              </button>
            )
          })}
        </div>
      </div>

      {/* ─── Content area ─── */}
      <div className="min-h-0 flex-1 overflow-y-auto overflow-x-hidden pb-0 md:pb-4">
        {/* Week view */}
        {viewMode === "week" && (
          <>
            {weekLoading && (
              <div className="flex items-center justify-center py-12">
                <div className="h-6 w-6 animate-spin rounded-full border-2 border-primary border-t-transparent" />
              </div>
            )}

            {!weekLoading && weekError && !weekData && (
              <div className="p-8 text-center">
                <p className="text-sm text-muted-foreground">{weekError}</p>
                <p className="mt-1 text-xs text-muted-foreground/60">
                  请先导入课表（含学期开始日期），或在课程中填写"学期开始日期"。
                </p>
              </div>
            )}

            {!weekLoading && !weekError && !weekData && (
              <p className="py-8 text-center text-sm text-muted-foreground">
                暂无课表数据
              </p>
            )}

            {!weekLoading && weekData && (
              <>
                {/* Desktop: 7-column grid */}
                <div className="hidden md:block overflow-x-auto">
                  <div
                    className="grid select-none"
                    style={{
                      gridTemplateColumns: "48px repeat(7, minmax(0, 1fr))",
                      minWidth: "680px",
                    }}
                  >
                    {/* Time axis column */}
                    <div className="relative bg-card" style={{ height: `${TOTAL_HOURS * HOUR_HEIGHT}px` }}>
                      {Array.from({ length: TOTAL_HOURS }, (_, i) => {
                        if (i === 0) return <div key={i} style={{ height: `${HOUR_HEIGHT}px` }} />
                        return (
                          <div
                            key={i}
                            className="absolute right-1.5 text-[10px] text-muted-foreground tabular-nums"
                            style={{ top: `${i * HOUR_HEIGHT - 6}px` }}
                          >
                            {`${String(START_HOUR + i).padStart(2, "0")}:00`}
                          </div>
                        )
                      })}
                    </div>

                    {/* 7 day columns */}
                    {Array.from({ length: 7 }, (_, dayIdx) => {
                      const dateKey = weekData ? dayCellKey(weekData.week_start_date, dayIdx) : ""
                      const isTodayCol = dateKey === today
                      const isSelCol = dayIdx === mobileDayIndex
                      const dayItems = (weekData.items ?? []).filter(
                        (it) => it.day_of_week === dayIdx + 1,
                      )
                      return (
                        <div
                          key={dayIdx}
                          className={cn(
                            "relative border-l border-border/25",
                            isTodayCol && "bg-primary/[0.04]",
                            isSelCol && !isTodayCol && "bg-primary/[0.02]",
                          )}
                          style={{ height: `${TOTAL_HOURS * HOUR_HEIGHT}px` }}
                        >
                          {/* Hour grid lines */}
                          {Array.from({ length: TOTAL_HOURS }, (_, i) => (
                            <div
                              key={i}
                              className="absolute inset-x-0 border-t border-border/25"
                              style={{ top: `${i * HOUR_HEIGHT}px` }}
                            />
                          ))}

                          {/* Course / exam blocks */}
                          {dayItems.map((item) => {
                            const startMin = timeToMinutes(item.start_time)
                            const endMin = timeToMinutes(item.end_time)
                            const top = minutesToTop(startMin)
                            const height = Math.max(
                              24,
                              ((endMin - startMin) / 60) * HOUR_HEIGHT - 2,
                            )
                            const isExam = item.kind === "exam"
                            const dark = isDarkColor(item.color)

                            return (
                              <button
                                key={`${item.kind}-${item.id}`}
                                type="button"
                                onClick={() => handleBlockClick(item)}
                                title={`${item.title}${item.location ? ` — ${item.location}` : ""}`}
                                className={cn(
                                  "absolute left-0.5 right-0.5 overflow-hidden rounded-lg px-1 py-px text-left transition-all flex flex-col justify-center",
                                  "hover:z-20 hover:shadow-lg",
                                  isExam
                                    ? "border-2 border-dashed cursor-default"
                                    : "cursor-pointer hover:brightness-105",
                                  dark ? "text-white" : "text-foreground",
                                )}
                                style={{
                                  top: `${top}px`,
                                  height: `${height}px`,
                                  backgroundColor: isExam
                                    ? hexToRgba(item.color, 0.16)
                                    : item.color,
                                  borderColor: isExam ? item.color : undefined,
                                }}
                              >
                                <span className="line-clamp-2 text-[11px] font-semibold leading-tight">
                                  {isExam && <GraduationCap className="inline h-3 w-3 shrink-0 mr-0.5 -mt-px" />}
                                  {item.title}
                                </span>
                                {item.location && height >= 40 && (
                                  <span className="truncate text-[9px] leading-tight opacity-65">
                                    {item.location}
                                  </span>
                                )}
                                {height < 40 && (
                                  <span className="sr-only">{item.title} — {item.location}</span>
                                )}
                              </button>
                            )
                          })}
                        </div>
                      )
                    })}
                  </div>
                </div>

                {/* Mobile: fit all seven days inside the viewport */}
                <div className="overflow-x-hidden md:hidden">
                  <div
                    className="grid w-full select-none"
                    style={{
                      gridTemplateColumns: "28px repeat(7, minmax(0, 1fr))",
                    }}
                  >
                    {/* Time axis column (sticky left) */}
                    <div className="relative bg-card" style={{ height: `${TOTAL_HOURS * HOUR_HEIGHT}px` }}>
                      {Array.from({ length: TOTAL_HOURS }, (_, i) => {
                        if (i === 0) return <div key={i} style={{ height: `${HOUR_HEIGHT}px` }} />
                        return (
                          <div
                            key={i}
                            className="absolute right-1 text-[9px] text-muted-foreground tabular-nums"
                            style={{ top: `${i * HOUR_HEIGHT - 5}px` }}
                          >
                            {`${String(START_HOUR + i).padStart(2, "0")}`}
                          </div>
                        )
                      })}
                    </div>

                    {/* 7 day columns */}
                    {Array.from({ length: 7 }, (_, dayIdx) => {
                      const dateKey = weekData ? dayCellKey(weekData.week_start_date, dayIdx) : ""
                      const isTodayCol = dateKey === today
                      const isSelCol = dayIdx === mobileDayIndex
                      const dayItems = (weekData.items ?? []).filter((it) => it.day_of_week === dayIdx + 1)
                      return (
                        <div
                          key={dayIdx}
                          className={cn(
                            "relative border-l border-border/25",
                            isTodayCol && "bg-primary/[0.06]",
                            isSelCol && "bg-primary/[0.03]",
                          )}
                          style={{ height: `${TOTAL_HOURS * HOUR_HEIGHT}px` }}
                        >
                          {/* Hour grid lines */}
                          {Array.from({ length: TOTAL_HOURS }, (_, i) => (
                            <div
                              key={i}
                              className="absolute inset-x-0 border-t border-border/25"
                              style={{ top: `${i * HOUR_HEIGHT}px` }}
                            />
                          ))}

                          {/* Course / exam blocks */}
                          {dayItems.map((item) => {
                            const startMin = timeToMinutes(item.start_time)
                            const endMin = timeToMinutes(item.end_time)
                            const top = minutesToTop(startMin)
                            const height = Math.max(26, ((endMin - startMin) / 60) * HOUR_HEIGHT - 2)
                            const isExam = item.kind === "exam"
                            const dark = isDarkColor(item.color)

                            return (
                              <button
                                key={`m-${item.kind}-${item.id}`}
                                type="button"
                                onClick={() => handleBlockClick(item)}
                                title={item.title}
                                className={cn(
                                  "absolute left-px right-px overflow-hidden rounded-lg px-1 py-px text-left transition-all active:brightness-90 flex items-center",
                                  isExam ? "border border-dashed" : "",
                                  dark ? "text-white" : "text-foreground",
                                )}
                                style={{
                                  top: `${top}px`,
                                  height: `${height}px`,
                                  backgroundColor: isExam ? hexToRgba(item.color, 0.16) : item.color,
                                  borderColor: isExam ? item.color : undefined,
                                }}
                              >
                                <span className="line-clamp-2 text-[10px] font-semibold leading-tight w-full">
                                  {item.title}
                                </span>
                              </button>
                            )
                          })}
                        </div>
                      )
                    })}
                  </div>
                </div>
              </>
            )}
          </>
        )}

        {/* List view (全部课程) */}
        {viewMode === "list" && (
          <>
            {loading && (
              <div className="flex items-center justify-center py-12">
                <div className="h-6 w-6 animate-spin rounded-full border-2 border-primary border-t-transparent" />
              </div>
            )}

            {!loading && error && courses.length === 0 && (
              <p className="py-8 text-center text-sm text-muted-foreground">{error}</p>
            )}

            {!loading && courses.length === 0 && !error && (
              <div className="p-8 text-center">
                <p className="text-sm text-muted-foreground">暂无课程</p>
                <p className="mt-1 text-xs text-muted-foreground/60">
                  点击右下角 + 按钮创建第一个课程
                </p>
              </div>
            )}

            {!loading && courses.length > 0 && (
              <div className="space-y-2 p-2">
                {courses.map((course) => (
                  <AcrylicPanel
                    key={course.id}
                    className="flex items-center gap-3 bg-card p-3 transition-all hover:-translate-y-0.5 hover:bg-card/95 hover:shadow-md"
                  >
                    <div
                      className="h-3 w-3 shrink-0 rounded-full"
                      style={{ backgroundColor: course.color }}
                    />
                    <div className="min-w-0 flex-1">
                      <div className="flex items-center gap-2">
                        <span className="truncate text-sm font-semibold text-foreground">
                          {course.name}
                        </span>
                        <span className="shrink-0 text-[10px] text-muted-foreground">
                          {DAY_LABELS[course.day_of_week - 1]} {course.start_time}-
                          {course.end_time}
                        </span>
                        {course.week_pattern && (
                          <span className="shrink-0 text-[10px] text-muted-foreground/60">
                            {course.week_pattern}
                          </span>
                        )}
                      </div>
                      <div className="mt-0.5 flex items-center gap-2">
                        {course.teacher && (
                          <span className="text-xs text-muted-foreground">
                            {course.teacher}
                          </span>
                        )}
                        {course.location && (
                          <span className="text-xs text-muted-foreground/60">
                            {course.location}
                          </span>
                        )}
                        {course.semester && (
                          <span className="text-[10px] text-muted-foreground/50">
                            {course.semester}
                          </span>
                        )}
                      </div>
                    </div>
                    <div className="flex shrink-0 items-center gap-1">
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-9 w-9 sm:h-7 sm:w-7 text-muted-foreground hover:text-foreground"
                        aria-label={`编辑课程 ${course.name}`}
                        onClick={() => openEditForm(course)}
                      >
                        <Pencil className="h-4 w-4 sm:h-3.5 sm:w-3.5" />
                      </Button>
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-9 w-9 sm:h-7 sm:w-7 text-muted-foreground hover:text-destructive"
                        aria-label={`删除课程 ${course.name}`}
                        onClick={() => void handleDelete(course.id)}
                      >
                        <Trash2 className="h-4 w-4 sm:h-3.5 sm:w-3.5" />
                      </Button>
                    </div>
                  </AcrylicPanel>
                ))}
              </div>
            )}
          </>
        )}
      </div>

      {/* ─── FAB: Add course ─── */}
      <button
        type="button"
        onClick={openCreateForm}
        aria-label="添加课程"
        className={cn(
          "fixed bottom-20 md:bottom-6 right-4 md:right-6 z-40",
          "h-14 w-14 rounded-full",
          "bg-primary text-primary-foreground",
          "shadow-lg shadow-primary/25",
          "flex items-center justify-center",
          "transition-transform active:scale-95 hover:scale-105",
        )}
      >
        <Plus className="h-5 w-5" />
      </button>

      {/* ─── Course Detail Modal ─── */}
      <Modal
        open={showDetail}
        onOpenChange={setShowDetail}
        title="课程详情"
        className="max-w-sm"
      >
        {detailCourse && (
          <div className="space-y-4">
            <div className="flex items-center gap-3">
              <div
                className="h-4 w-4 shrink-0 rounded"
                style={{ backgroundColor: detailCourse.color }}
              />
              <h3 className="text-lg font-semibold text-foreground">{detailCourse.name}</h3>
            </div>
            <div className="grid grid-cols-2 gap-3 text-sm">
              <div>
                <div className="text-xs text-muted-foreground">时间</div>
                <div className="mt-0.5 font-medium tabular-nums">
                  {DAY_LABELS[detailCourse.day_of_week - 1]} {detailCourse.start_time}-{detailCourse.end_time}
                </div>
              </div>
              {detailCourse.location && (
                <div>
                  <div className="text-xs text-muted-foreground">地点</div>
                  <div className="mt-0.5 font-medium">{detailCourse.location}</div>
                </div>
              )}
              {detailCourse.teacher && (
                <div>
                  <div className="text-xs text-muted-foreground">教师</div>
                  <div className="mt-0.5 font-medium">{detailCourse.teacher}</div>
                </div>
              )}
              <div>
                <div className="text-xs text-muted-foreground">学期</div>
                <div className="mt-0.5 font-medium">{detailCourse.semester || "—"}</div>
              </div>
              <div className="col-span-2">
                <div className="text-xs text-muted-foreground">周次</div>
                <div className="mt-0.5 font-medium">{detailCourse.week_pattern || "每 周"}</div>
              </div>
            </div>
            <div className="flex items-center gap-2 pt-2 border-t border-border/50">
              <Button
                size="sm"
                onClick={() => {
                  setShowDetail(false)
                  openEditForm(detailCourse)
                }}
              >
                <Pencil className="mr-1.5 h-3.5 w-3.5" />
                编辑
              </Button>
              <Button
                variant="ghost"
                size="sm"
                className="text-destructive hover:bg-destructive/10"
                onClick={() => {
                  setShowDetail(false)
                  handleDelete(detailCourse.id)
                }}
              >
                <Trash2 className="mr-1.5 h-3.5 w-3.5" />
                删除
              </Button>
            </div>
          </div>
        )}
      </Modal>

      {/* ─── Course Form Modal ─── */}
      <CourseFormModal
        open={showForm}
        onOpenChange={(open) => {
          setShowForm(open)
          if (!open) setEditingCourse(null)
        }}
        isEditing={!!editingCourse}
        form={form}
        setForm={setForm}
        saving={saving}
        onSave={handleSave}
      />

      {/* ─── Import Modal ─── */}
      <ImportModal
        open={showImport}
        onOpenChange={(open) => {
          setShowImport(open)
          if (!open) {
            setImportError(null)
            setImportFeedback(null)
          }
        }}
        importText={importText}
        setImportText={setImportText}
        importSemester={importSemester}
        setImportSemester={setImportSemester}
        importStartDate={importStartDate}
        setImportStartDate={setImportStartDate}
        importing={importing}
        importError={importError}
        importFeedback={importFeedback}
        onReadClipboard={handleReadClipboard}
        onImport={handleImport}
      />

      {/* ─── Reset Semester Start Date Modal ─── */}
      <Modal
        open={showResetDate}
        onOpenChange={(open) => {
          setShowResetDate(open)
          if (!open) setResetDate("")
        }}
        title="重置学期起始日"
        description="将所有课程和考试的学期开始日期统一设置为指定日期，用于修正周课表计算。"
        className="max-w-sm"
      >
        <div className="space-y-3">
          <div>
            <label className="mb-1 block text-xs font-medium text-muted-foreground">
              学期开始日期 <span className="text-destructive">*</span>
            </label>
            <input
              type="date"
              value={resetDate}
              onChange={(e) => setResetDate(e.target.value)}
              className={FIELD_CLASS}
            />
          </div>
        </div>

        <div className="mt-5 flex items-center justify-end gap-2">
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={() => setShowResetDate(false)}
          >
            取消
          </Button>
          <Button
            type="button"
            size="sm"
            disabled={resetting || !resetDate.trim()}
            onClick={() => void handleResetDate()}
          >
            <Save className="mr-1 h-3.5 w-3.5" />
            {resetting ? "保存中..." : "确认重置"}
          </Button>
        </div>
      </Modal>
    </div>
  )
}
