import { useState, useEffect, useCallback, useMemo } from "react"
import type { Task, TaskFilterParams, TaskPriority, CreateTaskRequest, UpdateTaskRequest } from "@/types/task"
import {
  listTasks,
  createTask,
  updateTask,
  deleteTask,
  completeDailyTask,
  uncompleteDailyTask,
} from "@/lib/api/tasks"
import { Button } from "@/components/ui/button"
import { TaskForm } from "@/components/todo/TaskForm"
import { AcrylicPanel } from "@/components/shared/acrylic-panel"
import { Modal } from "@/components/shared/modal"
import { Fab } from "@/components/shared/fab"
import { FilterChip } from "@/components/shared/filter-chip"
import { PageShell } from "@/components/shared/page-shell"
import { cn } from "@/lib/utils"
import { userErrorMessage } from "@/lib/errors"
import { Bell, Calendar, CheckCircle2, Circle, ListTodo, Plus, Repeat, Trash2 } from "lucide-react"

interface PriorityConfig {
  label: string
  textClass: string
  dotClass: string
  spineClass: string
}

const PRIORITY_CONFIG: Record<TaskPriority, PriorityConfig> = {
  high: {
    label: "高",
    textClass: "text-red-500 dark:text-red-400",
    dotClass: "bg-red-500/80",
    spineClass: "bg-red-400/80",
  },
  medium: {
    label: "中",
    textClass: "text-amber-600 dark:text-amber-400",
    dotClass: "bg-amber-500/80",
    spineClass: "bg-amber-400/80",
  },
  low: {
    label: "低",
    textClass: "text-muted-foreground",
    dotClass: "bg-muted-foreground/50",
    spineClass: "bg-border",
  },
}

const STATUS_OPTIONS = [
  { value: "", label: "未完成" },
  { value: "todo", label: "待办" },
  { value: "in_progress", label: "进行中" },
  { value: "done", label: "已完成" },
]

const PRIORITY_OPTIONS = [
  { value: "", label: "全部优先级" },
  { value: "high", label: "高" },
  { value: "medium", label: "中" },
  { value: "low", label: "低" },
]

const PRIORITY_RANK: Record<TaskPriority, number> = {
  high: 0,
  medium: 1,
  low: 2,
}

