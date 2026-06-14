import { useState, useEffect } from "react"
import { invoke } from "@tauri-apps/api/core"
import type { Exam, CreateExamRequest, UpdateExamRequest } from "@/types/exam"
import { Button } from "@/components/ui/button"
import { AcrylicPanel } from "@/components/shared/acrylic-panel"
import { cn } from "@/lib/utils"
import { Plus, Trash2, Pencil, X, Save, Clock, MapPin, FileText } from "lucide-react"

function computeDaysUntil(datetimeStr: string): number {
  const target = new Date(datetimeStr)
  const now = new Date()
  const diffMs = target.getTime() - now.getTime()
  return Math.ceil(diffMs / (1000 * 60 * 60 * 24))
}

function daysBadgeClass(days: number): string {
  if (days < 0) return "bg-muted text-muted-foreground"
  if (days <= 7) return "bg-red-500/15 text-red-400 border-red-500/30"
  if (days <= 14) return "bg-amber-500/15 text-amber-400 border-amber-500/30"
  return "bg-emerald-500/15 text-emerald-400 border-emerald-500/30"
}

function daysLabel(days: number): string {
  if (days < 0) return `已过 ${Math.abs(days)} 天`
  if (days === 0) return "今天"
  if (days === 1) return "明天"
  return `剩余 ${days} 天`
}

function formatDate(datetimeStr: string): string {
  const d = new Date(datetimeStr)
  const year = d.getFullYear()
  const month = String(d.getMonth() + 1).padStart(2, "0")
  const day = String(d.getDate()).padStart(2, "0")
  const hour = String(d.getHours()).padStart(2, "0")
  const min = String(d.getMinutes()).padStart(2, "0")
  return `${year}-${month}-${day} ${hour}:${min}`
}

interface ExamFormData {
  course_name: string
  exam_datetime: string
  location: string
  notes: string
  course_id: number | null
}

function emptyForm(): ExamFormData {
  return {
    course_name: "",
    exam_datetime: "",
    location: "",
    notes: "",
    course_id: null,
  }
}

function examToForm(e: Exam): ExamFormData {
  return {
    course_name: e.course_name,
    exam_datetime: e.exam_datetime,
    location: e.location,
    notes: e.notes,
    course_id: e.course_id,
  }
}

function formatDatetimeForInput(isoStr: string): string {
  const d = new Date(isoStr)
  if (isNaN(d.getTime())) return ""
  const year = d.getFullYear()
  const month = String(d.getMonth() + 1).padStart(2, "0")
  const day = String(d.getDate()).padStart(2, "0")
  const hour = String(d.getHours()).padStart(2, "0")
  const min = String(d.getMinutes()).padStart(2, "0")
  return `${year}-${month}-${day}T${hour}:${min}`
}

