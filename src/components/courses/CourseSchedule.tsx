import { useEffect, useRef, useState, type PointerEvent as ReactPointerEvent } from "react"
import { invoke } from "@tauri-apps/api/core"
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
  CalendarDays,
  List,
} from "lucide-react"

const DAY_LABELS = ["周一", "周二", "周三", "周四", "周五", "周六", "周日"]
const HOUR_HEIGHT = 56
const START_HOUR = 7
const END_HOUR = 22
const TOTAL_HOURS = END_HOUR - START_HOUR
const DRAG_THRESHOLD = 56

const FIELD_CLASS =
  "w-full h-9 rounded-md border border-input bg-background px-3 py-1 text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring focus:border-ring"

const COLOR_OPTIONS = [
  { value: "#7C8CC0", label: "Periwinkle" },
  { value: "#3B82F6", label: "蓝色" },
  { value: "#EF4444", label: "红色" },
  { value: "#10B981", label: "绿色" },
  { value: "#F59E0B", label: "琥珀" },
  { value: "#8B5CF6", label: "紫色" },
  { value: "#EC4899", label: "粉色" },
  { value: "#06B6D4", label: "青色" },
]

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

function contrastingTextColor(hex: string): string {
  const rgb = hexToRgb(hex)
  if (!rgb) return "#1e1e2e"
  const luminance = (0.299 * rgb.r + 0.587 * rgb.g + 0.114 * rgb.b) / 255
  return luminance > 0.55 ? "#1e1e2e" : "#ffffff"
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
    color: "#7C8CC0",
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
  const [semesterFilter, setSemesterFilter] = useState("")

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

  // 拖拽切周状态
  const dragStartRef = useRef<number | null>(null)
  const dragDeltaRef = useRef(0)

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

  function goPrevWeek() {
    setWeekIndex((w) => Math.max(1, w - 1))
  }

  function goNextWeek() {
    setWeekIndex((w) => w + 1)
  }

  function goCurrentWeek() {
    if (weekData?.semester_start_date) {
      setWeekIndex(computeCurrentWeek(weekData.semester_start_date))
    }
  }

  function goPrevDay() {
    setMobileDayIndex((d) => (d === 0 ? 6 : d - 1))
  }

  function goNextDay() {
    setMobileDayIndex((d) => (d === 6 ? 0 : d + 1))
  }

  function goToday() {
    const today = new Date().getDay()
    setMobileDayIndex(today === 0 ? 6 : today - 1)
    goCurrentWeek()
  }

  function handlePointerDown(e: ReactPointerEvent) {
    dragStartRef.current = e.clientX
    dragDeltaRef.current = 0
  }

  function handlePointerMove(e: ReactPointerEvent) {
    if (dragStartRef.current === null) return
    dragDeltaRef.current = e.clientX - dragStartRef.current
  }

  function handlePointerUp() {
    if (dragStartRef.current === null) return
    const delta = dragDeltaRef.current
    dragStartRef.current = null
    if (delta > DRAG_THRESHOLD) goPrevWeek()
    else if (delta < -DRAG_THRESHOLD) goNextWeek()
  }

  function handleBlockClick(item: WeekScheduleItem) {
    if (Math.abs(dragDeltaRef.current) > 8) return // 拖拽中忽略点击
    if (item.kind !== "course") return
    const course = courses.find((c) => c.id === item.id)
    if (course) openEditForm(course)
  }

  const today = todayKey()

  return (
    <div className="w-full max-w-6xl mx-auto space-y-4 animate-in fade-in-0 slide-in-from-bottom-2 duration-300">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h2 className="text-lg font-heading font-medium text-foreground">课程表</h2>
        <div className="flex flex-wrap items-center gap-2">
          {/* 视图切换 */}
          <div className="inline-flex rounded-lg border border-border/60 bg-card/60 p-0.5">
            <button
              type="button"
              onClick={() => setViewMode("week")}
              className={cn(
                "inline-flex items-center gap-1 rounded-md px-2.5 py-1 text-xs font-medium transition-colors",
                viewMode === "week"
                  ? "bg-primary text-primary-foreground shadow-sm"
                  : "text-muted-foreground hover:text-foreground",
              )}
            >
              <CalendarDays className="h-3.5 w-3.5" />
              周视图
            </button>
            <button
              type="button"
              onClick={() => setViewMode("list")}
              className={cn(
                "inline-flex items-center gap-1 rounded-md px-2.5 py-1 text-xs font-medium transition-colors",
                viewMode === "list"
                  ? "bg-primary text-primary-foreground shadow-sm"
                  : "text-muted-foreground hover:text-foreground",
              )}
            >
              <List className="h-3.5 w-3.5" />
              全部课程
            </button>
          </div>

          <div className="relative">
            <select
              value={semesterFilter}
              onChange={(e) => setSemesterFilter(e.target.value)}
              className="h-8 appearance-none rounded-md border border-input bg-background pl-2.5 pr-6 text-xs text-foreground focus:outline-none focus:ring-2 focus:ring-ring focus:border-ring"
            >
              <option value="">全部学期</option>
              <option value="2024S1">2024S1</option>
              <option value="2024S2">2024S2</option>
              <option value="2025S1">2025S1</option>
              <option value="2026S1">2026S1</option>
            </select>
            <ChevronRight className="pointer-events-none absolute right-1.5 top-1/2 h-3 w-3 -translate-y-1/2 rotate-90 text-muted-foreground" />
          </div>

          <Button size="sm" variant="outline" onClick={openImportPanel} disabled={importing} className="px-2.5 sm:px-3">
            <ClipboardPaste className="h-4 w-4 sm:mr-1" />
            <span className="hidden sm:inline">从剪贴板导入</span>
          </Button>
          <Button size="sm" onClick={openCreateForm} className="px-2.5 sm:px-3">
            <Plus className="h-4 w-4 sm:mr-1" />
            <span className="hidden sm:inline">添加课程</span>
          </Button>
        </div>
      </div>

      {/* 周视图 */}
      {viewMode === "week" && (
        <>
          <div className="flex items-center justify-between gap-3">
            <div className="flex items-center gap-2">
              <Button size="icon-sm" variant="outline" onClick={goPrevWeek} aria-label="上一周">
                <ChevronLeft className="h-4 w-4" />
              </Button>
              <Button size="icon-sm" variant="outline" onClick={goNextWeek} aria-label="下一周">
                <ChevronRight className="h-4 w-4" />
              </Button>
              <Button size="sm" variant="ghost" onClick={goCurrentWeek} disabled={!weekData?.semester_start_date}>
                本周
              </Button>
            </div>
            <div className="text-xs text-muted-foreground tabular-nums">
              第 {weekIndex} 周
              {weekData?.week_start_date && (
                <span className="ml-2 text-muted-foreground/70">
                  {weekData.week_start_date} ~ {weekData.week_end_date}
                </span>
              )}
            </div>
          </div>

          {/* 移动端单日导航 */}
          <div className="flex md:hidden items-center justify-between gap-2">
            <Button size="icon-sm" variant="outline" onClick={goPrevDay} aria-label="前一天">
              <ChevronLeft className="h-4 w-4" />
            </Button>
            <div className="text-center min-w-0">
              <span className="text-sm font-medium text-foreground">
                {DAY_LABELS[mobileDayIndex]}
              </span>
              {weekData && (
                <span className="ml-2 text-xs text-muted-foreground tabular-nums">
                  {dayCellKey(weekData.week_start_date, mobileDayIndex).slice(5).replace("-", "/")}
                </span>
              )}
            </div>
            <Button size="icon-sm" variant="outline" onClick={goNextDay} aria-label="后一天">
              <ChevronRight className="h-4 w-4" />
            </Button>
            <Button
              size="sm"
              variant="ghost"
              onClick={goToday}
              disabled={!weekData?.semester_start_date}
            >
              今天
            </Button>
          </div>

          {weekError ? (
            <AcrylicPanel className="p-8 text-center bg-card">
              <p className="text-sm text-muted-foreground">{weekError}</p>
              <p className="mt-1 text-xs text-muted-foreground/60">
                请先导入课表（含学期开始日期），或在课程中填写"学期开始日期"。
              </p>
            </AcrylicPanel>
          ) : (
            <>
              {/* 桌面端 7 列周视图 */}
              <AcrylicPanel className="hidden md:block bg-card overflow-x-auto">
                {weekLoading && (
                  <div className="flex items-center justify-center py-12">
                    <div className="h-6 w-6 animate-spin rounded-full border-2 border-primary border-t-transparent" />
                  </div>
                )}
                {!weekLoading && (
                  <div
                    className="grid select-none"
                    style={{ gridTemplateColumns: "48px repeat(7, minmax(0, 1fr))", minWidth: "680px" }}
                    onPointerDown={handlePointerDown}
                    onPointerMove={handlePointerMove}
                    onPointerUp={handlePointerUp}
                    onPointerLeave={handlePointerUp}
                  >
                    {/* 表头：星期 + 日期 */}
                    <div className="h-12 border-b border-border/40" />
                    {DAY_LABELS.map((label, i) => {
                      const dateKey = weekData ? dayCellKey(weekData.week_start_date, i) : ""
                      const isToday = dateKey === today
                      return (
                        <div
                          key={label}
                          className={cn(
                            "flex h-12 flex-col items-center justify-center border-b border-border/40 text-xs",
                            isToday ? "text-primary" : "text-foreground",
                          )}
                        >
                          <span className="font-medium">{label}</span>
                          {dateKey && (
                            <span
                              className={cn(
                                "mt-0.5 text-[10px] tabular-nums",
                                isToday
                                  ? "rounded-full bg-primary px-1.5 text-primary-foreground"
                                  : "text-muted-foreground/60",
                              )}
                            >
                              {dateKey.slice(5).replace("-", "/")}
                            </span>
                          )}
                        </div>
                      )
                    })}

                    {/* 时间轴列 */}
                    <div className="relative" style={{ height: `${TOTAL_HOURS * HOUR_HEIGHT}px` }}>
                      {Array.from({ length: TOTAL_HOURS }, (_, i) => (
                        <div
                          key={i}
                          className="absolute right-1.5 text-[10px] text-muted-foreground/60 tabular-nums"
                          style={{ top: `${i * HOUR_HEIGHT - 6}px` }}
                        >
                          {i > 0 ? `${String(START_HOUR + i).padStart(2, "0")}:00` : ""}
                        </div>
                      ))}
                    </div>

                    {/* 每日列 */}
                    {DAY_LABELS.map((_, dayIdx) => {
                      const dateKey = weekData ? dayCellKey(weekData.week_start_date, dayIdx) : ""
                      const isToday = dateKey === today
                      const dayItems = (weekData?.items ?? []).filter((it) => it.day_of_week === dayIdx + 1)
                      return (
                        <div
                          key={dayIdx}
                          className={cn(
                            "relative border-l border-border/20",
                            isToday && "bg-primary/[0.04]",
                          )}
                          style={{ height: `${TOTAL_HOURS * HOUR_HEIGHT}px` }}
                        >
                          {/* 小时网格线 */}
                          {Array.from({ length: TOTAL_HOURS }, (_, i) => (
                            <div
                              key={i}
                              className="absolute inset-x-0 border-t border-border/15"
                              style={{ top: `${i * HOUR_HEIGHT}px` }}
                            />
                          ))}

                          {/* 课程/考试块 */}
                          {dayItems.map((item) => {
                            const startMin = timeToMinutes(item.start_time)
                            const endMin = timeToMinutes(item.end_time)
                            const top = minutesToTop(startMin)
                            const height = Math.max(22, ((endMin - startMin) / 60) * HOUR_HEIGHT - 2)
                            const isExam = item.kind === "exam"
                            return (
                              <button
                                key={`${item.kind}-${item.id}`}
                                type="button"
                                onClick={() => handleBlockClick(item)}
                                className={cn(
                                  "absolute left-0.5 right-0.5 overflow-hidden rounded-md px-1.5 py-1 text-left transition-all",
                                  "hover:z-20 hover:shadow-lg",
                                  isExam ? "border-2 border-dashed cursor-default" : "cursor-pointer hover:brightness-105",
                                )}
                                style={{
                                  top: `${top}px`,
                                  height: `${height}px`,
                                  backgroundColor: isExam ? hexToRgba(item.color, 0.16) : item.color,
                                  borderColor: isExam ? item.color : undefined,
                                  color: isExam ? item.color : contrastingTextColor(item.color),
                                }}
                              >
                                <div className="flex items-center gap-1">
                                  {isExam && <GraduationCap className="h-3 w-3 shrink-0" />}
                                  <span className="truncate text-[11px] font-medium leading-tight">
                                    {item.title}
                                  </span>
                                </div>
                                <div className="truncate text-[9px] leading-tight opacity-80">
                                  {item.start_time}-{item.end_time}
                                </div>
                                {item.location && (
                                  <div className="truncate text-[9px] leading-tight opacity-70">
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
                )}
              </AcrylicPanel>

              {/* 移动端单日列视图 */}
              {!weekLoading && weekData && (
                <AcrylicPanel className="md:hidden bg-card">
                  {(() => {
                    const dayItems = (weekData.items ?? []).filter((it) => it.day_of_week === mobileDayIndex + 1)
                    return (
                      <div className="select-none" style={{ minHeight: `${TOTAL_HOURS * HOUR_HEIGHT + 20}px` }}>
                        {/* 时间轴列 + 当日列 */}
                        <div className="relative" style={{ height: `${TOTAL_HOURS * HOUR_HEIGHT}px` }}>
                          {/* 时间轴 */}
                          {Array.from({ length: TOTAL_HOURS }, (_, i) => (
                            <div
                              key={i}
                              className="absolute left-0 text-[10px] text-muted-foreground/60 tabular-nums w-10 text-right pr-2"
                              style={{ top: `${i * HOUR_HEIGHT - 6}px` }}
                            >
                              {i > 0 ? `${String(START_HOUR + i).padStart(2, "0")}:00` : ""}
                            </div>
                          ))}

                          {/* 当日内容区域 */}
                          <div className="ml-12 relative" style={{ height: `${TOTAL_HOURS * HOUR_HEIGHT}px` }}>
                            {/* 小时网格线 */}
                            {Array.from({ length: TOTAL_HOURS }, (_, i) => (
                              <div
                                key={i}
                                className="absolute inset-x-0 border-t border-border/15"
                                style={{ top: `${i * HOUR_HEIGHT}px` }}
                              />
                            ))}

                            {/* 课程/考试块 */}
                            {dayItems.map((item) => {
                              const startMin = timeToMinutes(item.start_time)
                              const endMin = timeToMinutes(item.end_time)
                              const top = minutesToTop(startMin)
                              const height = Math.max(28, ((endMin - startMin) / 60) * HOUR_HEIGHT - 2)
                              const isExam = item.kind === "exam"
                              return (
                                <button
                                  key={`m-${item.kind}-${item.id}`}
                                  type="button"
                                  onClick={() => handleBlockClick(item)}
                                  className={cn(
                                    "absolute left-1 right-1 overflow-hidden rounded-md px-2 py-1.5 text-left transition-all active:brightness-90",
                                    isExam ? "border-2 border-dashed" : "",
                                  )}
                                  style={{
                                    top: `${top}px`,
                                    height: `${height}px`,
                                    backgroundColor: isExam ? hexToRgba(item.color, 0.16) : item.color,
                                    borderColor: isExam ? item.color : undefined,
                                    color: isExam ? item.color : contrastingTextColor(item.color),
                                  }}
                                >
                                  <div className="flex items-center gap-1">
                                    {isExam && <GraduationCap className="h-3 w-3 shrink-0" />}
                                    <span className="truncate text-sm font-medium leading-tight">
                                      {item.title}
                                    </span>
                                  </div>
                                  <div className="truncate text-xs leading-tight opacity-80">
                                    {item.start_time}-{item.end_time}
                                  </div>
                                  {item.location && (
                                    <div className="truncate text-xs leading-tight opacity-70">
                                      {item.location}
                                    </div>
                                  )}
                                </button>
                              )
                            })}
                          </div>
                        </div>
                      </div>
                    )
                  })()}
                </AcrylicPanel>
              )}

              {/* 移动端空状态 */}
              {!weekLoading && !weekData && (
                <p className="md:hidden py-8 text-center text-sm text-muted-foreground">
                  {weekError ?? "暂无课表数据"}
                </p>
              )}
            </>
          )}
        </>
      )}

      {/* 全部课程列表 */}
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
            <AcrylicPanel className="p-8 text-center">
              <p className="text-sm text-muted-foreground">暂无课程</p>
              <p className="mt-1 text-xs text-muted-foreground/60">点击"添加课程"按钮创建第一个课程</p>
            </AcrylicPanel>
          )}

          {!loading && courses.length > 0 && (
            <div className="space-y-2">
              {courses.map((course) => (
                <AcrylicPanel
                  key={course.id}
                  className="flex items-center gap-3 bg-card p-3 transition-all hover:-translate-y-0.5 hover:bg-card/95 hover:shadow-md"
                >
                  <div className="h-3 w-3 shrink-0 rounded-full" style={{ backgroundColor: course.color }} />
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2">
                      <span className="truncate text-sm font-medium text-foreground">{course.name}</span>
                      <span className="shrink-0 text-[10px] text-muted-foreground">
                        {DAY_LABELS[course.day_of_week - 1]} {course.start_time}-{course.end_time}
                      </span>
                      {course.week_pattern && (
                        <span className="shrink-0 text-[10px] text-muted-foreground/60">{course.week_pattern}</span>
                      )}
                    </div>
                    <div className="mt-0.5 flex items-center gap-2">
                      {course.teacher && <span className="text-xs text-muted-foreground">{course.teacher}</span>}
                      {course.location && <span className="text-xs text-muted-foreground/60">{course.location}</span>}
                      {course.semester && <span className="text-[10px] text-muted-foreground/50">{course.semester}</span>}
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

      {/* 课程编辑/新建模态 */}
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
            <label className="mb-1 block text-xs font-medium text-muted-foreground">学期开始日期</label>
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
          <Button type="button" size="sm" disabled={saving || !form.name.trim()} onClick={handleSave}>
            <Save className="mr-1 h-3.5 w-3.5" />
            {saving ? "保存中..." : "保存"}
          </Button>
        </div>
      </Modal>

      {/* 导入模态 */}
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
              <label className="mb-1 block text-xs font-medium text-muted-foreground">学期开始日期</label>
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

        {importError && <p className="mt-3 text-sm text-destructive">{importError}</p>}
        {importFeedback && <p className="mt-3 text-sm text-muted-foreground">{importFeedback.message}</p>}

        <div className="mt-4 flex items-center justify-end gap-2">
          <Button type="button" variant="ghost" size="sm" onClick={() => void handleReadClipboard()}>
            <ClipboardPaste className="mr-1 h-3.5 w-3.5" />
            读取剪贴板
          </Button>
          <Button type="button" size="sm" disabled={importing || !importText.trim()} onClick={() => void handleImport()}>
            <Upload className="mr-1 h-3.5 w-3.5" />
            {importing ? "导入中..." : "开始导入"}
          </Button>
        </div>
      </Modal>
    </div>
  )
}
