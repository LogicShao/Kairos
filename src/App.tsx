import { useState } from "react"
import { Timer, CheckSquare, BookOpen, Clock, Sun, Moon, ArrowLeft } from "lucide-react"
import {
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { useTheme } from "@/hooks/use-theme"
import { AcrylicPanel } from "@/components/shared/acrylic-panel"
import { PomodoroTimer } from "@/components/pomodoro/PomodoroTimer"

const features = [
  {
    icon: Timer,
    title: "番茄钟",
    description: "专注计时，高效工作",
    color: "text-red-400",
    bgColor: "bg-red-500/10",
  },
  {
    icon: CheckSquare,
    title: "待办事项",
    description: "任务管理，井井有条",
    color: "text-blue-400",
    bgColor: "bg-blue-500/10",
  },
  {
    icon: BookOpen,
    title: "课程表",
    description: "周视图课程安排",
    color: "text-emerald-400",
    bgColor: "bg-emerald-500/10",
  },
  {
    icon: Clock,
    title: "考试倒计时",
    description: "重要日期提醒",
    color: "text-amber-400",
    bgColor: "bg-amber-500/10",
  },
]

function App() {
  const { theme, toggle } = useTheme()
  const [activeFeature, setActiveFeature] = useState<string | null>(null)

  if (activeFeature === "番茄钟") {
    return (
      <div className="min-h-screen bg-background flex flex-col items-center justify-center p-8">
        <Button
          variant="ghost"
          size="sm"
          className="fixed top-4 left-4"
          onClick={() => setActiveFeature(null)}
        >
          <ArrowLeft className="h-4 w-4 mr-1" />
          返回
        </Button>

        <Button
          variant="ghost"
          size="icon"
          className="fixed top-4 right-4 rounded-full"
          onClick={toggle}
          aria-label="切换深浅色模式"
        >
          {theme === "dark" ? (
            <Sun className="h-5 w-5" />
          ) : (
            <Moon className="h-5 w-5" />
          )}
        </Button>

        <AcrylicPanel className="p-8 w-full max-w-md">
          <PomodoroTimer />
        </AcrylicPanel>
      </div>
    )
  }

  return (
    <div className="min-h-screen bg-background flex flex-col items-center justify-center p-8">
      <Button
        variant="ghost"
        size="icon"
        className="fixed top-4 right-4 rounded-full"
        onClick={toggle}
        aria-label="切换深浅色模式"
      >
        {theme === "dark" ? (
          <Sun className="h-5 w-5" />
        ) : (
          <Moon className="h-5 w-5" />
        )}
      </Button>

      <div className="text-center mb-8">
        <h1 className="text-3xl font-heading font-medium text-primary tracking-tight">
          Kairos
        </h1>
        <p className="mt-1.5 text-base text-foreground/60">
          καιρός — 恰当时机
        </p>
      </div>

      <AcrylicPanel className="p-6 w-full max-w-2xl bg-card/95">
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
          {features.map(({ icon: Icon, title, description, color, bgColor }) => (
            <AcrylicPanel
              key={title}
              className="transition-colors hover:bg-card/95 cursor-pointer bg-card p-4"
              onClick={title === "番茄钟" ? () => setActiveFeature(title) : undefined}
            >
              <CardHeader>
                <div
                  className={`w-10 h-10 rounded-lg ${bgColor} flex items-center justify-center mb-2`}
                >
                  <Icon className={`w-5 h-5 ${color}`} />
                </div>
                <CardTitle>{title}</CardTitle>
                <CardDescription>{description}</CardDescription>
              </CardHeader>
            </AcrylicPanel>
          ))}
        </div>
      </AcrylicPanel>

      <p className="mt-12 text-sm text-muted-foreground">
        Kairos v0.1.0
      </p>
    </div>
  )
}

export default App
