import type { ReactNode } from "react"
import { Timer, CheckSquare, BookOpen, Clock, Cloud, Moon, Sun } from "lucide-react"
import { cn } from "@/lib/utils"
import { useTheme } from "@/hooks/use-theme"

interface NavItem {
  key: string
  label: string
  icon: typeof Timer
}

const NAV_ITEMS: NavItem[] = [
  { key: "pomodoro", label: "番茄钟", icon: Timer },
  { key: "todo", label: "待办事项", icon: CheckSquare },
  { key: "courses", label: "课程表", icon: BookOpen },
  { key: "exams", label: "考试倒计时", icon: Clock },
]

interface AppShellProps {
  active: string
  onNavigate: (key: string) => void
  children: ReactNode
}

/** 应用外壳：常驻左侧导航栏 + 右侧毛玻璃内容区。统一承载主题切换与同步入口，消除各功能页重复头部。 */
export function AppShell({ active, onNavigate, children }: AppShellProps) {
  const { theme, toggle } = useTheme()

  return (
    <div className="relative z-0 flex h-screen overflow-hidden">
      <aside className="flex w-52 shrink-0 flex-col border-r border-border/50 bg-card/40 px-3 py-4 backdrop-blur-xl">
        <div className="flex items-center gap-2 px-2 pb-5">
          <img src="/favicon.svg" alt="Kairos" className="h-8 w-8 shrink-0 rounded-lg" />
          <div className="min-w-0">
            <div className="font-heading text-sm font-medium leading-none text-foreground">Kairos</div>
            <div className="mt-1 truncate text-[10px] text-muted-foreground">καιρός · 恰当时机</div>
          </div>
        </div>

        <nav className="flex flex-1 flex-col gap-1">
          {NAV_ITEMS.map(({ key, label, icon: Icon }) => {
            const isActive = active === key
            return (
              <button
                key={key}
                type="button"
                onClick={() => onNavigate(key)}
                aria-current={isActive ? "page" : undefined}
                className={cn(
                  "group flex items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm font-medium transition-all",
                  isActive
                    ? "bg-primary/[0.12] text-primary"
                    : "text-muted-foreground hover:bg-muted/60 hover:text-foreground",
                )}
              >
                <Icon className="h-4 w-4 shrink-0 transition-transform group-hover:scale-110" />
                {label}
              </button>
            )
          })}
        </nav>

        <div className="mt-2 flex items-center justify-between border-t border-border/50 pt-3">
          <button
            type="button"
            onClick={() => onNavigate("sync")}
            aria-current={active === "sync" ? "page" : undefined}
            className={cn(
              "flex items-center gap-2 rounded-lg px-2.5 py-2 text-sm font-medium transition-colors",
              active === "sync"
                ? "bg-primary/[0.12] text-primary"
                : "text-muted-foreground hover:bg-muted/60 hover:text-foreground",
            )}
          >
            <Cloud className="h-4 w-4" />
            同步
          </button>
          <button
            type="button"
            onClick={toggle}
            aria-label="切换深浅色模式"
            className="rounded-lg p-2 text-muted-foreground transition-colors hover:bg-muted/60 hover:text-foreground"
          >
            {theme === "dark" ? <Sun className="h-4 w-4" /> : <Moon className="h-4 w-4" />}
          </button>
        </div>
      </aside>

      <main className="flex-1 overflow-y-auto">
        <div className="min-h-full px-6 py-8">{children}</div>
      </main>
    </div>
  )
}
