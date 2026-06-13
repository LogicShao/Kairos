import { useState, type FormEvent } from "react"
import type { Task, CreateTaskRequest, UpdateTaskRequest, TaskPriority, TaskStatus } from "@/types/task"
import { Button } from "@/components/ui/button"
import { Save, X } from "lucide-react"

interface TaskFormProps {
  task?: Task | null
  onSave: (task: CreateTaskRequest | UpdateTaskRequest) => Promise<void>
  onCancel: () => void
}

const PRIORITY_OPTIONS: { value: TaskPriority; label: string }[] = [
  { value: "high", label: "高" },
  { value: "medium", label: "中" },
  { value: "low", label: "低" },
]

const STATUS_OPTIONS: { value: TaskStatus; label: string }[] = [
  { value: "todo", label: "待办" },
  { value: "in_progress", label: "进行中" },
  { value: "done", label: "已完成" },
]

export function TaskForm({ task, onSave, onCancel }: TaskFormProps) {
  const [title, setTitle] = useState(task?.title ?? "")
  const [description, setDescription] = useState(task?.description ?? "")
  const [status, setStatus] = useState<TaskStatus>(task?.status ?? "todo")
  const [priority, setPriority] = useState<TaskPriority>(task?.priority ?? "medium")
  const [dueDate, setDueDate] = useState(task?.due_date ?? "")
  const [tags, setTags] = useState(() => {
    if (!task?.tags) return ""
    try {
      const parsed = JSON.parse(task.tags) as string[]
      return Array.isArray(parsed) ? parsed.join(", ") : ""
    } catch {
      return task.tags
    }
  })
  const [saving, setSaving] = useState(false)

  async function handleSubmit(e: FormEvent) {
    e.preventDefault()
    if (!title.trim()) return

    setSaving(true)
    const tagList = tags
      .split(",")
      .map((t) => t.trim())
      .filter(Boolean)

    const payload: CreateTaskRequest | UpdateTaskRequest = {
      title: title.trim(),
      description: description.trim() || undefined,
      status: task ? status : undefined,
      priority,
      due_date: dueDate || null,
      tags: JSON.stringify(tagList),
    }

    if (!task) {
      ;(payload as CreateTaskRequest).status = status
    }

    try {
      await onSave(payload)
    } finally {
      setSaving(false)
    }
  }

  const isEditing = Boolean(task)

  return (
    <form onSubmit={handleSubmit} className="space-y-4 p-4 rounded-lg border border-border/60 bg-card/60">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-medium text-foreground">
          {isEditing ? "编辑任务" : "新建任务"}
        </h3>
        <Button type="button" variant="ghost" size="icon" onClick={onCancel} className="h-7 w-7">
          <X className="h-4 w-4" />
        </Button>
      </div>

      <div className="space-y-3">
        <div>
          <label className="block text-xs font-medium text-muted-foreground mb-1">
            标题 <span className="text-destructive">*</span>
          </label>
          <input
            type="text"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            required
            placeholder="任务标题"
            className="w-full h-9 rounded-md border border-input bg-background px-3 py-1 text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring focus:border-ring"
          />
        </div>

        <div>
          <label className="block text-xs font-medium text-muted-foreground mb-1">
            描述
          </label>
          <textarea
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            placeholder="任务描述（可选）"
            rows={2}
            className="w-full rounded-md border border-input bg-background px-3 py-1.5 text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring focus:border-ring resize-none"
          />
        </div>

        <div className="grid grid-cols-2 gap-3">
          <div>
            <label className="block text-xs font-medium text-muted-foreground mb-1">
              优先级
            </label>
            <select
              value={priority}
              onChange={(e) => setPriority(e.target.value as TaskPriority)}
              className="w-full h-9 rounded-md border border-input bg-background px-3 py-1 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring focus:border-ring"
            >
              {PRIORITY_OPTIONS.map((opt) => (
                <option key={opt.value} value={opt.value}>{opt.label}</option>
              ))}
            </select>
          </div>

          <div>
            <label className="block text-xs font-medium text-muted-foreground mb-1">
              状态
            </label>
            <select
              value={status}
              onChange={(e) => setStatus(e.target.value as TaskStatus)}
              className="w-full h-9 rounded-md border border-input bg-background px-3 py-1 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring focus:border-ring"
            >
              {STATUS_OPTIONS.map((opt) => (
                <option key={opt.value} value={opt.value}>{opt.label}</option>
              ))}
            </select>
          </div>
        </div>

        <div>
          <label className="block text-xs font-medium text-muted-foreground mb-1">
            截止日期
          </label>
          <input
            type="date"
            value={dueDate}
            onChange={(e) => setDueDate(e.target.value)}
            className="w-full h-9 rounded-md border border-input bg-background px-3 py-1 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring focus:border-ring"
          />
        </div>

        <div>
          <label className="block text-xs font-medium text-muted-foreground mb-1">
            标签（逗号分隔）
          </label>
          <input
            type="text"
            value={tags}
            onChange={(e) => setTags(e.target.value)}
            placeholder="e.g. 学习, 工作, 个人"
            className="w-full h-9 rounded-md border border-input bg-background px-3 py-1 text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring focus:border-ring"
          />
        </div>
      </div>

      <div className="flex items-center justify-end gap-2 pt-1">
        <Button type="button" variant="ghost" size="sm" onClick={onCancel}>
          取消
        </Button>
        <Button type="submit" size="sm" disabled={saving || !title.trim()}>
          <Save className="h-3.5 w-3.5 mr-1" />
          {saving ? "保存中..." : "保存"}
        </Button>
      </div>
    </form>
  )
}
