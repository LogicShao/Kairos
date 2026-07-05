import { useCallback, useEffect, useState } from "react"
import { invoke } from "@tauri-apps/api/core"
import { ArrowLeft, Lock, MonitorCog, Pin, RefreshCw, Unlock } from "lucide-react"
import type { UpdateWidgetConfigRequest, WidgetConfig, WidgetMode } from "@/types/widget"
import { AcrylicPanel } from "@/components/shared/acrylic-panel"
import { Button } from "@/components/ui/button"
import { cn } from "@/lib/utils"

interface WidgetSettingsProps {
  onNavigate: (key: string) => void
}

interface SwitchRowProps {
  title: string
  detail: string
  checked: boolean
  disabled: boolean
  onToggle: () => void
}

const MODE_OPTIONS: Array<{ value: WidgetMode; label: string; detail: string }> = [
  { value: "small", label: "小", detail: "下一件事" },
  { value: "medium", label: "中", detail: "专注状态" },
  { value: "large", label: "大", detail: "今日概览" },
]

function fetchWidgetConfig(): Promise<WidgetConfig> {
  return invoke<WidgetConfig>("get_widget_config")
}

function SwitchRow({ title, detail, checked, disabled, onToggle }: SwitchRowProps) {
  return (
    <div className="flex items-center justify-between gap-3 rounded-lg border border-border/60 bg-muted/20 px-3 py-3">
      <div className="min-w-0">
        <div className="text-sm font-medium text-foreground">{title}</div>
        <div className="mt-0.5 truncate text-xs text-muted-foreground">{detail}</div>
      </div>
      <button
        type="button"
        role="switch"
        aria-label={title}
        aria-checked={checked}
        onClick={onToggle}
        disabled={disabled}
        className={cn(
          "relative inline-flex h-6 w-11 shrink-0 rounded-full border-2 border-transparent transition-colors",
          "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:opacity-50",
          checked ? "bg-primary" : "bg-muted-foreground/25",
        )}
      >
        <span
          className={cn(
            "pointer-events-none block h-5 w-5 rounded-full bg-card shadow-sm ring-0 transition-transform",
            checked ? "translate-x-5" : "translate-x-0",
          )}
        />
      </button>
    </div>
  )
}