export function ExamList() {
  const [exams, setExams] = useState<Exam[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [showForm, setShowForm] = useState(false)
  const [editingExam, setEditingExam] = useState<Exam | null>(null)
  const [form, setForm] = useState<ExamFormData>(emptyForm())
  const [saving, setSaving] = useState(false)

  async function fetchExams() {
    const result = await invoke<Exam[]>("get_all_exams")
    const enriched = result.map((e) => ({
      ...e,
      days_until: computeDaysUntil(e.exam_datetime),
    }))
    setExams(enriched)
    setError(null)
  }

  useEffect(() => {
    let cancelled = false
    async function load() {
      try {
        const result = await invoke<Exam[]>("get_all_exams")
        const enriched = result.map((e) => ({
          ...e,
          days_until: computeDaysUntil(e.exam_datetime),
        }))
        if (!cancelled) {
          setExams(enriched)
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
  }, [])

  function openCreateForm() {
    setForm(emptyForm())
    setEditingExam(null)
    setShowForm(true)
  }

  function openEditForm(exam: Exam) {
    setForm(examToForm(exam))
    setEditingExam(exam)
    setShowForm(true)
  }

  async function handleSave() {
    if (!form.course_name.trim() || !form.exam_datetime) return

    setSaving(true)
    try {
      if (editingExam) {
        const payload: UpdateExamRequest = {
          course_name: form.course_name,
          exam_datetime: form.exam_datetime,
          location: form.location,
          notes: form.notes,
          course_id: form.course_id,
        }
        await invoke("update_exam", { id: editingExam.id, cmd: payload })
      } else {
        const payload: CreateExamRequest = {
          course_name: form.course_name,
          exam_datetime: form.exam_datetime,
          location: form.location || undefined,
          notes: form.notes || undefined,
          course_id: form.course_id || undefined,
        }
        await invoke("create_exam", { cmd: payload })
      }
      setShowForm(false)
      setEditingExam(null)
      await fetchExams()
    } finally {
      setSaving(false)
    }
  }

  async function handleDelete(id: number) {
    await invoke("delete_exam", { id })
    await fetchExams()
  }

  const upcomingExams = exams.filter((e) => (e.days_until ?? 0) >= 0)
  const pastExams = exams.filter((e) => (e.days_until ?? 0) < 0)

  return (
    <div className="w-full max-w-2xl mx-auto space-y-4">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-heading font-medium text-foreground">
          考试倒计时
        </h2>
        <Button size="sm" onClick={openCreateForm} disabled={showForm}>
          <Plus className="h-4 w-4 mr-1" />
          添加考试
        </Button>
      </div>

      {showForm && (
        <AcrylicPanel className="p-4 bg-card">
          <div className="flex items-center justify-between mb-4">
            <h3 className="text-sm font-medium text-foreground">
              {editingExam ? "编辑考试" : "新建考试"}
            </h3>
            <Button
              type="button"
              variant="ghost"
              size="icon"
              className="h-7 w-7"
              onClick={() => { setShowForm(false); setEditingExam(null) }}
            >
              <X className="h-4 w-4" />
            </Button>
          </div>

          <div className="grid grid-cols-2 gap-3 mb-4">
            <div className="col-span-2">
              <label className="block text-xs font-medium text-muted-foreground mb-1">
                考试名称 <span className="text-destructive">*</span>
              </label>
              <input
                type="text"
                value={form.course_name}
                onChange={(e) => setForm({ ...form, course_name: e.target.value })}
                required
                placeholder="e.g. 高数期末考试"
                className="w-full h-9 rounded-md border border-input bg-background px-3 py-1 text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring focus:border-ring"
              />
            </div>
            <div>
              <label className="block text-xs font-medium text-muted-foreground mb-1">
                考试时间 <span className="text-destructive">*</span>
              </label>
              <input
                type="datetime-local"
                value={form.exam_datetime ? formatDatetimeForInput(form.exam_datetime) : ""}
                onChange={(e) => setForm({ ...form, exam_datetime: e.target.value ? new Date(e.target.value).toISOString() : "" })}
                className="w-full h-9 rounded-md border border-input bg-background px-3 py-1 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring focus:border-ring"
              />
            </div>
            <div>
              <label className="block text-xs font-medium text-muted-foreground mb-1">地点</label>
              <input
                type="text"
                value={form.location}
                onChange={(e) => setForm({ ...form, location: e.target.value })}
                placeholder="考场/教室"
                className="w-full h-9 rounded-md border border-input bg-background px-3 py-1 text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring focus:border-ring"
              />
            </div>
          </div>

          <div>
            <label className="block text-xs font-medium text-muted-foreground mb-1">备注</label>
            <textarea
              value={form.notes}
              onChange={(e) => setForm({ ...form, notes: e.target.value })}
              placeholder="考试注意事项（可选）"
              rows={2}
              className="w-full rounded-md border border-input bg-background px-3 py-1.5 text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring focus:border-ring resize-none"
            />
          </div>

          <div className="flex items-center justify-end gap-2 pt-3">
            <Button type="button" variant="ghost" size="sm" onClick={() => { setShowForm(false); setEditingExam(null) }}>
              取消
            </Button>
            <Button type="button" size="sm" disabled={saving || !form.course_name.trim() || !form.exam_datetime} onClick={handleSave}>
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

      {!loading && error && exams.length === 0 && (
        <p className="text-center text-sm text-muted-foreground py-8">{error}</p>
      )}

      {!loading && exams.length === 0 && !error && (
        <AcrylicPanel className="p-8 text-center">
          <p className="text-sm text-muted-foreground">暂无考试</p>
          <p className="text-xs text-muted-foreground/60 mt-1">
            点击"添加考试"按钮添加第一个考试
          </p>
        </AcrylicPanel>
      )}

      {!loading && upcomingExams.length > 0 && (
        <div className="space-y-2">
          <h3 className="text-xs font-medium text-muted-foreground uppercase tracking-wider">即将到来</h3>
          {upcomingExams.map((exam) => (
            <ExamCard
              key={exam.id}
              exam={exam}
              onEdit={() => openEditForm(exam)}
              onDelete={() => void handleDelete(exam.id)}
            />
          ))}
        </div>
      )}

      {!loading && pastExams.length > 0 && (
        <div className="space-y-2">
          <h3 className="text-xs font-medium text-muted-foreground uppercase tracking-wider mt-6">已结束</h3>
          {pastExams.map((exam) => (
            <ExamCard
              key={exam.id}
              exam={exam}
              onEdit={() => openEditForm(exam)}
              onDelete={() => void handleDelete(exam.id)}
            />
          ))}
        </div>
      )}
    </div>
  )
}

function ExamCard({
  exam,
  onEdit,
  onDelete,
}: {
  exam: Exam
  onEdit: () => void
  onDelete: () => void
}) {
  const days = exam.days_until ?? 0
  const isPast = days < 0

  return (
    <AcrylicPanel
      className={cn(
        "p-3 bg-card transition-colors hover:bg-card/95",
        isPast && "opacity-50"
      )}
    >
      <div className="flex items-start gap-3">
        <div
          className={cn(
            "inline-flex items-center shrink-0 rounded-md border px-2 py-1 text-xs font-semibold tabular-nums",
            daysBadgeClass(days)
          )}
        >
          {daysLabel(days)}
        </div>

        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <span className="text-sm font-medium text-foreground truncate">
              {exam.course_name}
            </span>
          </div>

          <div className="flex items-center gap-3 mt-1">
            <span className="inline-flex items-center gap-1 text-[11px] text-muted-foreground">
              <Clock className="h-3 w-3" />
              {formatDate(exam.exam_datetime)}
            </span>
            {exam.location && (
              <span className="inline-flex items-center gap-1 text-[11px] text-muted-foreground">
                <MapPin className="h-3 w-3" />
                {exam.location}
              </span>
            )}
          </div>

          {exam.notes && (
            <div className="flex items-start gap-1 mt-1">
              <FileText className="h-3 w-3 text-muted-foreground/50 mt-0.5 shrink-0" />
              <p className="text-xs text-muted-foreground/70">{exam.notes}</p>
            </div>
          )}
        </div>

        <div className="flex items-center gap-1 shrink-0">
          <Button
            variant="ghost"
            size="icon"
            className="h-7 w-7 text-muted-foreground hover:text-foreground"
            onClick={onEdit}
          >
            <Pencil className="h-3.5 w-3.5" />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            className="h-7 w-7 text-muted-foreground hover:text-destructive"
            onClick={onDelete}
          >
            <Trash2 className="h-3.5 w-3.5" />
          </Button>
        </div>
      </div>
    </AcrylicPanel>
  )
}
