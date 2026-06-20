import { useState, useEffect, useCallback } from "react"
import { invoke } from "@tauri-apps/api/core"
import type { Task, TaskFilterParams, TaskStatus, TaskPriority } from "@/types/task"
import { Button } from "@/components/ui/button"
import { TaskForm } from "@/components/todo/TaskForm"
import { AcrylicPanel } from "@/components/shared/acrylic-panel"
import { Modal } from "@/components/shared/modal"
import { cn } from "@/lib/utils"
import { Calendar, Circle, CircleDot, CheckCircle2, ChevronDown, Plus, Trash2 } from "lucide-react"

const PRIORITY_CONFIG: Record<TaskPriority, { label: string; className: string }> = {
  high: { label: "高", className: "bg-red-500/15 text-red-400 border-red-500/30" },
  medium: { label: "中", className: "bg-amber-500/15 text-amber-400 border-amber-500/30" },
  low: { label: "低", className: "bg-muted text-muted-foreground border-border" },
}

const STATUS_CONFIG: Record<TaskStatus, { label: string; icon: typeof Circle }> = {
  todo: { label: "待办", icon: Circle },
  in_progress: { label: "进行中", icon: CircleDot },
  done: { label: "已完成", icon: CheckCircle2 },
}

function formatDueDate(dateStr: string | null): { text: string; urgent: boolean } {
  if (!dateStr) return { text: "", urgent: false }
  const due = new Date(dateStr + "T00:00:00")
  const today = new Date()
  today.setHours(0, 0, 0, 0)
  const diff = Math.floor((due.getTime() - today.getTime()) / 86400000)

  if (diff === 0) return { text: "今天", urgent: true }
  if (diff === 1) return { text: "明天", urgent: false }
  if (diff === -1) return { text: "昨天", urgent: true }
  if (diff < 0) return { text: `逾期 ${Math.abs(diff)} 天`, urgent: true }
  return { text: dateStr, urgent: false }
}

