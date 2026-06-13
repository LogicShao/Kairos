import type { ComponentPropsWithoutRef } from "react"
import { cn } from "@/lib/utils"

export function AcrylicPanel({ children, className, ...props }: ComponentPropsWithoutRef<"div">) {
  return (
    <div
      className={cn(
        "rounded-lg border border-border/60",
        "bg-card/85 backdrop-blur-xl backdrop-saturate-150",
        "shadow-[0_1px_3px_rgba(0,0,0,0.04)]",
        className
      )}
      {...props}
    >
      {children}
    </div>
  )
}
