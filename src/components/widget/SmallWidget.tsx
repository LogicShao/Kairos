import { ArrowUpRight, CalendarClock } from "lucide-react"
import type { TodayBriefingResponse } from "@/types/briefing"
import type { MainNavigationTarget } from "@/types/widget"
import { getNextWidgetItem } from "@/components/widget/widget-format"

interface SmallWidgetProps {
  briefing: TodayBriefingResponse
  onOpen: (target: MainNavigationTarget) => void
}

export function SmallWidget({ briefing, onOpen }: SmallWidgetProps) {
  const item = getNextWidgetItem(briefing)

  return (
    <button
      type="button"
      onClick={() => onOpen(item.target)}
      className="flex h-full w-full flex-col items-start justify-between px-4 py-3 text-left transition-colors hover:bg-muted/30"
    >
      <span className="flex items-center gap-2 text-[11px] font-medium text-muted-foreground">
        <CalendarClock className="h-3.5 w-3.5" />
        {item.label}
      </span>
      <span className="min-w-0">
        <span className="block line-clamp-2 text-lg font-semibold leading-tight text-foreground">
          {item.title}
        </span>
        <span className="mt-1 block truncate text-xs text-muted-foreground">{item.detail}</span>
      </span>
      <span className="flex items-center gap-1 text-[11px] text-primary">
        打开
        <ArrowUpRight className="h-3 w-3" />
      </span>
    </button>
  )
}
