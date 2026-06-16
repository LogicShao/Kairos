import { useState } from "react"
import { Timer, CheckSquare, BookOpen, Clock } from "lucide-react"
import {
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"
import { Button } from "@fluentui/react-components"
import {
  WeatherMoon24Regular,
  WeatherSunny24Regular,
  ArrowLeft24Regular,
  Cloud24Regular
} from "@fluentui/react-icons"
import { useTheme } from "@/hooks/use-theme"
import { AcrylicPanel } from "@/components/shared/acrylic-panel"
import { PomodoroTimer } from "@/components/pomodoro/PomodoroTimer"
import { TaskList } from "@/components/todo/TaskList"
import { CourseSchedule } from "@/components/courses/CourseSchedule"
import { ExamList } from "@/components/exams/ExamList"
import { SyncSettings } from "@/components/sync/SyncSettings"

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
          appearance="subtle"
          size="small"
          className="fixed top-4 left-4"
          onClick={() => setActiveFeature(null)}
          icon={<ArrowLeft24Regular />}
        >
          返回
        </Button>

        <Button
          appearance="subtle"
          shape="circular"
          className="fixed top-4 right-4"
          onClick={toggle}
          icon={theme === "dark" ? <WeatherSunny24Regular /> : <WeatherMoon24Regular />}
          aria-label="切换深浅色模式"
        />

        <AcrylicPanel className="p-8 w-full max-w-md">
          <PomodoroTimer />
        </AcrylicPanel>
      </div>
    )
  }

  if (activeFeature === "待办事项") {
    return (
      <div className="min-h-screen bg-background flex flex-col items-center p-8">
        <Button
          appearance="subtle"
          size="small"
          className="fixed top-4 left-4"
          onClick={() => setActiveFeature(null)}
          icon={<ArrowLeft24Regular />}
        >
          返回
        </Button>

        <Button
          appearance="subtle"
          shape="circular"
          className="fixed top-4 right-4"
          onClick={toggle}
          icon={theme === "dark" ? <WeatherSunny24Regular /> : <WeatherMoon24Regular />}
          aria-label="切换深浅色模式"
        />

        <div className="w-full pt-12">
          <TaskList />
        </div>
      </div>
    )
  }

  if (activeFeature === "课程表") {
    return (
      <div className="min-h-screen bg-background flex flex-col items-center p-4 sm:p-8">
        <Button
          appearance="subtle"
          size="small"
          className="fixed top-4 left-4 z-20"
          onClick={() => setActiveFeature(null)}
          icon={<ArrowLeft24Regular />}
        >
          返回
        </Button>

        <Button
          appearance="subtle"
          shape="circular"
          className="fixed top-4 right-4 z-20"
          onClick={toggle}
          icon={theme === "dark" ? <WeatherSunny24Regular /> : <WeatherMoon24Regular />}
          aria-label="切换深浅色模式"
        />

        <div className="w-full pt-12">
          <CourseSchedule />
        </div>
      </div>
    )
  }

  if (activeFeature === "考试倒计时") {
    return (
      <div className="min-h-screen bg-background flex flex-col items-center p-4 sm:p-8">
        <Button
          appearance="subtle"
          size="small"
          className="fixed top-4 left-4"
          onClick={() => setActiveFeature(null)}
          icon={<ArrowLeft24Regular />}
        >
          返回
        </Button>

        <Button
          appearance="subtle"
          shape="circular"
          className="fixed top-4 right-4"
          onClick={toggle}
          icon={theme === "dark" ? <WeatherSunny24Regular /> : <WeatherMoon24Regular />}
          aria-label="切换深浅色模式"
        />

        <div className="w-full pt-12">
          <ExamList />
        </div>
      </div>
    )
  }

  if (activeFeature === "同步") {
    return (
      <div className="min-h-screen bg-background flex flex-col items-center p-4 sm:p-8">
        <Button
          appearance="subtle"
          size="small"
          className="fixed top-4 left-4"
          onClick={() => setActiveFeature(null)}
          icon={<ArrowLeft24Regular />}
        >
          返回
        </Button>

        <Button
          appearance="subtle"
          shape="circular"
          className="fixed top-4 right-4"
          onClick={toggle}
          icon={theme === "dark" ? <WeatherSunny24Regular /> : <WeatherMoon24Regular />}
          aria-label="切换深浅色模式"
        />

        <div className="w-full pt-12">
          <SyncSettings />
        </div>
      </div>
    )
  }

  return (
    <div className="min-h-screen bg-background flex flex-col items-center justify-center p-8">
      <Button
        appearance="subtle"
        shape="circular"
        className="fixed top-4 left-4"
        onClick={() => setActiveFeature("同步")}
        icon={<Cloud24Regular />}
        aria-label="同步设置"
      />

      <Button
        appearance="subtle"
        shape="circular"
        className="fixed top-4 right-4"
        onClick={toggle}
        icon={theme === "dark" ? <WeatherSunny24Regular /> : <WeatherMoon24Regular />}
        aria-label="切换深浅色模式"
      />

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
              onClick={() => setActiveFeature(title)}
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
