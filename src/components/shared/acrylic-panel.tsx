import { cn } from "@/lib/utils"

interface AcrylicPanelProps {
  children: React.ReactNode
  className?: string
}

export function AcrylicPanel({ children, className }: AcrylicPanelProps) {
  return (
    <div
      className={cn(
        "rounded-lg border border-border/40",
        "bg-card/40 backdrop-blur-xl backdrop-saturate-150",
        "shadow-[0_4px_30px_rgba(0,0,0,0.3)]",
        className
      )}
    >
      {children}
    </div>
  )
}
