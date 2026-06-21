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

function NavButton({
  active,
  icon: Icon,
  label,
  onClick,
}: {
  active: boolean
  icon: typeof Timer
  label: string
  onClick: () => void
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-current={active ? "page" : undefined}
      className={cn(
        "flex flex-col items-center justify-center gap-0.5 px-1 py-1 text-[10px] font-medium transition-colors rounded-lg",
        "md:flex-row md:gap-2.5 md:px-2.5 md:py-2 md:text-sm md:rounded-lg",
        active
          ? "text-primary md:bg-primary/[0.12]"
          : "text-muted-foreground hover:text-foreground md:hover:bg-muted/60",
      )}
    >
      <Icon className="h-5 w-5 shrink-0 md:h-4 md:w-4" />
      {label}
    </button>
  )
}

/** 应用外壳：桌面侧边栏 + 移动端底部 Tab Bar。统一承载主题切换与同步入口，消除各功能页重复头部。 */
export function AppShell({ active, onNavigate, children }: AppShellProps) {
  const { theme, toggle } = useTheme()

  return (
    <div className="relative z-0 flex flex-col h-screen overflow-hidden">
      {/* 移动端顶部栏：仅在 <md 时显示 */}
      <header className="flex md:hidden shrink-0 items-center gap-2 h-12 px-3 border-b border-border/50 bg-card/40 backdrop-blur-xl">
        <img src="/favicon.svg" alt="Kairos" className="h-7 w-7 shrink-0 rounded-lg" />
        <div className="min-w-0 flex-1">
          <div className="font-heading text-sm font-medium leading-none text-foreground">Kairos</div>
        </div>
        <button
          type="button"
          onClick={() => onNavigate("sync")}
          aria-label="同步设置"
          className="rounded-lg p-1.5 text-muted-foreground transition-colors hover:bg-muted/60 hover:text-foreground"
        >
          <Cloud className="h-4 w-4" />
        </button>
        <button
          type="button"
          onClick={toggle}
          aria-label="切换深浅色模式"
          className="rounded-lg p-1.5 text-muted-foreground transition-colors hover:bg-muted/60 hover:text-foreground"
        >
          {theme === "dark" ? <Sun className="h-4 w-4" /> : <Moon className="h-4 w-4" />}
        </button>
      </header>

      {/* 主体：桌面侧边栏 + 内容区 */}
      <div className="flex flex-1 overflow-hidden">
        {/* 桌面侧边栏：≥md 时显示 */}
        <aside className="hidden md:flex w-52 shrink-0 flex-col border-r border-border/50 bg-card/40 px-3 py-4 backdrop-blur-xl">
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

        {/* 内容区：底部在移动端留出 Tab Bar 空间 */}
        <main className="flex-1 overflow-y-auto pb-16 md:pb-0">
          <div className="min-h-full px-4 py-6 md:px-6 md:py-8">{children}</div>
        </main>
      </div>

      {/* 移动端底部 Tab Bar：仅在 <md 时显示 */}
      <nav className="flex md:hidden shrink-0 items-center justify-around h-16 border-t border-border/50 bg-card/40 backdrop-blur-xl">
        {NAV_ITEMS.map(({ key, label, icon: Icon }) => (
          <NavButton
            key={key}
            active={active === key}
            icon={Icon}
            label={label}
            onClick={() => onNavigate(key)}
          />
        ))}
      </nav>
    </div>
  )
}
