import { cn } from "@/lib/utils"

interface AcrylicPanelProps {
  children: React.ReactNode
  className?: string
}

export function AcrylicPanel({ children, className }: AcrylicPanelProps) {
  return (
    <div
      className={cn(
        "rounded-lg border border-border/60",
        "bg-card/85 backdrop-blur-xl backdrop-saturate-150",
        "shadow-[0_1px_3px_rgba(0,0,0,0.04)]",
        className
      )}
    >
      {children}
    </div>
  )
}
