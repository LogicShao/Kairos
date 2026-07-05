import type { ReactNode } from "react"
import { GripHorizontal, Lock, RefreshCw, X } from "lucide-react"
import type { WidgetConfig } from "@/types/widget"
import { cn } from "@/lib/utils"

interface WidgetFrameProps {
  config: WidgetConfig | null
  loading: boolean
  error: string | null
  onRetry: () => void
  onClose: () => void
  onDragStart: () => void
  children: ReactNode
}

export function WidgetFrame({
  config,
  loading,
  error,
  onRetry,
  onClose,
  onDragStart,
  children,
}: WidgetFrameProps) {
  const locked = config?.locked ?? false
  const opacity = config?.opacity ?? 0.92

  return (
    <div className="h-screen w-screen overflow-hidden bg-transparent p-1.5 text-foreground">
      <section
        className={cn(
          "glass-edge flex h-full w-full flex-col overflow-hidden rounded-lg border border-border/60",
          "bg-card/80 backdrop-blur-xl backdrop-saturate-150",
        )}
        style={{ opacity }}
      >
        <header className="flex h-8 shrink-0 items-center gap-1 border-b border-border/50 px-2">
          <button
            type="button"
            onPointerDown={locked ? undefined : onDragStart}
            aria-label={locked ? "小组件位置已锁定" : "拖动小组件"}
            className={cn(
              "flex h-7 min-w-7 flex-1 items-center justify-center rounded-md text-muted-foreground",
              locked ? "cursor-default" : "cursor-grab hover:bg-muted/50 hover:text-foreground",
            )}
          >
            {locked ? <Lock className="h-3.5 w-3.5" /> : <GripHorizontal className="h-4 w-4" />}
          </button>

          {error && (
            <button
              type="button"
              onClick={onRetry}
              aria-label="重新加载小组件"
              className="flex h-7 w-7 items-center justify-center rounded-md text-muted-foreground hover:bg-muted/50 hover:text-foreground"
            >
              <RefreshCw className="h-3.5 w-3.5" />
            </button>
          )}

          <button
            type="button"
            onClick={onClose}
            aria-label="关闭小组件"
            className="flex h-7 w-7 items-center justify-center rounded-md text-muted-foreground hover:bg-destructive/10 hover:text-destructive"
          >
            <X className="h-3.5 w-3.5" />
          </button>
        </header>

        <div className="min-h-0 flex-1">
          {loading ? (
            <div className="flex h-full items-center justify-center">
              <RefreshCw className="h-5 w-5 animate-spin text-muted-foreground" />
            </div>
          ) : error ? (
            <div className="flex h-full flex-col items-center justify-center gap-2 px-4 text-center">
              <p className="line-clamp-2 text-xs text-destructive">{error}</p>
              <button
                type="button"
                onClick={onRetry}
                className="rounded-md border border-border/70 px-2.5 py-1 text-xs text-muted-foreground hover:bg-muted/50 hover:text-foreground"
              >
                重试
              </button>
            </div>
          ) : (
            children
          )}
        </div>
      </section>
    </div>
  )
}