export function WidgetSettings({ onNavigate }: WidgetSettingsProps) {
  const [config, setConfig] = useState<WidgetConfig | null>(null)
  const [opacityDraft, setOpacityDraft] = useState(0.92)
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    let cancelled = false

    fetchWidgetConfig()
      .then((nextConfig) => {
        if (cancelled) return
        setConfig(nextConfig)
        setOpacityDraft(nextConfig.opacity)
      })
      .catch((err) => {
        if (cancelled) return
        setError(typeof err === "string" ? err : "无法加载桌面小组件配置")
      })
      .finally(() => {
        if (cancelled) return
        setLoading(false)
      })

    return () => {
      cancelled = true
    }
  }, [])

  const saveConfig = useCallback(async (req: UpdateWidgetConfigRequest) => {
    setSaving(true)
    setError(null)
    try {
      const nextConfig = await invoke<WidgetConfig>("update_widget_config", { req })
      setConfig(nextConfig)
      setOpacityDraft(nextConfig.opacity)
    } catch (err) {
      setError(typeof err === "string" ? err : "保存失败")
    } finally {
      setSaving(false)
    }
  }, [])

  const commitOpacity = useCallback(() => {
    if (!config || opacityDraft === config.opacity) return
    void saveConfig({ opacity: opacityDraft })
  }, [config, opacityDraft, saveConfig])

  if (loading) {
    return (
      <div className="flex h-64 items-center justify-center">
        <RefreshCw className="h-6 w-6 animate-spin text-muted-foreground" />
      </div>
    )
  }

  if (!config) {
    return (
      <div className="min-h-0 flex-1 overflow-y-auto pb-4">
        <div className="mx-auto flex w-full max-w-md flex-col gap-4">
          <Button size="icon-sm" variant="ghost" onClick={() => onNavigate("kairos")} aria-label="返回 Kairos">
            <ArrowLeft className="h-4 w-4" />
          </Button>
          <p className="text-center text-sm text-destructive">{error ?? "配置不可用"}</p>
        </div>
      </div>
    )
  }

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
            <h2 className="text-lg font-semibold text-foreground">桌面小组件</h2>
            <p className="mt-1 text-sm text-muted-foreground">管理悬浮窗口、尺寸和显示偏好。</p>
          </div>
        </div>

        <AcrylicPanel className="border-primary/15 p-4 sm:p-6">
          <div className="mb-4 flex items-center gap-3">
            <span className="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
              <MonitorCog className="h-5 w-5" />
            </span>
            <div className="min-w-0">
              <h3 className="text-sm font-semibold text-foreground">窗口状态</h3>
              <p className="truncate text-xs text-muted-foreground">
                {config.enabled ? "小组件已启用" : "小组件未启用"}
              </p>
            </div>
          </div>

          <SwitchRow
            title="启用小组件"
            detail="开启后创建独立悬浮窗口"
            checked={config.enabled}
            disabled={saving}
            onToggle={() => void saveConfig({ enabled: !config.enabled })}
          />
        </AcrylicPanel>

        <AcrylicPanel className="border-primary/15 p-4 sm:p-6">
          <h3 className="mb-3 text-sm font-semibold text-foreground">尺寸</h3>
          <div className="grid grid-cols-3 gap-2">
            {MODE_OPTIONS.map((option) => {
              const active = option.value === config.mode
              return (
                <button
                  key={option.value}
                  type="button"
                  onClick={() => void saveConfig({ mode: option.value })}
                  disabled={saving}
                  className={cn(
                    "min-h-16 rounded-lg border px-2 py-2 text-center transition-colors disabled:opacity-50",
                    active
                      ? "border-primary bg-primary/10 text-primary"
                      : "border-border bg-muted/20 text-muted-foreground hover:bg-muted/40 hover:text-foreground",
                  )}
                >
                  <span className="block text-sm font-semibold">{option.label}</span>
                  <span className="mt-1 block truncate text-[11px]">{option.detail}</span>
                </button>
              )
            })}
          </div>
        </AcrylicPanel>

        <AcrylicPanel className="flex flex-col gap-3 border-primary/15 p-4 sm:p-6">
          <SwitchRow
            title="窗口置顶"
            detail="保持在其他窗口上方"
            checked={config.always_on_top}
            disabled={saving}
            onToggle={() => void saveConfig({ always_on_top: !config.always_on_top })}
          />
          <SwitchRow
            title="锁定位置"
            detail={config.locked ? "当前位置不会被拖动保存" : "可拖动并记住位置"}
            checked={config.locked}
            disabled={saving}
            onToggle={() => void saveConfig({ locked: !config.locked })}
          />

          <div className="rounded-lg border border-border/60 bg-muted/20 px-3 py-3">
            <div className="mb-2 flex items-center justify-between gap-3">
              <span className="flex items-center gap-2 text-sm font-medium text-foreground">
                <Pin className="h-4 w-4 text-muted-foreground" />
                透明度
              </span>
              <span className="text-xs text-muted-foreground">{Math.round(opacityDraft * 100)}%</span>
            </div>
            <input
              type="range"
              min={0.6}
              max={1}
              step={0.02}
              value={opacityDraft}
              onChange={(event) => setOpacityDraft(Number(event.target.value))}
              onMouseUp={commitOpacity}
              onTouchEnd={commitOpacity}
              onBlur={commitOpacity}
              disabled={saving}
              className="w-full accent-primary"
            />
          </div>
        </AcrylicPanel>

        <div className="flex items-center justify-center gap-2 text-xs text-muted-foreground">
          {config.locked ? <Lock className="h-3.5 w-3.5" /> : <Unlock className="h-3.5 w-3.5" />}
          {config.width} × {config.height}
        </div>

        {error && <p className="text-center text-sm text-destructive">{error}</p>}
      </div>
    </div>
  )
}
