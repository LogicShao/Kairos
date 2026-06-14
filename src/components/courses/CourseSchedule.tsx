import { useState, useEffect } from "react"
import { invoke } from "@tauri-apps/api/core"
import type { Course, CreateCourseRequest, UpdateCourseRequest, CourseFilterParams } from "@/types/course"
import { Button } from "@/components/ui/button"
import { AcrylicPanel } from "@/components/shared/acrylic-panel"
import { cn } from "@/lib/utils"
import { Plus, Pencil, Trash2, X, Save } from "lucide-react"

const DAY_LABELS = ["周一", "周二", "周三", "周四", "周五", "周六", "周日"]
const HOUR_HEIGHT = 64
const START_HOUR = 7
const END_HOUR = 22
const TOTAL_HOURS = END_HOUR - START_HOUR

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

interface CourseFormData {
  name: string
  day_of_week: number
  start_time: string
  end_time: string
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
    end_time: "09:30",
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
  const [showForm, setShowForm] = useState(false)
  const [editingCourse, setEditingCourse] = useState<Course | null>(null)
  const [form, setForm] = useState<CourseFormData>(emptyForm())
  const [semesterFilter, setSemesterFilter] = useState("")
  const [saving, setSaving] = useState(false)

  async function fetchCourses() {
    const filters: CourseFilterParams = {}
    if (semesterFilter) filters.semester = semesterFilter
    const result = await invoke<Course[]>("get_all_courses", { filters })
    setCourses(result)
    setError(null)
  }

  useEffect(() => {
    let cancelled = false
    async function load() {
      try {
        const filters: CourseFilterParams = {}
        if (semesterFilter) filters.semester = semesterFilter
        const result = await invoke<Course[]>("get_all_courses", { filters })
        if (!cancelled) {
          setCourses(result)
          setError(null)
        }
      } catch {
        if (!cancelled) {
          setError("Tauri 不可用 — 展示离线 UI")
        }
      } finally {
        if (!cancelled) {
          setLoading(false)
        }
      }
    }
    void load()
    return () => { cancelled = true }
  }, [semesterFilter])

  function openCreateForm() {
    setForm(emptyForm())
    setEditingCourse(null)
    setShowForm(true)
  }

