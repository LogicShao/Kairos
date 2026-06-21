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
  Save,
  ClipboardPaste,
  Upload,
  ChevronLeft,
  ChevronRight,
  GraduationCap,
  EllipsisVertical,
} from "lucide-react"

const DAY_LABELS = ["周一", "周二", "周三", "周四", "周五", "周六", "周日"]
const DAY_SHORT = ["一", "二", "三", "四", "五", "六", "日"]
const HOUR_HEIGHT = 44
const START_HOUR = 7
const END_HOUR = 22
const TOTAL_HOURS = END_HOUR - START_HOUR

const FIELD_CLASS =
  "w-full h-9 rounded-md border border-input bg-background px-3 py-1 text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring focus:border-ring"

const COLOR_OPTIONS = [
  { value: "#B8C9E8", label: "雾蓝" },
  { value: "#C5E0D8", label: "薄荷" },
  { value: "#F0D5D8", label: "豆沙粉" },
  { value: "#E8D5F0", label: "香芋紫" },
  { value: "#F5E6C8", label: "鹅黄" },
  { value: "#D5E8F0", label: "浅海蓝" },
  { value: "#F0E0D0", label: "暖杏" },
  { value: "#D8E8D0", label: "嫩绿" },
]

const SEMESTER_OPTIONS = ["2024S1", "2024S2", "2025S1", "2026S1"]

function timeToMinutes(time: string): number {
  const [h, m] = time.split(":").map(Number)
  return h * 60 + m
}

function minutesToTop(minutes: number): number {
  return ((minutes - START_HOUR * 60) / 60) * HOUR_HEIGHT
}

function hexToRgb(hex: string): { r: number; g: number; b: number } | null {
  const match = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex)
  if (!match) return null
  return {
    r: parseInt(match[1], 16),
    g: parseInt(match[2], 16),
    b: parseInt(match[3], 16),
  }
}

function isDarkColor(hex: string): boolean {
  const rgb = hexToRgb(hex)
  if (!rgb) return false
  const luminance = (0.299 * rgb.r + 0.587 * rgb.g + 0.114 * rgb.b) / 255
  return luminance <= 0.55
}

function hexToRgba(hex: string, alpha: number): string {
  const rgb = hexToRgb(hex)
  if (!rgb) return hex
  return `rgba(${rgb.r}, ${rgb.g}, ${rgb.b}, ${alpha})`
}

function todayKey(): string {
  const d = new Date()
  const year = d.getFullYear()
  const month = String(d.getMonth() + 1).padStart(2, "0")
  const day = String(d.getDate()).padStart(2, "0")
  return `${year}-${month}-${day}`
}

function computeCurrentWeek(startDate: string): number {
  const start = new Date(`${startDate}T00:00:00`)
  if (Number.isNaN(start.getTime())) return 1
  const today = new Date()
  today.setHours(0, 0, 0, 0)
  const diffDays = Math.floor((today.getTime() - start.getTime()) / 86_400_000)
  return Math.max(1, Math.floor(diffDays / 7) + 1)
}

function dayCellKey(weekStartDate: string, offset: number): string {
  const start = new Date(`${weekStartDate}T00:00:00`)
  if (Number.isNaN(start.getTime())) return ""
  start.setDate(start.getDate() + offset)
  const month = String(start.getMonth() + 1).padStart(2, "0")
  const day = String(start.getDate()).padStart(2, "0")
  return `${start.getFullYear()}-${month}-${day}`
}

interface CourseFormData {
  name: string
  day_of_week: number
  start_time: string
  end_time: string
  week_pattern: string
  semester_start_date: string
  location: string
  teacher: string
  color: string
  semester: string
}

function emptyForm(): CourseFormData {
  return {
    name: "",
    day_of_week: 1,
    start_time: "08:00",
    end_time: "09:40",
    week_pattern: "",
    semester_start_date: "",
    location: "",
    teacher: "",
    color: "#B8C9E8",
    semester: "",
  }
}

function courseToForm(c: Course): CourseFormData {
  return {
    name: c.name,
    day_of_week: c.day_of_week,
    start_time: c.start_time,
    end_time: c.end_time,
    week_pattern: c.week_pattern,
    semester_start_date: c.semester_start_date,
    location: c.location,
    teacher: c.teacher,
    color: c.color,
    semester: c.semester,
  }
}

