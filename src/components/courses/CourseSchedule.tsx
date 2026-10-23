import { useEffect, useRef, useState } from "react"
import type { Course, CreateCourseRequest, UpdateCourseRequest, CourseFilterParams } from "@/types/course"
import {
  listCourses,
  getWeekSchedule,
  createCourse,
  updateCourse,
  deleteCourse,
  importCoursesFromText,
  resetSemesterDates,
} from "@/lib/api/courses"
import type { ImportTextResult } from "@/types/course-import"
import type { WeekScheduleItem, WeekScheduleResponse } from "@/types/schedule"
import { Button } from "@/components/ui/button"
import { AcrylicPanel } from "@/components/shared/acrylic-panel"
import { Modal } from "@/components/shared/modal"
import { cn } from "@/lib/utils"
import { userErrorMessage } from "@/lib/errors"
import { formatSemester } from "@/lib/semester"
import {
  Plus,
  Pencil,
  Trash2,
  ArrowLeft,
  Check,
  ChevronDown,
  ChevronLeft,
  ChevronRight,
  EllipsisVertical,
  BookOpen,
  CalendarClock,
  CalendarDays,
  MapPin,
  Save,
} from "lucide-react"
import {
  DAY_LABELS,
  DAY_SHORT,
  type CourseFormData,
  emptyForm,
  courseToForm,
  todayKey,
  computeCurrentWeek,
  dayCellKey,
  formatWeekRange,
  FIELD_CLASS,
} from "./utils"
import { WeekGrid } from "./WeekGrid"
import { CourseFormModal } from "./CourseFormModal"
import { ImportModal } from "./ImportModal"
import { CourseDetailModal } from "./CourseDetailModal"

// 导入入口暂时隐藏：现有解析器面向本科教务 TSV，与研究生系统不兼容；该功能后续可能弃用。恢复/适配后改回 true 即可。
const ENABLE_COURSE_IMPORT = false

const PHASE_LABELS: Record<WeekScheduleResponse["phase_type"], string> = {
  unknown: "未识别阶段",
  teaching: "教学周",
  exam: "考试周",
  break: "假期",
}

