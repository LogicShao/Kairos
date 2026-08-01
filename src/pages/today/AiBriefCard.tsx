import { useCallback, useEffect, useState } from "react"
import { Channel, invoke } from "@tauri-apps/api/core"
import { RefreshCw, Sparkles, Wand2 } from "lucide-react"
import { AcrylicPanel } from "@/components/shared/acrylic-panel"
import { Button } from "@/components/ui/button"
import { cn } from "@/lib/utils"
import { userErrorMessage } from "@/lib/errors"
import { listenWithCleanup } from "@/lib/tauri-events"
import { MarkdownSubset } from "@/lib/markdown-subset"
import type { AiConfig, AiMorningBrief } from "@/types/ai"

/** 后端流式推送的增量片段（与 commands/ai.rs 的 StreamChunk 对齐）。 */
interface StreamChunk {
  delta: string
}

/** 剥离 markdown 末尾的 --- 分隔线与来源 footer（存储仍保留以通过后端结构校验）。 */
function stripBriefFooter(md: string): string {
  const footerRe = /(?:\n?---\s*\n?)?(?:\*?(?:本地生成|AI生成，请核实)\*?)\s*$/
  return md.replace(footerRe, "").trim()
}

interface AiBriefCardProps {
  onNavigate: (key: string) => void
}

/**
 * 今日页的 AI 每日摘要卡片。
 * 未启用或未配置 key 时整卡隐藏（AI 可选，不打扰默认用户）。
 */
export function AiBriefCard({ onNavigate }: AiBriefCardProps) {
  const [config, setConfig] = useState<AiConfig | null>(null)
  const [brief, setBrief] = useState<AiMorningBrief | null>(null)
  const [loading, setLoading] = useState(true)
  const [generating, setGenerating] = useState(false)
  const [streamingText, setStreamingText] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)

  // ── 初始加载：config + 今日缓存 ──
  useEffect(() => {
    let disposed = false

    async function load() {
      try {
        const cfg = await invoke<AiConfig>("get_ai_config")
        if (disposed) return
        setConfig(cfg)
        if (!cfg.enabled || !cfg.api_key_configured) {
          setLoading(false)
          return
        }
        const cached = await invoke<AiMorningBrief | null>("get_ai_morning_brief")
        if (disposed) return
        setBrief(cached)
      } catch (err) {
        if (disposed) return
        setError(userErrorMessage(err, "无法加载 AI 摘要"))
      } finally {
        if (!disposed) setLoading(false)
      }
    }

    void load()
    return () => {
      disposed = true
    }
  }, [])

  // ── 监听 7:00 自动生成事件：页面打开时自动刷新 ──
  useEffect(() => {
    return listenWithCleanup<AiMorningBrief>(
      "ai-brief-generated",
      (event) => {
        setBrief(event.payload)
        setError(null)
      },
      () => undefined,
    )
  }, [])

  // ── 手动生成 / 重新生成（流式）──
  const handleGenerate = useCallback(async (force: boolean) => {
    setGenerating(true)
    setError(null)
    setStreamingText("")
    const channel = new Channel<StreamChunk>()
    channel.onmessage = (chunk) => {
      setStreamingText((prev) => prev + chunk.delta)
    }
    try {
      const result = await invoke<AiMorningBrief>("generate_ai_morning_brief_streaming", {
        force,
        channel,
      })
      setBrief(result)
      setStreamingText(null)
    } catch (err) {
      setError(userErrorMessage(err, "生成摘要失败"))
    } finally {
      setGenerating(false)
    }
  }, [])

  if (loading) {
    return (
      <AcrylicPanel className="bg-card p-4">
        <div className="h-4 w-24 animate-pulse rounded bg-muted" />
        <div className="mt-3 h-20 animate-pulse rounded-lg bg-muted/60" />
      </AcrylicPanel>
    )
  }

  if (!config?.enabled || !config.api_key_configured) {
    // AI 可选：未启用时整卡隐藏。
    return null
  }

  return (
    <AcrylicPanel className="border-primary/15 bg-card p-4 sm:p-5">
      <div className="mb-3 flex items-center justify-between gap-2">
        <div className="flex items-center gap-2">
          <span className="flex h-6 w-6 items-center justify-center rounded-md bg-primary/10 text-primary">
            <Sparkles className="h-3.5 w-3.5" />
          </span>
          <h2 className="text-sm font-semibold text-foreground">AI 每日摘要</h2>
        </div>

        {brief && (
          <div className="flex items-center gap-2">
            <span
              className={cn(
                "rounded-full px-2 py-0.5 text-[10px] font-medium",
                brief.source === "ai"
                  ? "bg-primary/10 text-primary"
                  : "bg-muted text-muted-foreground",
              )}
            >
              {brief.source === "ai" ? "AI 生成" : "本地生成"}
            </span>
            <Button
              type="button"
              variant="ghost"
              size="icon-sm"
              aria-label="重新生成"
              onClick={() => void handleGenerate(true)}
              disabled={generating}
            >
              <RefreshCw className={cn("h-3.5 w-3.5", generating && "animate-spin")} />
            </Button>
          </div>
        )}
      </div>

      {brief || (generating && streamingText) ? (
        <MarkdownSubset text={stripBriefFooter(brief ? brief.markdown : (streamingText ?? ""))} />
      ) : (
        <div className="flex flex-col gap-3">
          <p className="text-sm text-muted-foreground">
            今天还没有生成摘要。点击生成，由本地数据生成一份晨间学习计划。
          </p>
          <Button
            type="button"
            onClick={() => void handleGenerate(false)}
            disabled={generating}
            className="w-full sm:w-auto"
          >
            {generating ? (
              <RefreshCw className="mr-1.5 h-4 w-4 animate-spin" />
            ) : (
              <Wand2 className="mr-1.5 h-4 w-4" />
            )}
            生成今日摘要
          </Button>
        </div>
      )}

      {error && (
        <p className="mt-3 text-xs text-destructive">
          {error} ·{" "}
          <button
            type="button"
            className="text-foreground underline underline-offset-2 hover:text-primary"
            onClick={() => onNavigate("ai-settings")}
          >
            前往 AI 设置
          </button>
        </p>
      )}

      {brief && (
        <p className="mt-3 flex items-center gap-1 text-[11px] text-muted-foreground/70">
          {brief.source === "ai" && brief.model ? `模型 ${brief.model} · ` : null}
          生成于 {new Date(brief.generated_at).toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit" })}
        </p>
      )}
    </AcrylicPanel>
  )
}