/** +08:00 今日日期 YYYY-MM-DD（与后端 complete_daily_task 的 today_china 一致）。 */
function todayChina(): string {
  const now = new Date()
  const china = new Date(now.getTime() + now.getTimezoneOffset() * 60000 + 8 * 3600000)
  return china.toISOString().slice(0, 10)
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

function parseTaskTags(tags: string): string[] {
  if (!tags) return []
  try {
    const parsed: unknown = JSON.parse(tags)
    return Array.isArray(parsed)
      ? parsed.filter((tag): tag is string => typeof tag === "string")
      : []
  } catch {
    return []
  }
}

export function TaskList() {
  const [tasks, setTasks] = useState<Task[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [showForm, setShowForm] = useState(false)
  const [editingTask, setEditingTask] = useState<Task | null>(null)
  const [statusFilter, setStatusFilter] = useState<string>("")
  const [priorityFilter, setPriorityFilter] = useState<string>("")

  const today = todayChina()

  // 每日任务：不受状态筛选影响、始终全量显示（含今日已完成），按优先级/标题排序。
  const dailyTasks = useMemo(() => {
    return tasks
      .filter((t) => t.is_daily)
      .sort((a, b) => {
        const pa = PRIORITY_RANK[a.priority] ?? PRIORITY_RANK.medium
        const pb = PRIORITY_RANK[b.priority] ?? PRIORITY_RANK.medium
        if (pa !== pb) return pa - pb
        return a.title.localeCompare(b.title)
      })
  }, [tasks])
  const dailyCompleted = dailyTasks.filter((t) => t.last_completed_date === today).length

  // 普通任务：默认隐藏已完成；显式选状态时按状态过滤。排序优先级高 → 低、截止日期早 → 晚。
  const normalTasks = useMemo(() => {
    const base = tasks.filter((t) => !t.is_daily)
    const visible = statusFilter ? base.filter((t) => t.status === statusFilter) : base.filter((t) => t.status !== "done")
    return [...visible].sort((a, b) => {
      const pa = PRIORITY_RANK[a.priority] ?? PRIORITY_RANK.medium
      const pb = PRIORITY_RANK[b.priority] ?? PRIORITY_RANK.medium
      if (pa !== pb) return pa - pb
      if (a.due_date && b.due_date) return a.due_date.localeCompare(b.due_date)
      if (a.due_date) return -1
      if (b.due_date) return 1
      return 0
    })
  }, [tasks, statusFilter])

  // 状态筛选在前端做（每日任务需要全量数据）；优先级筛选交给后端。
  const fetchTasks = useCallback(async () => {
    const filters: TaskFilterParams = {}
    if (priorityFilter) filters.priority_filter = priorityFilter
    const result = await listTasks(filters)
    setTasks(result)
    setError(null)
  }, [priorityFilter])

  useEffect(() => {
    let cancelled = false
    async function load() {
      try {
        const filters: TaskFilterParams = {}
        if (priorityFilter) filters.priority_filter = priorityFilter
        const result = await listTasks(filters)
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
  }, [priorityFilter])

  async function handleCreate(data: CreateTaskRequest) {
    try {
      await createTask(data)
      setShowForm(false)
      await fetchTasks()
    } catch (e) {
      setError(userErrorMessage(e, "创建任务失败"))
    }
  }

  async function handleUpdate(data: UpdateTaskRequest) {
    if (!editingTask) return
    try {
      await updateTask(editingTask.id, data)
      setEditingTask(null)
      await fetchTasks()
    } catch (e) {
      setError(userErrorMessage(e, "更新任务失败"))
    }
  }

  async function handleDelete(id: number) {
    try {
      await deleteTask(id)
      await fetchTasks()
    } catch (e) {
      setError(userErrorMessage(e, "删除任务失败"))
    }
  }

  // 每日任务完成/取消：按 today 判断今日状态，走专用命令。
  async function handleToggleDaily(task: Task) {
    const completedToday = task.last_completed_date === today
    try {
      if (completedToday) {
        await uncompleteDailyTask(task.id)
      } else {
        await completeDailyTask(task.id)
      }
      await fetchTasks()
    } catch (e) {
      setError(userErrorMessage(e, "更新每日任务失败"))
    }
  }

  // 普通任务一次性完成。
  async function handleComplete(task: Task) {
    try {
      await updateTask(task.id, { status: "done" })
      await fetchTasks()
    } catch (e) {
      setError(userErrorMessage(e, "完成任务失败"))
    }
  }

  function openNewForm() {
    setEditingTask(null)
    setShowForm(true)
  }

  const taskSummary = dailyTasks.length > 0
    ? `${normalTasks.length} 项任务 · 每日 ${dailyCompleted}/${dailyTasks.length} 已完成`
    : `${normalTasks.length} 项任务`

  return (
    <PageShell
      title="待办事项"
      width="2xl"
      titleClassName="text-xl font-heading font-semibold"
      subtitle={taskSummary}
    >
      <Modal
        open={showForm || editingTask !== null}
        onOpenChange={(open) => {
          if (!open) {
            setShowForm(false)
            setEditingTask(null)
          }
        }}
        variant="center"
        title={editingTask ? "编辑任务" : "新建任务"}
        description="填写任务标题、优先级、截止日期等信息"
      >
        <TaskForm
          task={editingTask}
          onCreate={handleCreate}
          onUpdate={handleUpdate}
          onCancel={() => {
            setShowForm(false)
            setEditingTask(null)
          }}
        />
      </Modal>

      <Fab onClick={openNewForm} />

      {/* 胶囊筛选 */}
      <div className="mb-1 flex flex-wrap items-center gap-2">
        <FilterChip
          label="状态"
          options={STATUS_OPTIONS}
          value={statusFilter}
          onChange={setStatusFilter}
        />
        <FilterChip
          label="优先级"
          options={PRIORITY_OPTIONS}
          value={priorityFilter}
          onChange={setPriorityFilter}
        />
      </div>

      {error && tasks.length > 0 && (
        <p className="mb-3 text-center text-sm text-destructive">{error}</p>
      )}

      {loading && (
        <div className="flex items-center justify-center py-12">
          <div className="animate-spin h-6 w-6 border-2 border-primary border-t-transparent rounded-full" />
        </div>
      )}

      {!loading && error && tasks.length === 0 && (
        <AcrylicPanel className="p-6">
          <div className="flex flex-col items-center gap-3 text-center">
            <span className="flex h-11 w-11 items-center justify-center rounded-full bg-muted text-muted-foreground">
              <ListTodo className="h-5 w-5" />
            </span>
            <div>
              <p className="text-sm font-medium text-foreground">无法加载待办</p>
              <p className="mt-1 text-xs text-muted-foreground">{error}</p>
            </div>
            <Button variant="outline" size="sm" onClick={() => void fetchTasks()}>
              重试
            </Button>
          </div>
        </AcrylicPanel>
      )}

      {/* 空状态：无卡片，直接显示在背景上 */}
      {!loading && tasks.length === 0 && !error && (
        <div className="flex flex-col items-center justify-center gap-4 rounded-2xl border border-dashed border-border/60 bg-card/40 px-6 py-14">
          <span className="flex h-11 w-11 items-center justify-center rounded-full bg-muted text-muted-foreground">
            <ListTodo className="h-5 w-5" strokeWidth={1.5} />
          </span>
          <div className="space-y-1 text-center">
            <p className="text-sm font-medium text-foreground">还没有待办任务</p>
            <p className="text-xs text-muted-foreground">把今天要做的事写下来，从一件开始。</p>
          </div>
          <Button size="sm" className="min-h-11 md:min-h-8" onClick={openNewForm}>
            <Plus className="mr-1 h-4 w-4" />添加任务
          </Button>
        </div>
      )}

      {!loading && tasks.length > 0 && (
        <div className="space-y-5">
          {/* 每日任务区块 */}
          {dailyTasks.length > 0 && (
            <div>
              <div className="mb-2.5 flex items-center gap-2.5">
                <span className="flex h-7 w-7 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
                  <Repeat className="h-3.5 w-3.5" />
                </span>
                <h3 className="text-[13px] font-semibold text-foreground">每日任务</h3>
                <span className="text-[10px] text-muted-foreground tabular-nums">
                  {dailyCompleted}/{dailyTasks.length}
                </span>
                <div className="ml-auto h-1.5 w-20 overflow-hidden rounded-full bg-muted md:w-28">
                  <div
                    className="h-full rounded-full bg-primary transition-[width] duration-300"
                    style={{
                      width: `${dailyTasks.length > 0 ? (dailyCompleted / dailyTasks.length) * 100 : 0}%`,
                    }}
                  />
                </div>
              </div>
              <div className="space-y-2">
                {dailyTasks.map((task) => {
                  const completedToday = task.last_completed_date === today
                  const priorityCfg = PRIORITY_CONFIG[task.priority] ?? PRIORITY_CONFIG.medium
                  const tags = parseTaskTags(task.tags)
                  return (
                    <AcrylicPanel
                      key={task.id}
                      className={cn(
                        "group relative cursor-pointer overflow-hidden bg-card/70 p-3 pl-4 transition-all hover:-translate-y-0.5 hover:bg-card hover:shadow-md",
                        completedToday && "bg-card/50",
                      )}
                      onClick={() => { setEditingTask(task); setShowForm(false) }}
                    >
                      <span
                        className="absolute inset-y-2.5 left-0 w-[3px] rounded-r-full bg-primary/70"
                        aria-hidden="true"
                      />
                      <div className="flex items-start gap-3">
                        <div className="shrink-0">
                          <button
                            type="button"
                            onClick={(e) => {
                              e.stopPropagation()
                              void handleToggleDaily(task)
                            }}
                            aria-label={completedToday ? "取消今日完成" : "完成今日任务"}
                            className="flex h-11 w-11 items-center justify-center rounded-full transition-colors hover:bg-primary/5 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring md:h-6 md:w-6"
                          >
                            {completedToday ? (
                              <CheckCircle2 className="h-5 w-5 text-emerald-500" />
                            ) : (
                              <Circle
                                className="h-5 w-5 text-muted-foreground/40 transition-colors hover:text-primary"
                                strokeWidth={1.5}
                              />
                            )}
                          </button>
                        </div>
                        <div className="min-w-0 flex-1">
                          <div className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1">
                            <span
                              className={cn(
                                "min-w-0 truncate text-[13px] font-medium",
                                completedToday && "text-muted-foreground line-through",
                              )}
                            >
                              {task.title}
                            </span>
                            <span
                              className={cn(
                                "shrink-0 rounded-full px-1.5 py-0.5 text-[9px] font-medium leading-none",
                                completedToday
                                  ? "bg-emerald-500/10 text-emerald-600 dark:text-emerald-400"
                                  : "bg-primary/10 text-primary",
                              )}
                            >
                              {completedToday ? "今日已完成" : "今日待完成"}
                            </span>
                            <span
                              className={cn(
                                "inline-flex shrink-0 items-center gap-1 text-[10px] font-medium",
                                priorityCfg.textClass,
                              )}
                            >
                              <span className={cn("h-1.5 w-1.5 rounded-full", priorityCfg.dotClass)} />
                              {priorityCfg.label}
                            </span>
                          </div>
                          {task.description && (
                            <p className="mt-1 truncate text-[11px] text-muted-foreground">
                              {task.description}
                            </p>
                          )}
                          <div className="mt-1.5 flex flex-wrap items-center gap-x-3 gap-y-1 text-[10px] text-muted-foreground">
                            {task.reminder_time && (
                              <span className="inline-flex items-center gap-1">
                                <Bell className="h-3 w-3" />
                                每天 {task.reminder_time} 提醒
                              </span>
                            )}
                            {tags.length > 0 && (
                              <span className="text-muted-foreground/60">
                                {tags.join(" · ")}
                              </span>
                            )}
                          </div>
                        </div>
                        <Button
                          variant="ghost"
                          size="icon"
                          className="h-11 w-11 shrink-0 text-muted-foreground/50 hover:text-destructive md:h-7 md:w-7 md:opacity-0 md:group-hover:opacity-100 md:focus-visible:opacity-100"
                          aria-label={`删除任务 ${task.title}`}
                          onClick={(e) => {
                            e.stopPropagation()
                            void handleDelete(task.id)
                          }}
                        >
                          <Trash2 className="h-4 w-4 sm:h-3.5 sm:w-3.5" />
                        </Button>
                      </div>
                    </AcrylicPanel>
                  )
                })}
              </div>
            </div>
          )}

          {/* 普通任务区块 */}
          {normalTasks.length > 0 && (
            <div>
              {dailyTasks.length > 0 && (
                <div className="mb-2.5 flex items-center gap-2.5">
                  <span className="flex h-7 w-7 shrink-0 items-center justify-center rounded-lg bg-muted text-muted-foreground">
                    <ListTodo className="h-3.5 w-3.5" />
                  </span>
                  <h3 className="text-[13px] font-semibold text-foreground">普通任务</h3>
                  <span className="text-[10px] text-muted-foreground tabular-nums">
                    {normalTasks.length}
                  </span>
                </div>
              )}
              <div className="space-y-2">
                {normalTasks.map((task) => {
                  const dueInfo = formatDueDate(task.due_date)
                  const priorityCfg = PRIORITY_CONFIG[task.priority] ?? PRIORITY_CONFIG.medium
                  const tags = parseTaskTags(task.tags)
                  const isDone = task.status === "done"

                  return (
                    <AcrylicPanel
                      key={task.id}
                      className={cn(
                        "group relative cursor-pointer overflow-hidden bg-card/70 p-3 pl-4 transition-all hover:-translate-y-0.5 hover:bg-card hover:shadow-md",
                        isDone && "bg-card/50",
                      )}
                      onClick={() => { setEditingTask(task); setShowForm(false) }}
                    >
                      <span
                        className={cn(
                          "absolute inset-y-2.5 left-0 w-[3px] rounded-r-full",
                          priorityCfg.spineClass,
                        )}
                        aria-hidden="true"
                      />
                      <div className="flex items-start gap-3">
                        <div className="shrink-0">
                          <button
                            type="button"
                            onClick={(e) => {
                              e.stopPropagation()
                              if (isDone) return
                              void handleComplete(task)
                            }}
                            disabled={isDone}
                            aria-label={isDone ? "已完成" : "完成任务"}
                            className="flex h-11 w-11 items-center justify-center rounded-full transition-colors hover:bg-primary/5 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-default md:h-6 md:w-6"
                          >
                            {isDone ? (
                              <CheckCircle2 className="h-5 w-5 text-emerald-500" />
                            ) : (
                              <Circle
                                className="h-5 w-5 text-muted-foreground/40 transition-colors hover:text-primary"
                                strokeWidth={1.5}
                              />
                            )}
                          </button>
                        </div>
                        <div className="min-w-0 flex-1">
                          <div className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1">
                            <span
                              className={cn(
                                "min-w-0 truncate text-[13px] font-medium",
                                isDone && "text-muted-foreground line-through",
                              )}
                            >
                              {task.title}
                            </span>
                            {task.status === "in_progress" && (
                              <span className="shrink-0 rounded-full bg-primary/10 px-1.5 py-0.5 text-[9px] font-medium leading-none text-primary">
                                进行中
                              </span>
                            )}
                            <span
                              className={cn(
                                "inline-flex shrink-0 items-center gap-1 text-[10px] font-medium",
                                priorityCfg.textClass,
                              )}
                            >
                              <span
                                className={cn("h-1.5 w-1.5 rounded-full", priorityCfg.dotClass)}
                              />
                              {priorityCfg.label}
                            </span>
                          </div>
                          {task.description && (
                            <p className="mt-1 truncate text-[11px] text-muted-foreground">
                              {task.description}
                            </p>
                          )}
                          <div className="mt-1.5 flex flex-wrap items-center gap-x-3 gap-y-1 text-[10px] text-muted-foreground">
                            {dueInfo.text && (
                              <span
                                className={cn(
                                  "inline-flex items-center gap-1",
                                  dueInfo.urgent
                                    ? "text-red-500 dark:text-red-400"
                                    : "text-muted-foreground",
                                )}
                              >
                                <Calendar className="h-3 w-3" />
                                {dueInfo.text}
                              </span>
                            )}
                            {tags.length > 0 && (
                              <span className="text-muted-foreground/60">
                                {tags.join(" · ")}
                              </span>
                            )}
                          </div>
                        </div>
                        <Button
                          variant="ghost"
                          size="icon"
                          className="h-11 w-11 shrink-0 text-muted-foreground/50 hover:text-destructive md:h-7 md:w-7 md:opacity-0 md:group-hover:opacity-100 md:focus-visible:opacity-100"
                          aria-label={`删除任务 ${task.title}`}
                          onClick={(e) => {
                            e.stopPropagation()
                            void handleDelete(task.id)
                          }}
                        >
                          <Trash2 className="h-4 w-4 sm:h-3.5 sm:w-3.5" />
                        </Button>
                      </div>
                    </AcrylicPanel>
                  )
                })}
              </div>
            </div>
          )}
        </div>
      )}
    </PageShell>
  )
}