export function CourseSchedule({ onNavigate }: { onNavigate: (key: string) => void }) {
  const [courses, setCourses] = useState<Course[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [actionError, setActionError] = useState<string | null>(null)
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
    return listCourses(filters)
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
        const res = await getWeekSchedule({
          semester: semesterFilter,
          week_index: weekIndex,
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
      .then(() => setActionError(null))
      .catch((e) => setActionError(userErrorMessage(e, "刷新课程列表失败")))
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
        await updateCourse(editingCourse.id, payload)
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
        await createCourse(payload)
      }
      setShowForm(false)
      setEditingCourse(null)
      refreshAll()
      setActionError(null)
    } catch (e) {
      setActionError(userErrorMessage(e, "保存课程失败"))
    } finally {
      setSaving(false)
    }
  }

  async function handleDelete(id: number) {
    try {
      await deleteCourse(id)
      refreshAll()
    } catch (e) {
      setActionError(userErrorMessage(e, "删除课程失败"))
    }
  }

  async function handleReadClipboard() {
    setImportError(null)
    try {
      // 给 Android 一点时间完成剪贴板权限切换
      await new Promise((r) => setTimeout(r, 150))
      const text = await navigator.clipboard.readText()
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
      const result = await importCoursesFromText({
        text: importText,
        semester: importSemester.trim(),
        semester_start_date: importStartDate.trim(),
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
      await resetSemesterDates(resetDate.trim())
      setShowResetDate(false)
      setResetDate("")
      refreshAll()
    } catch {
      setActionError("重置学期起始日失败")
    } finally {
      setResetting(false)
    }
  }

  const today = todayKey()
  const currentWeek = weekData?.semester_start_date
    ? computeCurrentWeek(weekData.semester_start_date)
    : null
  const isCurrentWeek = currentWeek !== null && currentWeek === weekIndex
  const weekRangeLabel = weekData && weekData.week_index === weekIndex
    ? formatWeekRange(weekData.week_start_date, weekData.week_end_date)
    : ""
  const weekMonthLabel =
    weekData && weekData.week_index === weekIndex && weekData.week_start_date
      ? `${Number(weekData.week_start_date.slice(5, 7))}月`
      : ""
  const semesterLabel = semesterFilter
    ? `${semesterFilter.slice(0, 4)} ${formatSemester(semesterFilter)}`
    : ""

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      {/* ─── Header: back + week nav + overflow ─── */}
      <div className="flex shrink-0 items-center gap-1.5 px-1.5 pb-2 pt-0.5 md:gap-2 md:px-2 md:pt-1">
        <Button
          size="icon"
          variant="ghost"
          onClick={() => onNavigate("kairos")}
          aria-label="返回"
          className="h-11 w-11 shrink-0 rounded-xl text-muted-foreground hover:text-foreground md:h-8 md:w-8"
        >
          <ArrowLeft className="h-4 w-4" />
        </Button>

        {/* Week navigation cluster */}
        <div className="relative z-50 flex items-center rounded-xl border border-border/50 bg-card/60 p-0.5 shadow-sm backdrop-blur-md">
          <Button
            size="icon"
            variant="ghost"
            onClick={handlePrevWeek}
            aria-label="上一周"
            className="h-11 w-11 shrink-0 rounded-lg text-muted-foreground hover:text-foreground md:h-9 md:w-9"
          >
            <ChevronLeft className="h-4 w-4" />
          </Button>

          <div className="relative" ref={weekPickerRef}>
            <button
              type="button"
              onClick={() => setShowWeekPicker(!showWeekPicker)}
              aria-expanded={showWeekPicker}
              aria-label={`选择周次，当前第 ${weekIndex} 周`}
              className={cn(
                "flex h-11 min-w-[88px] flex-col items-center justify-center rounded-lg px-2 transition-colors md:h-9 md:min-w-[104px]",
                "hover:bg-muted/60",
                showWeekPicker && "bg-muted/70",
              )}
            >
              <span className="flex items-center gap-1">
                <span className="text-sm font-semibold leading-none text-foreground tabular-nums">
                  第 {weekIndex} 周
                </span>
                <ChevronDown
                  className={cn(
                    "h-3 w-3 text-muted-foreground transition-transform",
                    showWeekPicker && "rotate-180",
                  )}
                  aria-hidden="true"
                />
              </span>
              <span className="mt-0.5 text-[10px] leading-none text-muted-foreground tabular-nums">
                {weekLoading ? "加载中…" : weekRangeLabel || "—"}
              </span>
            </button>
            {showWeekPicker && (
              <div className="absolute left-1/2 top-full z-50 mt-2 w-44 -translate-x-1/2 overflow-hidden rounded-xl border border-border/60 bg-popover p-1 shadow-xl backdrop-blur-xl animate-in fade-in-0 zoom-in-95 origin-top">
                <div className="max-h-64 overflow-y-auto">
                  {Array.from({ length: 20 }, (_, i) => i + 1).map((w) => {
                    const isSelected = w === weekIndex
                    return (
                      <button
                        key={w}
                        type="button"
                        onClick={() => handleSelectWeek(w)}
                        className={cn(
                          "flex w-full items-center justify-between gap-2 rounded-lg px-2.5 py-2 text-left text-[13px] transition-colors tabular-nums",
                          isSelected
                            ? "bg-primary/10 font-semibold text-primary"
                            : "text-foreground hover:bg-muted/60",
                        )}
                      >
                        <span>第 {w} 周</span>
                        {w === currentWeek ? (
                          <span className="text-[10px] font-normal text-muted-foreground">本周</span>
                        ) : isSelected ? (
                          <Check className="h-3.5 w-3.5" />
                        ) : null}
                      </button>
                    )
                  })}
                </div>
              </div>
            )}
          </div>

          <Button
            size="icon"
            variant="ghost"
            onClick={handleNextWeek}
            aria-label="下一周"
            className="h-11 w-11 shrink-0 rounded-lg text-muted-foreground hover:text-foreground md:h-9 md:w-9"
          >
            <ChevronRight className="h-4 w-4" />
          </Button>
        </div>

        {!isCurrentWeek && currentWeek !== null && (
          <Button
            size="sm"
            variant="ghost"
            onClick={() => {
              setWeekIndex(currentWeek)
              setViewMode("week")
            }}
            className="h-11 shrink-0 rounded-lg px-2.5 text-[11px] font-medium text-primary hover:bg-primary/10 md:h-9"
          >
            今天
          </Button>
        )}

        <div className="flex-1" />

        <span className="hidden text-[10px] font-medium text-muted-foreground/70 tabular-nums lg:inline">
          {semesterLabel}
        </span>

        {/* Overflow menu */}
        <div className="relative" ref={overflowMenuRef}>
          <Button
            size="icon"
            variant="ghost"
            aria-label="更多选项"
            aria-expanded={showOverflowMenu}
            onClick={() => setShowOverflowMenu(!showOverflowMenu)}
            className={cn(
              "h-11 w-11 rounded-xl text-muted-foreground hover:text-foreground md:h-8 md:w-8",
              showOverflowMenu && "bg-muted",
            )}
          >
            <EllipsisVertical className="h-4 w-4" />
          </Button>
          {showOverflowMenu && (
            <div className="absolute right-0 top-full z-50 mt-2 w-48 overflow-hidden rounded-xl border border-border/60 bg-popover p-1 shadow-xl backdrop-blur-xl animate-in fade-in-0 zoom-in-95 origin-top-right">
              {ENABLE_COURSE_IMPORT && (
                <button
                  type="button"
                  onClick={() => {
                    openImportPanel()
                    setShowOverflowMenu(false)
                  }}
                  className="flex w-full items-center gap-2 rounded-lg px-2.5 py-2 text-left text-[13px] text-foreground transition-colors hover:bg-muted/60"
                >
                  <BookOpen className="h-3.5 w-3.5 text-muted-foreground" />
                  导入课表
                </button>
              )}

              <button
                type="button"
                onClick={() => {
                  setShowOverflowMenu(false)
                  setResetDate(weekData?.semester_start_date ?? "")
                  setShowResetDate(true)
                }}
                className="flex w-full items-center gap-2 rounded-lg px-2.5 py-2 text-left text-[13px] text-foreground transition-colors hover:bg-muted/60"
              >
                <CalendarClock className="h-3.5 w-3.5 text-muted-foreground" />
                重置学期起始日
              </button>

              {viewMode === "list" && (
                <button
                  type="button"
                  onClick={() => {
                    setViewMode("week")
                    setShowOverflowMenu(false)
                  }}
                  className="flex w-full items-center gap-2 rounded-lg px-2.5 py-2 text-left text-[13px] text-foreground transition-colors hover:bg-muted/60"
                >
                  <CalendarDays className="h-3.5 w-3.5 text-muted-foreground" />
                  周视图
                </button>
              )}

              <button
                type="button"
                onClick={handleShowAllCourses}
                className={cn(
                  "flex w-full items-center gap-2 rounded-lg px-2.5 py-2 text-left text-[13px] transition-colors",
                  viewMode === "list"
                    ? "bg-primary/10 font-medium text-primary"
                    : "text-foreground hover:bg-muted/60",
                )}
              >
                <BookOpen className="h-3.5 w-3.5" />
                全部课程
                {viewMode === "list" && <Check className="ml-auto h-3.5 w-3.5" />}
              </button>
            </div>
          )}
        </div>
      </div>

      {actionError && (
        <p className="px-3 pb-2 text-center text-sm text-destructive">
          {actionError}
        </p>
      )}

      {/* ─── Content area ─── */}
      <div className="min-h-0 flex-1 overflow-y-auto overflow-x-hidden pb-0 md:pb-4">
        {/* Week view */}
        {viewMode === "week" && (
          <>
            {weekData && (
              <div className="sticky top-0 z-40 bg-card/75 pb-1.5 backdrop-blur-xl">
                <div className="grid select-none grid-cols-[28px_repeat(7,minmax(0,1fr))] md:grid-cols-[48px_repeat(7,minmax(0,1fr))]">
                  <div className="flex items-start justify-center pt-2.5">
                    <span className="hidden text-[9px] font-medium leading-none text-muted-foreground/60 tabular-nums md:inline">
                      {weekMonthLabel}
                    </span>
                  </div>
                  {DAY_SHORT.map((day, i) => {
                    const dateKey = weekData ? dayCellKey(weekData.week_start_date, i) : ""
                    const dateNum = dateKey ? dateKey.slice(8) : ""
                    const isToday = dateKey === today
                    const isWeekend = i >= 5
                    return (
                      <button
                        key={i}
                        type="button"
                        onClick={() => handleDayClick(i, dateKey)}
                        className={cn(
                          "flex h-14 min-w-0 flex-col items-center justify-center rounded-xl text-center transition-colors",
                          isToday ? "text-primary" : "text-foreground hover:bg-muted/40",
                        )}
                      >
                        <span
                          className={cn(
                            "text-[10px] font-medium leading-none",
                            isToday
                              ? "text-primary"
                              : isWeekend
                                ? "text-muted-foreground/60"
                                : "text-muted-foreground",
                          )}
                        >
                          <span className="md:hidden">{day}</span>
                          <span className="hidden md:inline">周{day}</span>
                        </span>
                        <span
                          className={cn(
                            "mt-1.5 flex h-7 min-w-7 items-center justify-center rounded-full px-1.5 text-[15px] font-semibold leading-none tabular-nums",
                            isToday
                              ? "bg-primary text-primary-foreground shadow-sm shadow-primary/30"
                              : isWeekend
                                ? "text-muted-foreground"
                                : "text-foreground",
                          )}
                        >
                          {dateNum || "—"}
                        </span>
                      </button>
                    )
                  })}
                </div>
              </div>
            )}

            {weekLoading && !weekData && (
              <div className="flex items-center justify-center py-12">
                <div className="h-6 w-6 animate-spin rounded-full border-2 border-primary border-t-transparent" />
              </div>
            )}

            {!weekLoading && weekError && !weekData && (
              <div className="flex min-h-[280px] items-center justify-center p-6">
                <div className="flex w-full max-w-sm flex-col items-center rounded-2xl border border-dashed border-border/60 bg-card/50 px-6 py-8 text-center">
                  <span className="flex h-11 w-11 items-center justify-center rounded-full bg-muted text-muted-foreground">
                    <BookOpen className="h-5 w-5" />
                  </span>
                  <p className="mt-3 text-sm font-medium text-foreground">无法加载课表</p>
                  <p className="mt-1 text-xs text-muted-foreground">{weekError}</p>
                  <p className="mt-1 text-[11px] leading-relaxed text-muted-foreground/60">
                    请先设置学期开始日期，或添加课程后再试。
                  </p>
                  <Button
                    variant="outline"
                    size="sm"
                    className="mt-4"
                    onClick={() => setWeekRefresh((n) => n + 1)}
                  >
                    重试
                  </Button>
                </div>
              </div>
            )}

            {!weekLoading && !weekError && !weekData && (
              <div className="flex min-h-[240px] items-center justify-center p-6 text-center">
                <div className="flex flex-col items-center">
                  <span className="flex h-11 w-11 items-center justify-center rounded-full bg-muted text-muted-foreground">
                    <BookOpen className="h-5 w-5" />
                  </span>
                  <p className="mt-3 text-sm font-medium text-foreground">暂无课表数据</p>
                  <p className="mt-1 text-xs text-muted-foreground">
                    添加课程并设置学期开始日期后，这里会显示每周安排。
                  </p>
                </div>
              </div>
            )}

            {!weekLoading && weekData && !weekData.courses_visible && (
              <div className="flex min-h-[280px] items-center justify-center p-6">
                <AcrylicPanel className="w-full max-w-sm p-6 text-center">
                  <span className="mx-auto flex h-11 w-11 items-center justify-center rounded-full bg-primary/10 text-primary">
                    <CalendarClock className="h-5 w-5" />
                  </span>
                  <p className="mt-3 text-sm font-semibold text-foreground">
                    {PHASE_LABELS[weekData.phase_type]}
                  </p>
                  <p className="mt-1 text-xs text-muted-foreground">
                    当前阶段不显示课程安排
                  </p>
                  <p className="mt-3 inline-flex rounded-full bg-muted px-2.5 py-1 text-[10px] text-muted-foreground/80 tabular-nums">
                    第 {weekData.week_index} 周 · {weekRangeLabel}
                  </p>
                </AcrylicPanel>
              </div>
            )}

            {weekData && weekData.courses_visible && (
              <div
                className={cn(
                  "transition-opacity duration-200",
                  weekLoading && "opacity-50",
                )}
                aria-busy={weekLoading}
              >
                <WeekGrid
                  weekData={weekData}
                  today={today}
                  mobileDayIndex={mobileDayIndex}
                  onBlockClick={handleBlockClick}
                />
              </div>
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
              <div className="flex min-h-[240px] items-center justify-center p-6 text-center">
                <div className="flex flex-col items-center">
                  <span className="flex h-11 w-11 items-center justify-center rounded-full bg-muted text-muted-foreground">
                    <BookOpen className="h-5 w-5" />
                  </span>
                  <p className="mt-3 text-sm font-medium text-foreground">无法加载课程</p>
                  <p className="mt-1 text-xs text-muted-foreground">{error}</p>
                  <Button variant="outline" size="sm" className="mt-4" onClick={refreshAll}>
                    重试
                  </Button>
                </div>
              </div>
            )}

            {!loading && courses.length === 0 && !error && (
              <div className="flex min-h-[240px] items-center justify-center p-6 text-center">
                <div className="flex flex-col items-center">
                  <span className="flex h-11 w-11 items-center justify-center rounded-full bg-muted text-muted-foreground">
                    <BookOpen className="h-5 w-5" />
                  </span>
                  <p className="mt-3 text-sm font-medium text-foreground">暂无课程</p>
                  <p className="mt-1 text-xs text-muted-foreground">
                    点击右下角 + 按钮创建第一门课程。
                  </p>
                </div>
              </div>
            )}

            {!loading && courses.length > 0 && (
              <>
                <div className="flex items-baseline justify-between gap-3 px-2.5 pb-1.5 pt-2">
                  <h3 className="text-[13px] font-semibold text-foreground">全部课程</h3>
                  <span className="shrink-0 text-[10px] text-muted-foreground/70 tabular-nums">
                    {courses.length} 门 · {semesterLabel}
                  </span>
                </div>
                <div className="space-y-2 p-1.5 md:p-2">
                  {courses.map((course) => (
                    <AcrylicPanel
                      key={course.id}
                      className="relative flex items-center gap-3 overflow-hidden bg-card/70 p-3 pl-4 transition-all hover:-translate-y-0.5 hover:bg-card hover:shadow-md"
                    >
                      <span
                        className="absolute inset-y-2.5 left-0 w-[3px] rounded-r-full"
                        style={{ backgroundColor: course.color }}
                        aria-hidden="true"
                      />
                      <div className="min-w-0 flex-1">
                        <div className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1">
                          <span className="truncate text-sm font-semibold text-foreground">
                            {course.name}
                          </span>
                          <span className="inline-flex shrink-0 items-center rounded-md bg-muted/60 px-1.5 py-0.5 text-[10px] font-medium text-muted-foreground tabular-nums">
                            {DAY_LABELS[course.day_of_week - 1]} {course.start_time}–{course.end_time}
                          </span>
                          {course.week_pattern && (
                            <span className="shrink-0 rounded-md border border-border/50 px-1.5 py-0.5 text-[10px] text-muted-foreground/80">
                              {course.week_pattern}
                            </span>
                          )}
                        </div>
                        <div className="mt-1 flex min-w-0 flex-wrap items-center gap-x-3 gap-y-1 text-[11px] text-muted-foreground">
                          {course.teacher && (
                            <span className="truncate">{course.teacher}</span>
                          )}
                          {course.location && (
                            <span className="flex min-w-0 items-center gap-0.5">
                              <MapPin className="h-3 w-3 shrink-0 opacity-70" aria-hidden="true" />
                              <span className="truncate">{course.location}</span>
                            </span>
                          )}
                          {course.semester && (
                            <span className="shrink-0 text-muted-foreground/60">
                              {formatSemester(course.semester)}
                            </span>
                          )}
                        </div>
                      </div>
                      <div className="flex shrink-0 items-center gap-0.5">
                        <Button
                          variant="ghost"
                          size="icon"
                          className="h-11 w-11 text-muted-foreground hover:text-foreground sm:h-8 sm:w-8"
                          aria-label={`编辑课程 ${course.name}`}
                          onClick={() => openEditForm(course)}
                        >
                          <Pencil className="h-4 w-4 sm:h-3.5 sm:w-3.5" />
                        </Button>
                        <Button
                          variant="ghost"
                          size="icon"
                          className="h-11 w-11 text-muted-foreground hover:text-destructive sm:h-8 sm:w-8"
                          aria-label={`删除课程 ${course.name}`}
                          onClick={() => void handleDelete(course.id)}
                        >
                          <Trash2 className="h-4 w-4 sm:h-3.5 sm:w-3.5" />
                        </Button>
                      </div>
                    </AcrylicPanel>
                  ))}
                </div>
              </>
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
      <CourseDetailModal
        open={showDetail}
        onOpenChange={setShowDetail}
        course={detailCourse}
        onEdit={(course) => {
          setShowDetail(false)
          openEditForm(course)
        }}
        onDelete={(courseId) => {
          setShowDetail(false)
          handleDelete(courseId)
        }}
      />

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

      {ENABLE_COURSE_IMPORT && (
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
      )}

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
