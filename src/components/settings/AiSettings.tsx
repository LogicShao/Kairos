import { useCallback, useEffect, useRef, useState } from "react"
import { invoke } from "@tauri-apps/api/core"
import { ArrowLeft, ShieldCheck } from "lucide-react"
import { Button } from "@/components/ui/button"
import { AcrylicPanel } from "@/components/shared/acrylic-panel"
import { cn } from "@/lib/utils"
import { userErrorMessage } from "@/lib/errors"
import type { AiConfig, UpdateAiConfigRequest } from "@/types/ai"

interface AiSettingsProps {
  onNavigate: (key: string) => void
}

interface AiDraft {
  base_url: string
  model: string
}

function draftFromConfig(config: AiConfig): AiDraft {
  return { base_url: config.base_url, model: config.model }
}

function draftsEqual(a: AiDraft, b: AiDraft): boolean {
  return a.base_url === b.base_url && a.model === b.model
}

function inputClass(): string {
  return cn(
    "w-full rounded-lg border bg-muted/40 px-3 py-2 text-sm outline-none transition-colors",
    "placeholder:text-muted-foreground/45",
    "focus:ring-2 focus:ring-primary/30 border-border focus:border-primary",
  )
}

/**
 * AI 设置页：启用开关 + base_url/model/api_key。
 * 仿 SyncSettings 的 save-on-leave 范式：文本字段在离开页面时自动保存，开关即时保存。
 */