  function openEditForm(course: Course) {
    setForm(courseToForm(course))
    setEditingCourse(course)
    setShowForm(true)
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
          location: form.location || undefined,
          teacher: form.teacher || undefined,
          color: form.color || undefined,
          semester: form.semester || undefined,
        }
        await invoke("create_course", { cmd: payload })
      }
      setShowForm(false)
      setEditingCourse(null)
      await fetchCourses()
    } finally {
      setSaving(false)
    }
  }

  async function handleDelete(id: number) {
    await invoke("delete_course", { id })
    await fetchCourses()
  }

  return (
    <div className="w-full max-w-6xl mx-auto space-y-4">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-heading font-medium text-foreground">
          课程表
        </h2>
        <div className="flex items-center gap-2">
          <div className="relative">
            <select
              value={semesterFilter}
              onChange={(e) => setSemesterFilter(e.target.value)}
              className="h-8 rounded-md border border-input bg-background pl-2.5 pr-6 text-xs text-foreground focus:outline-none focus:ring-2 focus:ring-ring focus:border-ring appearance-none"
            >
              <option value="">全部学期</option>
              <option value="2024S1">2024S1</option>
              <option value="2024S2">2024S2</option>
              <option value="2025S1">2025S1</option>
            </select>
            <svg className="pointer-events-none absolute right-1.5 top-1/2 -translate-y-1/2 h-3 w-3 text-muted-foreground" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><path d="m6 9 6 6 6-6"/></svg>
          </div>
          <Button size="sm" onClick={openCreateForm} disabled={showForm}>
            <Plus className="h-4 w-4 mr-1" />
            添加课程
          </Button>
        </div>
      </div>

      {showForm && (
        <AcrylicPanel className="p-4 bg-card">
          <div className="flex items-center justify-between mb-4">
            <h3 className="text-sm font-medium text-foreground">
              {editingCourse ? "编辑课程" : "新建课程"}
            </h3>
            <Button
              type="button"
              variant="ghost"
              size="icon"
              className="h-7 w-7"
              onClick={() => { setShowForm(false); setEditingCourse(null) }}
            >
              <X className="h-4 w-4" />
            </Button>
          </div>

          <div className="grid grid-cols-2 sm:grid-cols-4 gap-3 mb-4">
            <div>
              <label className="block text-xs font-medium text-muted-foreground mb-1">
                课程名称 <span className="text-destructive">*</span>
              </label>
              <input
                type="text"
                value={form.name}
                onChange={(e) => setForm({ ...form, name: e.target.value })}
                required
                placeholder="课程名称"
                className="w-full h-9 rounded-md border border-input bg-background px-3 py-1 text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring focus:border-ring"
              />
            </div>
            <div>
              <label className="block text-xs font-medium text-muted-foreground mb-1">星期</label>
              <select
                value={form.day_of_week}
                onChange={(e) => setForm({ ...form, day_of_week: Number(e.target.value) })}
                className="w-full h-9 rounded-md border border-input bg-background px-3 py-1 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring focus:border-ring"
              >
                {DAY_LABELS.map((label, i) => (
                  <option key={i + 1} value={i + 1}>{label}</option>
                ))}
              </select>
            </div>
            <div>
              <label className="block text-xs font-medium text-muted-foreground mb-1">开始时间</label>
              <input
                type="time"
                value={form.start_time}
                onChange={(e) => setForm({ ...form, start_time: e.target.value })}
                className="w-full h-9 rounded-md border border-input bg-background px-3 py-1 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring focus:border-ring"
              />
            </div>
            <div>
              <label className="block text-xs font-medium text-muted-foreground mb-1">结束时间</label>
              <input
                type="time"
                value={form.end_time}
                onChange={(e) => setForm({ ...form, end_time: e.target.value })}
                className="w-full h-9 rounded-md border border-input bg-background px-3 py-1 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring focus:border-ring"
              />
            </div>
          </div>

          <div className="grid grid-cols-2 sm:grid-cols-4 gap-3 mb-4">
            <div>
              <label className="block text-xs font-medium text-muted-foreground mb-1">地点</label>
              <input
                type="text"
                value={form.location}
                onChange={(e) => setForm({ ...form, location: e.target.value })}
                placeholder="教室/地点"
                className="w-full h-9 rounded-md border border-input bg-background px-3 py-1 text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring focus:border-ring"
              />
            </div>
            <div>
              <label className="block text-xs font-medium text-muted-foreground mb-1">教师</label>
              <input
                type="text"
                value={form.teacher}
                onChange={(e) => setForm({ ...form, teacher: e.target.value })}
                placeholder="授课教师"
                className="w-full h-9 rounded-md border border-input bg-background px-3 py-1 text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring focus:border-ring"
              />
            </div>
            <div>
              <label className="block text-xs font-medium text-muted-foreground mb-1">颜色</label>
              <select
                value={form.color}
                onChange={(e) => setForm({ ...form, color: e.target.value })}
                className="w-full h-9 rounded-md border border-input bg-background px-3 py-1 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring focus:border-ring"
              >
                {COLOR_OPTIONS.map((opt) => (
                  <option key={opt.value} value={opt.value}>{opt.label}</option>
                ))}
              </select>
            </div>
            <div>
              <label className="block text-xs font-medium text-muted-foreground mb-1">学期</label>
              <input
                type="text"
                value={form.semester}
                onChange={(e) => setForm({ ...form, semester: e.target.value })}
                placeholder="e.g. 2024S1"
                className="w-full h-9 rounded-md border border-input bg-background px-3 py-1 text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring focus:border-ring"
              />
            </div>
          </div>

          <div className="flex items-center justify-end gap-2 pt-1">
            <Button type="button" variant="ghost" size="sm" onClick={() => { setShowForm(false); setEditingCourse(null) }}>
              取消
            </Button>
            <Button type="button" size="sm" disabled={saving || !form.name.trim()} onClick={handleSave}>
              <Save className="h-3.5 w-3.5 mr-1" />
              {saving ? "保存中..." : "保存"}
            </Button>
          </div>
        </AcrylicPanel>
      )}

      {loading && (
        <div className="flex items-center justify-center py-12">
          <div className="animate-spin h-6 w-6 border-2 border-primary border-t-transparent rounded-full" />
        </div>
      )}

      {!loading && error && courses.length === 0 && (
        <p className="text-center text-sm text-muted-foreground py-8">{error}</p>
      )}

      {!loading && courses.length === 0 && !error && (
        <AcrylicPanel className="p-8 text-center">
          <p className="text-sm text-muted-foreground">暂无课程</p>
          <p className="text-xs text-muted-foreground/60 mt-1">
            点击"添加课程"按钮创建第一个课程
          </p>
        </AcrylicPanel>
      )}

      {!loading && courses.length > 0 && (
        <AcrylicPanel className="overflow-x-auto bg-card">
          <div
            className="grid gap-px"
            style={{
              gridTemplateColumns: `60px repeat(7, 1fr)`,
              minWidth: "768px",
            }}
          >
            <div className="h-10 flex items-center justify-center text-xs text-muted-foreground font-medium border-b border-border/40 bg-card" />
            {DAY_LABELS.map((label) => (
              <div
                key={label}
                className="h-10 flex items-center justify-center text-xs font-medium text-foreground border-b border-border/40 bg-card"
              >
                {label}
              </div>
            ))}

            {Array.from({ length: TOTAL_HOURS }, (_, i) => {
              const hour = START_HOUR + i
              const timeLabel = `${String(hour).padStart(2, "0")}:00`
              return (
                <div
                  key={hour}
                  className="contents"
                >
                  <div className="relative flex items-start justify-end pr-2 pt-0 text-[10px] text-muted-foreground/60 border-b border-border/20 bg-card">
                    <span className="-mt-1.5">{i > 0 ? timeLabel : ""}</span>
                  </div>
                  {DAY_LABELS.map((_, dayIdx) => (
                    <div
                      key={dayIdx}
                      className="relative border-b border-border/20"
                      style={{ height: `${HOUR_HEIGHT}px` }}
                    >
                      {courses
                        .filter((c) => {
                          const startMin = timeToMinutes(c.start_time)
                          const endMin = timeToMinutes(c.end_time)
                          const slotMin = hour * 60
                          const nextSlotMin = slotMin + 60
                          return (
                            c.day_of_week === dayIdx + 1 &&
                            startMin < nextSlotMin &&
                            endMin > slotMin
                          )
                        })
                        .map((course) => {
                          const startMin = timeToMinutes(course.start_time)
                          const endMin = timeToMinutes(course.end_time)
                          const slotMin = hour * 60
                          const top =
                            startMin > slotMin
                              ? minutesToTop(startMin) - i * HOUR_HEIGHT
                              : 0
                          const blockInnerHeight =
                            Math.min(endMin, slotMin + 60) -
                            Math.max(startMin, slotMin)
                          const height = (blockInnerHeight / 60) * HOUR_HEIGHT
                          return (
                            <div
                              key={course.id}
                              className={cn(
                                "absolute left-0.5 right-0.5 rounded px-1.5 py-0.5 overflow-hidden cursor-pointer transition-opacity hover:opacity-90 z-10",
                              )}
                              style={{
                                top: `${top}px`,
                                height: `${height}px`,
                                backgroundColor: course.color,
                                color: contrastingTextColor(course.color),
                              }}
                              onClick={() => openEditForm(course)}
                            >
                              <div className="text-[10px] font-medium leading-tight truncate">
                                {course.name}
                              </div>
                              <div className="text-[9px] leading-tight opacity-80 truncate">
                                {course.start_time}-{course.end_time}
                              </div>
                              {course.location && (
                                <div className="text-[8px] leading-tight opacity-60 truncate">
                                  {course.location}
                                </div>
                              )}
                            </div>
                          )
                        })}
                    </div>
                  ))}
                </div>
              )
            })}
          </div>
        </AcrylicPanel>
      )}

      {!loading && courses.length > 0 && (
        <div className="space-y-2">
          <h3 className="text-sm font-medium text-foreground mt-6">课程列表</h3>
          {courses.map((course) => (
            <AcrylicPanel
              key={course.id}
              className="p-3 bg-card flex items-center gap-3 transition-colors hover:bg-card/95"
            >
              <div
                className="w-3 h-3 rounded-full shrink-0"
                style={{ backgroundColor: course.color }}
              />
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                  <span className="text-sm font-medium text-foreground truncate">
                    {course.name}
                  </span>
                  <span className="text-[10px] text-muted-foreground shrink-0">
                    {DAY_LABELS[course.day_of_week - 1]} {course.start_time}-{course.end_time}
                  </span>
                </div>
                <div className="flex items-center gap-2 mt-0.5">
                  {course.teacher && (
                    <span className="text-xs text-muted-foreground">{course.teacher}</span>
                  )}
                  {course.location && (
                    <span className="text-xs text-muted-foreground/60">{course.location}</span>
                  )}
                  {course.semester && (
                    <span className="text-[10px] text-muted-foreground/50">{course.semester}</span>
                  )}
                </div>
              </div>
              <div className="flex items-center gap-1 shrink-0">
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-7 w-7 text-muted-foreground hover:text-foreground"
                  onClick={() => openEditForm(course)}
                >
                  <Pencil className="h-3.5 w-3.5" />
                </Button>
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-7 w-7 text-muted-foreground hover:text-destructive"
                  onClick={() => void handleDelete(course.id)}
                >
                  <Trash2 className="h-3.5 w-3.5" />
                </Button>
              </div>
            </AcrylicPanel>
          ))}
        </div>
      )}
    </div>
  )
}
