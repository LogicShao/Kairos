import { useRef, useState } from "react"
import { AppBackground } from "@/components/shared/AppBackground"
import { AppShell } from "@/components/shared/AppShell"
import { AcrylicPanel } from "@/components/shared/acrylic-panel"
import { PomodoroTimer } from "@/components/pomodoro/PomodoroTimer"
import { TaskList } from "@/components/todo/TaskList"
import { CalendarView } from "@/components/calendar/CalendarView"
import { CourseSchedule } from "@/components/courses/CourseSchedule"
import { ExamList } from "@/components/exams/ExamList"
import { KairosHub } from "@/components/kairos/KairosHub"
import { NotificationSettings } from "@/components/settings/NotificationSettings"
import { SemesterPhaseSettings } from "@/components/settings/SemesterPhaseSettings"
import { AiSettings } from "@/components/settings/AiSettings"
import { SyncSettings } from "@/components/sync/SyncSettings"
import { LzuServicesPage } from "@/pages/lzu/LzuServicesPage"
import { TodayPage } from "@/pages/today/TodayPage"
import { useAndroidBack } from "@/hooks/use-android-back"

/** 主页面（底部 Tab 平级切换，不进导航栈；切换时重置导航栈） */
const MAIN_PAGES = ["today", "pomodoro", "todo", "calendar", "kairos"]

function MainApp() {
  const [active, setActive] = useState("today")
  // 子页面导航栈：进入子页面压栈，返回键出栈；空栈 = 主页面（返回键退出应用）。
  const [navStack, setNavStack] = useState<string[]>([])

  const navigate = (key: string) => {
    if (key === active) return
    if (MAIN_PAGES.includes(key)) {
      // Tab 平级切换：重置导航栈（Android 惯例：切换后返回键退出）。
      setNavStack([])
    } else {
      // 进入子页面：当前页压栈。
      setNavStack([...navStack, active])
    }
    setActive(key)
  }

  const goBack = () => {
    if (navStack.length === 0) return
    const prev = navStack[navStack.length - 1]
    setActive(prev)
    setNavStack(navStack.slice(0, -1))
  }

  // 返回键 handler 需要读取最新栈；渲染期同步 ref。
  const navStackRef = useRef(navStack)
  navStackRef.current = navStack
  useAndroidBack(navStackRef, goBack)

  return (
    <>
      <AppBackground />
      <AppShell active={active} onNavigate={navigate}>
        {active === "pomodoro" && (
          <div className="flex min-h-[60vh] md:min-h-[70vh] items-center justify-center">
            <AcrylicPanel className="w-full max-w-md p-5 sm:p-8 animate-in fade-in-0 zoom-in-95 duration-300">
              <PomodoroTimer />
            </AcrylicPanel>
          </div>
        )}
        {active === "today" && <TodayPage onNavigate={navigate} />}
        {active === "todo" && <TaskList />}
        {active === "calendar" && <CalendarView onNavigate={navigate} />}
        {active === "kairos" && <KairosHub onNavigate={navigate} />}
        {active === "courses" && <CourseSchedule onNavigate={navigate} />}
        {active === "exams" && <ExamList onNavigate={navigate} />}
        {active === "lzu-services" && <LzuServicesPage onNavigate={navigate} />}
        {active === "notifications" && <NotificationSettings onNavigate={navigate} />}
        {active === "semester-phases" && <SemesterPhaseSettings onNavigate={navigate} />}
        {active === "ai-settings" && <AiSettings onNavigate={navigate} />}
        {active === "sync" && <SyncSettings onNavigate={navigate} />}
      </AppShell>
    </>
  )
}

function App() {
  return <MainApp />
}

export default App