export function CourseSchedule() {
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
  const [weekLoading, setWeekLoading] = useState(false)
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
  const [showSemesterSubmenu, setShowSemesterSubmenu] = useState(false)
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
        setShowSemesterSubmenu(false)
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
      const text = await readText()
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
    setShowSemesterSubmenu(false)
  }

  const handleSelectSemester = (semester: string) => {
    setSemesterFilter(semester)
    setShowOverflowMenu(false)
    setShowSemesterSubmenu(false)
  }

  const today = todayKey()
  const currentWeek = weekData?.semester_start_date
    ? computeCurrentWeek(weekData.semester_start_date)
    : null
  const isCurrentWeek = currentWeek !== null && currentWeek === weekIndex

  return (
    <div className="flex flex-col h-full">
      {/* ─── Header: week nav + overflow ─── */}
      <div className="flex items-center gap-2 px-3 py-2 shrink-0">
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
            <span className={cn("text-[10px] text-muted-foreground transition-transform", showWeekPicker && "rotate-180")}>▼</span>
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

              {/* 切换学期 submenu */}
              <div>
                <button
                  type="button"
                  onClick={() => setShowSemesterSubmenu(!showSemesterSubmenu)}
                  className={cn(
                    "w-full text-left rounded-md px-3 py-2 text-sm text-foreground hover:bg-muted/60 transition-colors flex items-center justify-between",
                    showSemesterSubmenu && "bg-muted/60",
                  )}
                >
                  切换学期
                  <span className={cn("text-[10px] text-muted-foreground transition-transform", showSemesterSubmenu && "rotate-180")}>▼</span>
                </button>
                {showSemesterSubmenu && (
                  <div className="ml-2 mt-0.5 space-y-0.5 border-l border-border/40 pl-2">
                    <button
                      type="button"
                      onClick={() => handleSelectSemester("")}
                      className={cn(
                        "w-full text-left rounded-md px-3 py-1.5 text-xs transition-colors",
                        !semesterFilter
                          ? "bg-primary text-primary-foreground"
                          : "text-foreground hover:bg-muted/60",
                      )}
                    >
                      全部学期
                    </button>
                    {SEMESTER_OPTIONS.map((sem) => (
                      <button
                        key={sem}
                        type="button"
                        onClick={() => handleSelectSemester(sem)}
                        className={cn(
                          "w-full text-left rounded-md px-3 py-1.5 text-xs transition-colors",
                          semesterFilter === sem
                            ? "bg-primary text-primary-foreground"
                            : "text-foreground hover:bg-muted/60",
                        )}
                      >
                        {sem}
                      </button>
                    ))}
                  </div>
                )}
              </div>

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
                  isToday ? "bg-primary text-primary-foreground shadow-sm" : "text-foreground hover:bg-muted/40",
                )}
              >
                <span className="text-[10px] leading-tight font-medium">{day}</span>
                <span className="text-xs font-semibold leading-tight tabular-nums mt-0.5">{dateNum || "—"}</span>
              </button>
            )
          })}
        </div>
      </div>

      {/* ─── Date strip (mobile: aligned with 32px + 7×44px grid) ─── */}
      <div className="md:hidden shrink-0 pb-1 overflow-x-auto scrollbar-none">
        <div
          className="grid select-none"
          style={{ gridTemplateColumns: "32px repeat(7, minmax(44px, 1fr))", minWidth: "348px" }}
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
                  isToday ? "bg-primary text-primary-foreground shadow-sm" : "text-foreground hover:bg-muted/40",
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
      <div className="flex-1 overflow-y-auto min-h-0 pb-16 md:pb-4">
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
                            className="absolute right-1.5 text-[10px] text-muted-foreground/70 tabular-nums"
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
                                className={cn(
                                  "absolute left-0.5 right-0.5 overflow-hidden rounded-lg px-2 py-1.5 text-left transition-all",
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
                                <div className="flex items-center gap-1">
                                  {isExam && (
                                    <GraduationCap className="h-3 w-3 shrink-0" />
                                  )}
                                  <span className="line-clamp-2 text-[11px] font-semibold leading-tight">
                                    {item.title}
                                  </span>
                                </div>
                                <div className="text-[9px] leading-tight opacity-70">
                                  {item.start_time}
                                </div>
                                {item.location && (
                                  <div className="truncate text-[10px] leading-tight opacity-70">
                                    {item.location}
                                  </div>
                                )}
                              </button>
                            )
                          })}
                        </div>
                      )
                    })}
                  </div>
                </div>

                {/* Mobile: scrollable 7-column week grid */}
                <div className="md:hidden overflow-x-auto">
                  <div
                    className="grid select-none"
                    style={{
                      gridTemplateColumns: "32px repeat(7, minmax(44px, 1fr))",
                      minWidth: "348px",
                    }}
                  >
                    {/* Time axis column (sticky left) */}
                    <div className="relative bg-card" style={{ height: `${TOTAL_HOURS * HOUR_HEIGHT}px` }}>
                      {Array.from({ length: TOTAL_HOURS }, (_, i) => {
                        if (i === 0) return <div key={i} style={{ height: `${HOUR_HEIGHT}px` }} />
                        return (
                          <div
                            key={i}
                            className="absolute right-1 text-[9px] text-muted-foreground/70 tabular-nums"
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
                                className={cn(
                                  "absolute left-px right-px overflow-hidden rounded-lg px-2 py-1.5 text-left transition-all active:brightness-90",
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
                                <span className="block line-clamp-2 text-[10px] font-semibold leading-tight">
                                  {item.title}
                                </span>
                                <span className="block text-[9px] leading-tight opacity-65">
                                  {item.start_time}
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
      <Modal
        open={showForm}
        onOpenChange={(open) => {
          setShowForm(open)
          if (!open) setEditingCourse(null)
        }}
        title={editingCourse ? "编辑课程" : "新建课程"}
        description="填写课程的时间、周次与地点等信息"
        className="max-w-2xl"
      >
        <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
          <div className="col-span-2">
            <label className="mb-1 block text-xs font-medium text-muted-foreground">
              课程名称 <span className="text-destructive">*</span>
            </label>
            <input
              type="text"
              value={form.name}
              onChange={(e) => setForm({ ...form, name: e.target.value })}
              required
              placeholder="课程名称"
              className={FIELD_CLASS}
            />
          </div>
          <div>
            <label className="mb-1 block text-xs font-medium text-muted-foreground">星期</label>
            <select
              value={form.day_of_week}
              onChange={(e) => setForm({ ...form, day_of_week: Number(e.target.value) })}
              className={FIELD_CLASS}
            >
              {DAY_LABELS.map((label, i) => (
                <option key={i + 1} value={i + 1}>
                  {label}
                </option>
              ))}
            </select>
          </div>
          <div>
            <label className="mb-1 block text-xs font-medium text-muted-foreground">颜色</label>
            <select
              value={form.color}
              onChange={(e) => setForm({ ...form, color: e.target.value })}
              className={FIELD_CLASS}
            >
              {COLOR_OPTIONS.map((opt) => (
                <option key={opt.value} value={opt.value}>
                  {opt.label}
                </option>
              ))}
            </select>
          </div>
          <div>
            <label className="mb-1 block text-xs font-medium text-muted-foreground">开始时间</label>
            <input
              type="time"
              value={form.start_time}
              onChange={(e) => setForm({ ...form, start_time: e.target.value })}
              className={FIELD_CLASS}
            />
          </div>
          <div>
            <label className="mb-1 block text-xs font-medium text-muted-foreground">结束时间</label>
            <input
              type="time"
              value={form.end_time}
              onChange={(e) => setForm({ ...form, end_time: e.target.value })}
              className={FIELD_CLASS}
            />
          </div>
          <div>
            <label className="mb-1 block text-xs font-medium text-muted-foreground">周次</label>
            <input
              type="text"
              value={form.week_pattern}
              onChange={(e) => setForm({ ...form, week_pattern: e.target.value })}
              placeholder="e.g. 1-17周全周"
              className={FIELD_CLASS}
            />
          </div>
          <div>
            <label className="mb-1 block text-xs font-medium text-muted-foreground">
              学期开始日期
            </label>
            <input
              type="date"
              value={form.semester_start_date}
              onChange={(e) => setForm({ ...form, semester_start_date: e.target.value })}
              className={FIELD_CLASS}
            />
          </div>
          <div>
            <label className="mb-1 block text-xs font-medium text-muted-foreground">地点</label>
            <input
              type="text"
              value={form.location}
              onChange={(e) => setForm({ ...form, location: e.target.value })}
              placeholder="教室/地点"
              className={FIELD_CLASS}
            />
          </div>
          <div>
            <label className="mb-1 block text-xs font-medium text-muted-foreground">教师</label>
            <input
              type="text"
              value={form.teacher}
              onChange={(e) => setForm({ ...form, teacher: e.target.value })}
              placeholder="授课教师"
              className={FIELD_CLASS}
            />
          </div>
          <div>
            <label className="mb-1 block text-xs font-medium text-muted-foreground">学期</label>
            <input
              type="text"
              value={form.semester}
              onChange={(e) => setForm({ ...form, semester: e.target.value })}
              placeholder="e.g. 2026S1"
              className={FIELD_CLASS}
            />
          </div>
        </div>

        <div className="mt-5 flex items-center justify-end gap-2">
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={() => {
              setShowForm(false)
              setEditingCourse(null)
            }}
          >
            取消
          </Button>
          <Button
            type="button"
            size="sm"
            disabled={saving || !form.name.trim()}
            onClick={handleSave}
          >
            <Save className="mr-1 h-3.5 w-3.5" />
            {saving ? "保存中..." : "保存"}
          </Button>
        </div>
      </Modal>

      {/* ─── Import Modal ─── */}
      <Modal
        open={showImport}
        onOpenChange={(open) => {
          setShowImport(open)
          if (!open) {
            setImportError(null)
            setImportFeedback(null)
          }
        }}
        title="从剪贴板导入课表"
        description="从教务系统网页复制课表表格后粘贴或读取剪贴板"
        className="max-w-2xl"
      >
        <div className="grid grid-cols-1 gap-3 sm:grid-cols-[minmax(0,1fr)_200px]">
          <div>
            <label className="mb-1 block text-xs font-medium text-muted-foreground">
              课表文本 <span className="text-destructive">*</span>
            </label>
            <textarea
              value={importText}
              onChange={(e) => setImportText(e.target.value)}
              placeholder="先从教务系统网页复制课表表格，再点击「读取剪贴板」或直接粘贴到这里。"
              rows={8}
              className="w-full resize-y rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring focus:border-ring"
            />
          </div>
          <div className="space-y-3">
            <div>
              <label className="mb-1 block text-xs font-medium text-muted-foreground">学期</label>
              <input
                type="text"
                value={importSemester}
                onChange={(e) => setImportSemester(e.target.value)}
                placeholder="e.g. 2026S1"
                className={FIELD_CLASS}
              />
            </div>
            <div>
              <label className="mb-1 block text-xs font-medium text-muted-foreground">
                学期开始日期
              </label>
              <input
                type="date"
                value={importStartDate}
                onChange={(e) => setImportStartDate(e.target.value)}
                className={FIELD_CLASS}
              />
            </div>
            <div className="rounded-md border border-border/60 bg-background/60 p-3 text-xs text-muted-foreground space-y-1">
              <p>解析与去重在后端完成。</p>
              <p>填写开始日期可启用周视图。</p>
              <p>重复课程会被自动跳过。</p>
            </div>
          </div>
        </div>

        {importError && (
          <p className="mt-3 text-sm text-destructive">{importError}</p>
        )}
        {importFeedback && (
          <p className="mt-3 text-sm text-muted-foreground">{importFeedback.message}</p>
        )}

        <div className="mt-4 flex items-center justify-end gap-2">
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={() => void handleReadClipboard()}
          >
            <ClipboardPaste className="mr-1 h-3.5 w-3.5" />
            读取剪贴板
          </Button>
          <Button
            type="button"
            size="sm"
            disabled={importing || !importText.trim()}
            onClick={() => void handleImport()}
          >
            <Upload className="mr-1 h-3.5 w-3.5" />
            {importing ? "导入中..." : "开始导入"}
          </Button>
        </div>
      </Modal>
    </div>
  )
}
