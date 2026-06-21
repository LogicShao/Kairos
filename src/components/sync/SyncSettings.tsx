import { useState, useEffect, useCallback } from "react"
import { invoke } from "@tauri-apps/api/core"
import { RefreshCw, Check, X } from "lucide-react"
import type { SyncConfig, SyncResult } from "@/types/sync"
import { Button } from "@/components/ui/button"
import { AcrylicPanel } from "@/components/shared/acrylic-panel"
import { cn } from "@/lib/utils"

function inputClass(hasError: boolean): string {
  return cn(
    "w-full rounded-lg border bg-background px-3 py-2 text-sm outline-none transition-colors",
    "placeholder:text-muted-foreground/60",
    hasError
      ? "border-destructive/60 focus:border-destructive"
      : "border-border focus:border-primary",
  )
}

export function SyncSettings() {
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

  useEffect(() => {
    invoke<SyncConfig>("get_sync_config")
      .then((cfg) => {
        setConfig(cfg)
        setServerUrl(cfg.server_url)
        setUsername(cfg.username)
        setPassword(cfg.password)
        setAutoSync(cfg.auto_sync)
        setLastSyncAt(cfg.last_sync_at)
      })
      .catch(console.error)
  }, [])

  const saveConfig = useCallback(() => {
    if (!config) return
    const updated: SyncConfig = {
      ...config,
      server_url: serverUrl,
      username,
      password,
      auto_sync: autoSync,
    }
    invoke("update_sync_config", { config: updated })
      .then(() => setConfig(updated))
      .catch(console.error)
  }, [config, serverUrl, username, password, autoSync])

  const handleTestConnection = useCallback(() => {
    saveConfig()
    setTestStatus("loading")
    setTestMessage("")
    invoke<boolean>("test_sync_connection")
      .then((ok) => {
        setTestStatus(ok ? "ok" : "fail")
        setTestMessage(ok ? "连接成功" : "无法连接到服务器")
      })
      .catch((err: string) => {
        setTestStatus("fail")
        setTestMessage(typeof err === "string" ? err : "连接失败")
      })
  }, [saveConfig])

  const handleSyncNow = useCallback(() => {
    saveConfig()
    setSyncStatus("loading")
    setSyncResult(null)
    setSyncError(null)
    invoke<SyncResult>("sync_now")
      .then((result) => {
        setSyncResult(result)
        setSyncStatus("done")
        const now = new Date().toISOString()
        setLastSyncAt(now)
      })
      .catch((err: string) => {
        setSyncError(typeof err === "string" ? err : "同步失败")
        setSyncStatus("done")
      })
  }, [saveConfig])

  const showPassword = password.length > 0
  const passwordMasked = showPassword ? "●".repeat(Math.min(password.length, 12)) : ""

  return (
    <div className="flex flex-col gap-6 w-full max-w-md mx-auto animate-in fade-in-0 slide-in-from-bottom-2 duration-300">
      <AcrylicPanel className="p-4 sm:p-6">
        <h2 className="text-lg font-heading font-medium text-foreground mb-4">
          WebDAV 同步设置
        </h2>

        <div className="flex flex-col gap-4">
          <div>
            <label className="block text-sm text-muted-foreground mb-1.5">
              服务器地址
            </label>
            <input
              type="text"
              className={inputClass(false)}
              placeholder="webdav.example.com"
              value={serverUrl}
              onChange={(e) => setServerUrl(e.target.value)}
              onBlur={saveConfig}
            />
          </div>

          <div>
            <label className="block text-sm text-muted-foreground mb-1.5">
              用户名
            </label>
            <input
              type="text"
              className={inputClass(false)}
              placeholder="username"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              onBlur={saveConfig}
            />
          </div>

          <div>
            <label className="block text-sm text-muted-foreground mb-1.5">
              密码
            </label>
            <input
              type="password"
              className={inputClass(false)}
              placeholder={passwordMasked || "••••••••"}
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              onBlur={saveConfig}
            />
          </div>

          <div className="flex items-center justify-between pt-1">
            <span className="text-sm text-muted-foreground">自动同步</span>
            <button
              type="button"
              role="switch"
              aria-checked={autoSync}
              onClick={() => {
                setAutoSync(!autoSync)
                setTimeout(saveConfig, 0)
              }}
              className={cn(
                "relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors",
                "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2",
                autoSync ? "bg-primary" : "bg-muted",
              )}
            >
              <span
                className={cn(
                  "pointer-events-none block h-5 w-5 rounded-full bg-background shadow-sm ring-0 transition-transform",
                  autoSync ? "translate-x-5" : "translate-x-0",
                )}
              />
            </button>
          </div>
        </div>
      </AcrylicPanel>

      <div className="flex flex-col sm:flex-row gap-3">
        <Button
          variant="outline"
          onClick={handleTestConnection}
          disabled={testStatus === "loading" || !serverUrl}
          className="flex-1"
        >
          {testStatus === "loading" ? (
            <RefreshCw className="h-4 w-4 mr-1 animate-spin" />
          ) : testStatus === "ok" ? (
            <Check className="h-4 w-4 mr-1 text-emerald-400" />
          ) : testStatus === "fail" ? (
            <X className="h-4 w-4 mr-1 text-destructive" />
          ) : null}
          测试连接
        </Button>

        <Button
          onClick={handleSyncNow}
          disabled={syncStatus === "loading" || !serverUrl}
          className="flex-1"
        >
          {syncStatus === "loading" ? (
            <RefreshCw className="h-4 w-4 mr-1 animate-spin" />
          ) : null}
          立即同步
        </Button>
      </div>

      {testMessage && (
        <p
          className={cn(
            "text-sm text-center",
            testStatus === "ok" ? "text-emerald-400" : "text-destructive",
          )}
        >
          {testMessage}
        </p>
      )}

      {lastSyncAt && (
        <p className="text-xs text-center text-muted-foreground">
          上次同步: {new Date(lastSyncAt).toLocaleString()}
        </p>
      )}

      {syncResult && (
        <AcrylicPanel className="p-4">
          <h3 className="text-sm font-medium text-foreground mb-2">同步结果</h3>
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
              {syncResult.uploaded ? "✓ 已上传" : "✗ 未上传"} /{" "}
              {syncResult.downloaded ? "✓ 已下载" : "✗ 无远程数据"}
            </p>
          </div>
        </AcrylicPanel>
      )}

      {syncError && (
        <p className="text-sm text-center text-destructive">{syncError}</p>
      )}
    </div>
  )
}