export function AiSettings({ onNavigate }: AiSettingsProps) {
  const [config, setConfig] = useState<AiConfig | null>(null)
  const [baseUrl, setBaseUrl] = useState("")
  const [model, setModel] = useState("")
  const [apiKey, setApiKey] = useState("")
  const [apiKeyDirty, setApiKeyDirty] = useState(false)
  const [enabled, setEnabled] = useState(false)
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const configRef = useRef<AiConfig | null>(null)
  const latestDraftRef = useRef<AiDraft | null>(null)
  const lastPersistedRef = useRef<AiDraft | null>(null)
  const apiKeyDraftRef = useRef("")
  const apiKeyDirtyRef = useRef(false)

  const applyConfig = useCallback((cfg: AiConfig, updateEditableFields: boolean) => {
    setConfig(cfg)
    configRef.current = cfg
    const persisted = draftFromConfig(cfg)
    lastPersistedRef.current = persisted
    if (updateEditableFields) {
      setBaseUrl(cfg.base_url)
      setModel(cfg.model)
      setEnabled(cfg.enabled)
      setApiKey("")
      setApiKeyDirty(false)
      apiKeyDraftRef.current = ""
      apiKeyDirtyRef.current = false
      latestDraftRef.current = persisted
    }
  }, [])

  useEffect(() => {
    let disposed = false
    invoke<AiConfig>("get_ai_config")
      .then((cfg) => {
        if (disposed) return
        applyConfig(cfg, true)
      })
      .catch((err) => {
        if (disposed) return
        setError(userErrorMessage(err, "无法加载 AI 配置"))
      })
    return () => {
      disposed = true
    }
  }, [applyConfig])

  // 跟踪最新草稿（含 key 字段），供离开时持久化。
  useEffect(() => {
    if (!config) return
    latestDraftRef.current = { base_url: baseUrl, model }
    apiKeyDraftRef.current = apiKey
    apiKeyDirtyRef.current = apiKeyDirty
  }, [config, baseUrl, model, apiKey, apiKeyDirty])

  // 保存当前配置，返回保存后的完整配置。
  const saveConfig = useCallback(
    async (overrides?: Partial<AiDraft> & { enabled?: boolean }): Promise<AiConfig> => {
      const request: UpdateAiConfigRequest = {
        base_url: overrides?.base_url ?? baseUrl,
        model: overrides?.model ?? model,
        enabled: overrides?.enabled ?? enabled,
      }
      if (apiKeyDirty) {
        request.api_key = apiKey
      }
      await invoke("update_ai_config", { req: request })
      const saved = await invoke<AiConfig>("get_ai_config")
      applyConfig(saved, true)
      return saved
    },
    [apiKey, apiKeyDirty, applyConfig, baseUrl, enabled, model],
  )

  // 离开页面时持久化未保存的文本字段。
  useEffect(() => {
    return () => {
      const draft = latestDraftRef.current
      const persisted = lastPersistedRef.current
      if (!draft || !persisted) return
      const hasUnsavedChanges = !draftsEqual(draft, persisted) || apiKeyDirtyRef.current
      if (!hasUnsavedChanges) return

      const request: UpdateAiConfigRequest = { ...draft }
      if (apiKeyDirtyRef.current) {
        request.api_key = apiKeyDraftRef.current
      }
      void invoke("update_ai_config", { req: request }).catch(() => undefined)
    }
  }, [])

  // 启用开关：即时保存。
  const handleEnabledToggle = useCallback(async () => {
    const next = !enabled
    setEnabled(next)
    setError(null)
    try {
      await saveConfig({ enabled: next })
    } catch (err) {
      setEnabled(!next)
      setError(userErrorMessage(err, "保存设置失败"))
    }
  }, [enabled, saveConfig])

  // 显式保存按钮：给用户确定感（save-on-leave 之外的主路径）。
  const handleSave = useCallback(async () => {
    setSaving(true)
    setError(null)
    try {
      await saveConfig()
    } catch (err) {
      setError(userErrorMessage(err, "保存失败"))
    } finally {
      setSaving(false)
    }
  }, [saveConfig])

  const apiKeyPlaceholder =
    config?.api_key_configured && !apiKeyDirty ? "已保存密钥，留空不变" : "••••••••"

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
            <h2 className="text-lg font-semibold text-foreground">AI 设置</h2>
            <p className="mt-1 text-sm text-muted-foreground">
              配置 AI 每日摘要，让每天早起的计划更清晰。
            </p>
          </div>
        </div>

        <AcrylicPanel className="border-primary/15 p-4 sm:p-6">
          <div className="flex items-center justify-between pb-4">
            <span className="text-sm text-foreground">启用 AI 每日摘要</span>
            <button
              type="button"
              role="switch"
              aria-checked={enabled}
              onClick={() => void handleEnabledToggle()}
              className={cn(
                "relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors",
                "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2",
                enabled ? "bg-primary" : "bg-muted-foreground/25",
              )}
            >
              <span
                className={cn(
                  "pointer-events-none block h-5 w-5 rounded-full bg-card shadow-sm ring-0 transition-transform",
                  enabled ? "translate-x-5" : "translate-x-0",
                )}
              />
            </button>
          </div>

          <div className="flex flex-col gap-4 border-t border-border/50 pt-4">
            <div>
              <label className="mb-0.5 block text-sm text-muted-foreground">API Base URL</label>
              <input
                type="text"
                className={inputClass()}
                placeholder="https://api.deepseek.com"
                value={baseUrl}
                onChange={(e) => setBaseUrl(e.target.value)}
              />
              <p className="mt-1 text-[11px] text-muted-foreground/70">
                不含 /v1，后端统一追加 /v1/chat/completions。
              </p>
            </div>

            <div>
              <label className="mb-0.5 block text-sm text-muted-foreground">模型</label>
              <input
                type="text"
                className={inputClass()}
                placeholder="deepseek-chat"
                value={model}
                onChange={(e) => setModel(e.target.value)}
              />
            </div>

            <div>
              <label className="mb-0.5 block text-sm text-muted-foreground">API Key</label>
              <input
                type="password"
                className={inputClass()}
                placeholder={apiKeyPlaceholder}
                value={apiKey}
                onChange={(e) => {
                  setApiKey(e.target.value)
                  setApiKeyDirty(true)
                }}
              />
              <p className="mt-1 text-[11px] text-muted-foreground/70">
                密钥仅本机 AES 加密存储，不同步 WebDAV；留空表示保留已保存密钥。
              </p>
            </div>

            <div className="flex flex-col gap-2 pt-1">
              <Button onClick={() => void handleSave()} disabled={saving} className="min-h-11 md:min-h-0">
                {saving ? "保存中…" : "保存设置"}
              </Button>
            </div>
          </div>
        </AcrylicPanel>

        <AcrylicPanel className="flex gap-3 p-4">
          <ShieldCheck className="mt-0.5 h-4 w-4 shrink-0 text-emerald-500" />
          <div className="space-y-1 text-xs text-muted-foreground">
            <p>
              启用后，今日的课表、待办、考试与番茄钟数据会发送给所配置的第三方 AI 服务用于生成摘要。
            </p>
            <p>单次生成约 &lt;1000 token，每日最多一次，成本极低。</p>
            <p>可在本页随时关闭；关闭后 Today 页不再显示 AI 摘要。</p>
          </div>
        </AcrylicPanel>

        {error && <p className="text-center text-sm text-destructive">{error}</p>}
      </div>
    </div>
  )
}
