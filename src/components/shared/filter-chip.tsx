import { useState, useRef, useEffect } from "react"
import { Check, ChevronDown } from "lucide-react"
import { cn } from "@/lib/utils"

interface FilterOption {
  value: string
  label: string
}

interface FilterChipProps {
  label: string
  options: FilterOption[]
  value: string
  onChange: (value: string) => void
  className?: string
}

/** 胶囊筛选按钮：显示当前选中值 + 下拉图标，点击弹出浮层选项列表。 */
export function FilterChip({ label, options, value, onChange, className }: FilterChipProps) {
  const [open, setOpen] = useState(false)
  const containerRef = useRef<HTMLDivElement>(null)

  // Close on click outside
  useEffect(() => {
    function handleClickOutside(e: MouseEvent) {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setOpen(false)
      }
    }
    if (open) {
      document.addEventListener("mousedown", handleClickOutside)
      return () => document.removeEventListener("mousedown", handleClickOutside)
    }
  }, [open])

  const selectedOption = options.find((o) => o.value === value)
  const displayLabel = selectedOption?.label ?? label

  return (
    <div ref={containerRef} className={cn("relative", className)}>
      <button
        type="button"
        onClick={() => setOpen(!open)}
        aria-expanded={open}
        className={cn(
          "inline-flex min-h-11 items-center gap-1.5 rounded-lg border px-2.5 py-1.5 text-xs font-medium transition-colors md:min-h-8",
          value
            ? "border-primary/30 bg-primary/10 text-primary"
            : "border-border/60 bg-card/70 text-muted-foreground hover:bg-card hover:text-foreground",
        )}
      >
        {value && <span className="h-1.5 w-1.5 rounded-full bg-primary" aria-hidden="true" />}
        {displayLabel}
        <ChevronDown className={cn("h-3 w-3 opacity-70 transition-transform", open && "rotate-180")} />
      </button>

      {open && (
        <div
          className={cn(
            "absolute left-0 top-full z-30 mt-2",
            "min-w-[140px] overflow-hidden rounded-xl border border-border/60 bg-popover p-1 shadow-xl backdrop-blur-xl",
            "animate-in fade-in-0 zoom-in-95 origin-top",
          )}
        >
          {options.map((opt) => {
            const isSelected = opt.value === value
            return (
              <button
                key={opt.value}
                type="button"
                onClick={() => {
                  onChange(opt.value)
                  setOpen(false)
                }}
                className={cn(
                  "flex w-full items-center justify-between gap-2 rounded-lg px-2.5 py-2 text-left text-[13px] transition-colors",
                  isSelected
                    ? "bg-primary/10 font-medium text-primary"
                    : "text-popover-foreground hover:bg-muted/60",
                )}
              >
                {opt.label}
                {isSelected && <Check className="h-3.5 w-3.5" />}
              </button>
            )
          })}
        </div>
      )}
    </div>
  )
}