export function TaskList() {
  const [tasks, setTasks] = useState<Task[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [showForm, setShowForm] = useState(false)
  const [editingTask, setEditingTask] = useState<Task | null>(null)
  const [statusFilter, setStatusFilter] = useState<string>("")
  const [priorityFilter, setPriorityFilter] = useState<string>("")

  const fetchTasks = useCallback(async () => {
    const filters: TaskFilterParams = {}
    if (statusFilter) filters.status_filter = statusFilter
    if (priorityFilter) filters.priority_filter = priorityFilter
    filters.sort_by = "created_at"
    filters.sort_order = "DESC"

    const result = await invoke<Task[]>("get_all_tasks", { filters })
    setTasks(result)
    setError(null)
  }, [statusFilter, priorityFilter])

  useEffect(() => {
    let cancelled = false
    async function load() {
      try {
        const filters: TaskFilterParams = {}
        if (statusFilter) filters.status_filter = statusFilter
        if (priorityFilter) filters.priority_filter = priorityFilter
        filters.sort_by = "created_at"
        filters.sort_order = "DESC"

        const result = await invoke<Task[]>("get_all_tasks", { filters })
        if (!cancelled) {
          setTasks(result)
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
  }, [statusFilter, priorityFilter])

  async function handleCreate(data: Record<string, unknown>) {
    await invoke("create_task", { cmd: data })
    setShowForm(false)
    await fetchTasks()
  }

  async function handleUpdate(data: Record<string, unknown>) {
    if (!editingTask) return
    await invoke("update_task", { id: editingTask.id, cmd: data })
    setEditingTask(null)
    await fetchTasks()
  }

  async function handleDelete(id: number) {
    await invoke("delete_task", { id })
    await fetchTasks()
  }

  const statusIcon = (status: TaskStatus) => {
    const { icon: Icon, label } = STATUS_CONFIG[status]
    return (
      <span
        className={cn(
          "inline-flex items-center gap-1 text-xs",
          status === "done" ? "text-emerald-400" : status === "in_progress" ? "text-amber-400" : "text-muted-foreground"
        )}
        title={label}
      >
        <Icon className="h-3.5 w-3.5" />
      </span>
    )
  }

  return (
    <div className="w-full max-w-2xl mx-auto space-y-4 animate-in fade-in-0 slide-in-from-bottom-2 duration-300">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-heading font-medium text-foreground">
          待办事项
        </h2>
        <Button
          size="sm"
          onClick={() => { setShowForm(true); setEditingTask(null) }}
          disabled={showForm}
        >
          <Plus className="h-4 w-4 mr-1" />
          新建
        </Button>
      </div>

      <Modal
        open={showForm || editingTask !== null}
        onOpenChange={(open) => {
          if (!open) {
            setShowForm(false)
            setEditingTask(null)
          }
        }}
        title={editingTask ? "编辑任务" : "新建任务"}
        description="填写任务标题、优先级、截止日期等信息"
      >
        <TaskForm
          task={editingTask}
          onSave={(data) => {
            if (editingTask) {
              void handleUpdate(data as unknown as Record<string, unknown>)
            } else {
              void handleCreate(data as unknown as Record<string, unknown>)
            }
          }}
          onCancel={() => {
            setShowForm(false)
            setEditingTask(null)
          }}
        />
      </Modal>

      <div className="flex items-center gap-2">
        <div className="relative flex-1">
          <select
            value={statusFilter}
            onChange={(e) => setStatusFilter(e.target.value)}
            className="w-full h-8 appearance-none rounded-md border border-input bg-background pl-2.5 pr-7 text-xs text-foreground focus:outline-none focus:ring-2 focus:ring-ring focus:border-ring"
          >
            <option value="">全部状态</option>
            <option value="todo">待办</option>
            <option value="in_progress">进行中</option>
            <option value="done">已完成</option>
          </select>
          <ChevronDown className="pointer-events-none absolute right-2 top-1/2 -translate-y-1/2 h-3 w-3 text-muted-foreground" />
        </div>
        <div className="relative flex-1">
          <select
            value={priorityFilter}
            onChange={(e) => setPriorityFilter(e.target.value)}
            className="w-full h-8 appearance-none rounded-md border border-input bg-background pl-2.5 pr-7 text-xs text-foreground focus:outline-none focus:ring-2 focus:ring-ring focus:border-ring"
          >
            <option value="">全部优先级</option>
            <option value="high">高</option>
            <option value="medium">中</option>
            <option value="low">低</option>
          </select>
          <ChevronDown className="pointer-events-none absolute right-2 top-1/2 -translate-y-1/2 h-3 w-3 text-muted-foreground" />
        </div>
      </div>

      {loading && (
        <div className="flex items-center justify-center py-12">
          <div className="animate-spin h-6 w-6 border-2 border-primary border-t-transparent rounded-full" />
        </div>
      )}

      {!loading && error && tasks.length === 0 && (
        <p className="text-center text-sm text-muted-foreground py-8">{error}</p>
      )}

      {!loading && tasks.length === 0 && !error && (
        <AcrylicPanel className="p-8 text-center">
          <p className="text-sm text-muted-foreground">暂无任务</p>
          <p className="text-xs text-muted-foreground/60 mt-1">
            点击"新建"按钮添加第一个任务
          </p>
        </AcrylicPanel>
      )}

      {!loading && tasks.length > 0 && (
        <div className="space-y-2">
          {tasks.map((task) => {
            const dueInfo = formatDueDate(task.due_date)
            const priorityCfg = PRIORITY_CONFIG[task.priority as TaskPriority] ?? PRIORITY_CONFIG.medium

            return (
              <AcrylicPanel
                key={task.id}
                className={cn(
                  "p-3 cursor-pointer transition-all bg-card hover:-translate-y-0.5 hover:bg-card/95 hover:shadow-md",
                  task.status === "done" && "opacity-60"
                )}
                onClick={() => { setEditingTask(task); setShowForm(false) }}
              >
                <div className="flex items-start gap-3">
                  <div className="mt-0.5 shrink-0">
                    {statusIcon(task.status as TaskStatus)}
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2">
                      <span
                        className={cn(
                          "text-sm font-medium truncate",
                          task.status === "done" && "line-through text-muted-foreground"
                        )}
                      >
                        {task.title}
                      </span>
                      <span
                        className={cn(
                          "inline-flex items-center rounded border px-1.5 py-px text-[10px] font-medium shrink-0",
                          priorityCfg.className
                        )}
                      >
                        {priorityCfg.label}
                      </span>
                    </div>
                    {task.description && (
                      <p className="text-xs text-muted-foreground mt-0.5 truncate">
                        {task.description}
                      </p>
                    )}
                    <div className="flex items-center gap-2 mt-1">
                      {dueInfo.text && (
                        <span
                          className={cn(
                            "inline-flex items-center gap-1 text-[10px]",
                            dueInfo.urgent ? "text-red-400" : "text-muted-foreground"
                          )}
                        >
                          <Calendar className="h-3 w-3" />
                          {dueInfo.text}
                        </span>
                      )}
                      {task.tags && (() => {
                        try {
                          const tagArr = JSON.parse(task.tags) as string[]
                          if (!Array.isArray(tagArr) || tagArr.length === 0) return null
                          return (
                            <span className="text-[10px] text-muted-foreground/60">
                              {tagArr.join(" · ")}
                            </span>
                          )
                        } catch {
                          return null
                        }
                      })()}
                    </div>
                  </div>
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-7 w-7 shrink-0 text-muted-foreground hover:text-destructive"
                    onClick={(e) => {
                      e.stopPropagation()
                      void handleDelete(task.id)
                    }}
                  >
                    <Trash2 className="h-3.5 w-3.5" />
                  </Button>
                </div>
              </AcrylicPanel>
            )
          })}
        </div>
      )}
    </div>
  )
}
