import { useState, useEffect } from "react"
import { invoke } from "@tauri-apps/api/core"
import type { LzuAuthStatus, LzuCourseImportResult } from "@/types/lzu"
import { Button } from "@/components/ui/button"
import { LogIn, LogOut, Download, User, Loader2, AlertCircle } from "lucide-react"
import { FIELD_CLASS } from "../utils"

interface LzuImportPanelProps {
  /** 导入成功后刷新课程列表的回调。 */
  onImportSuccess?: (result: LzuCourseImportResult) => Promise<void> | void
}

export function LzuImportPanel({ onImportSuccess }: LzuImportPanelProps) {
  const [authStatus, setAuthStatus] = useState<LzuAuthStatus | null>(null)
  const [authLoading, setAuthLoading] = useState(true)
  const [username, setUsername] = useState("")
  const [password, setPassword] = useState("")
  const [loginLoading, setLoginLoading] = useState(false)
  const [loginError, setLoginError] = useState<string | null>(null)
  const [importLoading, setImportLoading] = useState(false)
  const [importResult, setImportResult] = useState<LzuCourseImportResult | null>(null)
  const [importError, setImportError] = useState<string | null>(null)

  /** 挂载或切换到面板时检查登录状态。 */
  useEffect(() => {
    let cancelled = false
    async function load() {
      try {
        const status = await invoke<LzuAuthStatus>("lzu_get_auth_status")
        if (!cancelled) setAuthStatus(status)
      } catch {
        if (!cancelled) setAuthStatus({ is_logged_in: false, username: null })
      } finally {
        if (!cancelled) setAuthLoading(false)
      }
    }
    void load()
    return () => {
      cancelled = true
    }
  }, [])

  async function handleLogin() {
    if (!username.trim() || !password.trim()) {
      setLoginError("请输入账号和密码")
      return
    }
    setLoginLoading(true)
    setLoginError(null)
    try {
      const status = await invoke<LzuAuthStatus>("lzu_login", {
        username: username.trim(),
        password,
      })
      setAuthStatus(status)
      setUsername("")
      setPassword("")
    } catch (e) {
      setLoginError(typeof e === "string" ? e : "登录失败，请检查账号和密码。")
    } finally {
      setLoginLoading(false)
    }
  }

  async function handleLogout() {
    setLoginLoading(true)
    try {
      const status = await invoke<LzuAuthStatus>("lzu_logout")
      setAuthStatus(status)
      setImportResult(null)
      setImportError(null)
    } catch {
      setAuthStatus({ is_logged_in: false, username: null })
    } finally {
      setLoginLoading(false)
    }
  }

  async function handleImport() {
    setImportLoading(true)
    setImportError(null)
    setImportResult(null)
    try {
      const result = await invoke<LzuCourseImportResult>("import_lzu_courses")
      setImportResult(result)
      try {
        await onImportSuccess?.(result)
      } catch {
        setImportError("导入已完成，但刷新课程列表失败。")
      }
    } catch (e) {
      setImportError(typeof e === "string" ? e : "导入失败，请稍后重试。")
    } finally {
      setImportLoading(false)
    }
  }

  // ── Loading state (initial auth check) ──
  if (authLoading) {
    return (
      <div className="flex items-center justify-center py-8">
        <Loader2 className="h-5 w-5 animate-spin text-muted-foreground" />
      </div>
    )
  }

  // ── Not logged in: login form ──
  if (!authStatus?.is_logged_in) {
    return (
      <div className="space-y-4">
        <div className="space-y-3">
          <div>
            <label className="mb-1 block text-xs font-medium text-muted-foreground">账号</label>
            <input
              type="text"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              placeholder="学号/工号"
              className={FIELD_CLASS}
              autoComplete="username"
              disabled={loginLoading}
            />
          </div>
          <div>
            <label className="mb-1 block text-xs font-medium text-muted-foreground">密码</label>
            <input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder="统一认证密码"
              className={FIELD_CLASS}
              autoComplete="current-password"
              disabled={loginLoading}
              onKeyDown={(e) => {
                if (e.key === "Enter") void handleLogin()
              }}
            />
          </div>
        </div>

        {loginError && (
          <p className="flex items-start gap-1.5 text-sm text-destructive">
            <AlertCircle className="mt-0.5 h-3.5 w-3.5 shrink-0" />
            <span>{loginError}</span>
          </p>
        )}

        <Button
          type="button"
          className="w-full"
          size="sm"
          disabled={loginLoading || !username.trim() || !password.trim()}
          onClick={() => void handleLogin()}
        >
          {loginLoading ? (
            <>
              <Loader2 className="mr-1.5 h-3.5 w-3.5 animate-spin" />
              登录中...
            </>
          ) : (
            <>
              <LogIn className="mr-1.5 h-3.5 w-3.5" />
              登录
            </>
          )}
        </Button>
      </div>
    )
  }

  // ── Logged in: account summary + import ──
  return (
    <div className="space-y-4">
      {/* Account summary */}
      <div className="flex items-center justify-between rounded-md border border-border/60 bg-background/60 p-3">
        <div className="flex items-center gap-2.5 min-w-0">
          <User className="h-4 w-4 shrink-0 text-muted-foreground" />
          <span className="truncate text-sm font-medium text-foreground">
            {authStatus.username ?? "已登录"}
          </span>
        </div>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          disabled={loginLoading}
          onClick={() => void handleLogout()}
        >
          {loginLoading ? (
            <Loader2 className="h-3.5 w-3.5 animate-spin" />
          ) : (
            <>
              <LogOut className="mr-1 h-3.5 w-3.5" />
              退出
            </>
          )}
        </Button>
      </div>

      {/* Import button */}
      <Button
        type="button"
        className="w-full"
        size="sm"
        disabled={importLoading}
        onClick={() => void handleImport()}
      >
        {importLoading ? (
          <>
            <Loader2 className="mr-1.5 h-3.5 w-3.5 animate-spin" />
            导入中...
          </>
        ) : (
          <>
            <Download className="mr-1.5 h-3.5 w-3.5" />
            导入课表
          </>
        )}
      </Button>

      {/* Import error */}
      {importError && (
        <p className="flex items-start gap-1.5 text-sm text-destructive">
          <AlertCircle className="mt-0.5 h-3.5 w-3.5 shrink-0" />
          <span>{importError}</span>
        </p>
      )}

      {/* Import result */}
      {importResult && (
        <div className="rounded-md border border-border/60 bg-background/60 p-3 text-sm space-y-1">
          <div className="flex items-center gap-4 text-muted-foreground flex-wrap">
            <span>
              拉取 <strong className="text-foreground">{importResult.parsed}</strong> 条
            </span>
            <span>
              导入 <strong className="text-foreground">{importResult.imported}</strong> 条
            </span>
            {importResult.skipped > 0 && (
              <span className="text-muted-foreground/70">
                跳过 <strong>{importResult.skipped}</strong> 条
              </span>
            )}
            {importResult.failed > 0 && (
              <span className="text-muted-foreground/70">
                失败 <strong>{importResult.failed}</strong> 条
              </span>
            )}
          </div>
          {importResult.message && (
            <p className="text-xs text-muted-foreground/70">{importResult.message}</p>
          )}
        </div>
      )}
    </div>
  )
}
