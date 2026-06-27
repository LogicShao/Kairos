import { useState, useEffect, useCallback, useRef } from "react"
import { invoke } from "@tauri-apps/api/core"
import { listen, type UnlistenFn } from "@tauri-apps/api/event"
import { ArrowLeft, RefreshCw, Check, X } from "lucide-react"
import type { SyncConfig, SyncResult } from "@/types/sync"
import { Button } from "@/components/ui/button"
import { AcrylicPanel } from "@/components/shared/acrylic-panel"
import { cn } from "@/lib/utils"

interface SyncSettingsProps {
  onNavigate: (key: string) => void
}

function inputClass(hasError: boolean): string {
  return cn(
    "w-full rounded-lg border bg-muted/40 px-3 py-2 text-sm outline-none transition-colors",
    "placeholder:text-muted-foreground/45",
    "focus:ring-2 focus:ring-primary/30",
    hasError
      ? "border-destructive/60 focus:border-destructive focus:ring-destructive/20"
      : "border-border focus:border-primary",
  )
}

export function SyncSettings({ onNavigate }: SyncSettingsProps) {
  const [config, setConfig] = useState<SyncConfig | null>(null)
  const [serverUrl, setServerUrl] = useState("")
  const [username, setUsername] = useState("")
  const [password, setPassword] = useState("")
  const [autoSync, setAutoSync] = useState(false)
  const [lastSyncAt, setLastSyncAt] = useState<string | null>(null)

  const [testStatus, setTestStatus] = useState<"idle" | "loading" | "ok" | "fail">("idle")
  const [testMessage, setTestMessage] = useState("")

  const [syncStatus, setSyncStatus] = useState<"idle" | "loading" | "done">("idle")
  const [syncResult, setSyncResult] = useState<SyncResult | null>(null)
  const [syncError, setSyncError] = useState<string | null>(null)
  const latestDraftRef = useRef<SyncConfig | null>(null)
  const lastPersistedRef = useRef<SyncConfig | null>(null)

  // ── 初始加载 ──────────────────────────────────────────
  useEffect(() => {
    invoke<SyncConfig>("get_sync_config")
      .then((cfg) => {
        setConfig(cfg)
        setServerUrl(cfg.server_url)
        setUsername(cfg.username)
        setPassword(cfg.password)
        setAutoSync(cfg.auto_sync)
        setLastSyncAt(cfg.last_sync_at)
        latestDraftRef.current = cfg
        lastPersistedRef.current = cfg
      })
      .catch(console.error)
  }, [])

  // ── 监听后端同步完成事件（手动 + 自动） ──────────────────
  useEffect(() => {
    let unlisten: UnlistenFn | undefined

    listen<{ last_sync_at: string }>("sync-finished", (event) => {
      setLastSyncAt(event.payload.last_sync_at)
    })
      .then((fn) => {
        unlisten = fn
      })
      .catch(console.error)

    return () => {
      unlisten?.()
    }
  }, [])

  // ── 顺序化保存配置：先写库，返回 Promise<SyncConfig> ──────
  const saveConfig = useCallback(
    async (overrides?: Partial<SyncConfig>): Promise<SyncConfig> => {
      if (!config) {
        throw new Error("Config not loaded")
      }
      const updated: SyncConfig = {
        ...config,
        server_url: serverUrl,
        username,
        password,
        auto_sync: autoSync,
        ...overrides,
      }
      await invoke("update_sync_config", { config: updated })
      setConfig(updated)
      lastPersistedRef.current = updated
      latestDraftRef.current = updated
      return updated
    },
    [config, serverUrl, username, password, autoSync],
  )

  useEffect(() => {
    if (!config) return

    latestDraftRef.current = {
      ...config,
      server_url: serverUrl,
      username,
      password,
      auto_sync: autoSync,
    }
  }, [config, serverUrl, username, password, autoSync])

  useEffect(() => {
    return () => {
      const draft = latestDraftRef.current
      const persisted = lastPersistedRef.current
      if (!draft || !persisted) return

      const hasUnsavedChanges =
        draft.server_url !== persisted.server_url ||
        draft.username !== persisted.username ||
        draft.password !== persisted.password ||
        draft.auto_sync !== persisted.auto_sync

      if (!hasUnsavedChanges) return

      void invoke("update_sync_config", { config: draft }).then(() => {
        lastPersistedRef.current = draft
      })
    }
  }, [])

  // ── 测试连接：先保存最新配置，再测试 ────────────────────
  const handleTestConnection = useCallback(async () => {
    setTestStatus("loading")
    setTestMessage("")
    try {
      await saveConfig()
      const ok = await invoke<boolean>("test_sync_connection")
      setTestStatus(ok ? "ok" : "fail")
      setTestMessage(ok ? "连接成功" : "无法连接到服务器")
    } catch (err) {
      setTestStatus("fail")
      setTestMessage(typeof err === "string" ? err : "连接失败")
    }
  }, [saveConfig])

  // ── 立即同步：先保存最新配置，再同步 ────────────────────
  const handleSyncNow = useCallback(async () => {
    setSyncStatus("loading")
    setSyncResult(null)
    setSyncError(null)
    try {
      await saveConfig()
      const result = await invoke<SyncResult>("sync_now")
      setSyncResult(result)
      setSyncStatus("done")
      // 同步成功后重新读取 last_sync_at（避免前端猜测时间）
      const cfg = await invoke<SyncConfig>("get_sync_config")
      setConfig(cfg)
      setLastSyncAt(cfg.last_sync_at)
    } catch (err) {
      setSyncError(typeof err === "string" ? err : "同步失败")
      setSyncStatus("done")
    }
  }, [saveConfig])

  // ── 自动同步开关：先保存状态，再触发后端启停 ─────────────
  const handleAutoSyncToggle = useCallback(async () => {
    const nextAutoSync = !autoSync
    setAutoSync(nextAutoSync)
    try {
      await saveConfig({ auto_sync: nextAutoSync })
    } catch (err) {
      // 保存失败时回退 UI 状态
      setAutoSync(!nextAutoSync)
      console.error("Failed to save auto_sync config:", err)
    }
  }, [autoSync, saveConfig])

  const showPassword = password.length > 0
  const passwordMasked = showPassword ? "●".repeat(Math.min(password.length, 12)) : ""

  return (
    <div className="min-h-0 flex-1 overflow-y-auto pb-4">
      <div className="mx-auto flex w-full max-w-md flex-col gap-4 animate-in fade-in-0 slide-in-from-bottom-2 duration-300">
        <div className="flex items-start gap-2">
          <Button
            size="icon-sm"
            variant="ghost"
            onClick={() => onNavigate("kairos")}
            aria-label="返回 Kairos"
            className="mt-0.5 shrink-0"
          >
            <ArrowLeft className="h-4 w-4" />
          </Button>
          <div className="min-w-0">
            <h2 className="text-lg font-semibold text-foreground">同步设置</h2>
            <p className="mt-1 text-sm text-muted-foreground">
              配置 WebDAV 备份与多端同步，保护本地学习数据。
            </p>
          </div>
        </div>

        <AcrylicPanel className="border-primary/15 p-4 sm:p-6">
          <h2 className="mb-4 text-lg font-semibold text-foreground">
            WebDAV 连接
          </h2>

          <div className="flex flex-col gap-4">
            <div className="mb-3">
              <label className="mb-0.5 block text-sm text-muted-foreground">
                服务器地址
              </label>
              <input
                type="text"
                className={inputClass(false)}
                placeholder="webdav.example.com"
                value={serverUrl}
                onChange={(e) => setServerUrl(e.target.value)}
              />
            </div>

            <div className="mb-3">
              <label className="mb-0.5 block text-sm text-muted-foreground">
                用户名
              </label>
              <input
                type="text"
                className={inputClass(false)}
                placeholder="username"
                value={username}
                onChange={(e) => setUsername(e.target.value)}
              />
            </div>

            <div className="mb-3">
              <label className="mb-0.5 block text-sm text-muted-foreground">
                密码
              </label>
              <input
                type="password"
                className={inputClass(false)}
                placeholder={passwordMasked || "••••••••"}
                value={password}
                onChange={(e) => setPassword(e.target.value)}
              />
            </div>

            <div className="flex items-center justify-between pt-1">
              <span className="text-sm text-muted-foreground">自动同步</span>
              <button
                type="button"
                role="switch"
                aria-checked={autoSync}
                onClick={handleAutoSyncToggle}
                className={cn(
                  "relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors",
                  "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2",
                  autoSync ? "bg-primary" : "bg-muted-foreground/25",
                )}
              >
                <span
                  className={cn(
                    "pointer-events-none block h-5 w-5 rounded-full bg-card shadow-sm ring-0 transition-transform",
                    autoSync ? "translate-x-5" : "translate-x-0",
                  )}
                />
              </button>
            </div>

            <div className="flex flex-col gap-3 pt-2 sm:flex-row">
              <Button
                variant="outline"
                onClick={handleTestConnection}
                disabled={testStatus === "loading" || !serverUrl}
                className="min-h-11 flex-1 border-primary/40 text-primary hover:bg-primary/5 md:min-h-0"
              >
                {testStatus === "loading" ? (
                  <RefreshCw className="mr-1 h-4 w-4 animate-spin" />
                ) : testStatus === "ok" ? (
                  <Check className="mr-1 h-4 w-4 text-emerald-400" />
                ) : testStatus === "fail" ? (
                  <X className="mr-1 h-4 w-4 text-destructive" />
                ) : null}
                测试连接
              </Button>

              <Button
                onClick={handleSyncNow}
                disabled={syncStatus === "loading" || !serverUrl}
                className="min-h-11 flex-1 md:min-h-0"
              >
                {syncStatus === "loading" ? (
                  <RefreshCw className="mr-1 h-4 w-4 animate-spin" />
                ) : null}
                立即同步
              </Button>
            </div>
          </div>
        </AcrylicPanel>

        {testMessage && (
          <p
            className={cn(
              "text-center text-sm",
              testStatus === "ok" ? "text-emerald-400" : "text-destructive",
            )}
          >
            {testMessage}
          </p>
        )}

        {lastSyncAt && (
          <p className="text-center text-xs text-muted-foreground">
            上次同步: {new Date(lastSyncAt).toLocaleString()}
          </p>
        )}

        {syncResult && (
          <AcrylicPanel className="p-4">
            <h3 className="mb-2 text-sm font-medium text-foreground">同步结果</h3>
            <div className="space-y-1 text-xs text-muted-foreground">
              <p>任务: {syncResult.stats.tasks_merged} 条已合并</p>
              <p>课程: {syncResult.stats.courses_merged} 条已合并</p>
              <p>考试: {syncResult.stats.exams_merged} 条已合并</p>
              <p>番茄记录: {syncResult.stats.sessions_merged} 条已合并</p>
              {syncResult.stats.conflicts > 0 && (
                <p className="text-amber-400">
                  冲突: {syncResult.stats.conflicts} 条 (本地版本已保留)
                </p>
              )}
              <p className="pt-1 text-muted-foreground/60">
                {syncResult.uploaded ? "已上传" : "未上传"} /{" "}
                {syncResult.downloaded ? "已下载" : "无远程数据"}
              </p>
            </div>
          </AcrylicPanel>
        )}

        {syncError && (
          <p className="text-center text-sm text-destructive">{syncError}</p>
        )}
      </div>
    </div>
  )
}
