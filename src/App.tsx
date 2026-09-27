import { useEffect, useState } from "react"
import { AppBackground } from "@/components/shared/AppBackground"
import { AppShell } from "@/components/shared/AppShell"
import { AcrylicPanel } from "@/components/shared/acrylic-panel"
import { LoginPage } from "@/components/auth/LoginPage"
import { getToken } from "@/lib/api/client"
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
import { TodayPage } from "@/pages/today/TodayPage"

function MainApp() {
  const [active, setActive] = useState("today")

  const navigate = (key: string) => {
    setActive(key)
  }

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
        {active === "notifications" && <NotificationSettings onNavigate={navigate} />}
        {active === "semester-phases" && <SemesterPhaseSettings onNavigate={navigate} />}
        {active === "ai-settings" && <AiSettings onNavigate={navigate} />}
        {active === "sync" && <SyncSettings onNavigate={navigate} />}
      </AppShell>
    </>
  )
}

function App() {
  const [authed, setAuthed] = useState(() => getToken() !== null)

  useEffect(() => {
    const handleUnauthorized = () => setAuthed(false)
    window.addEventListener("kairos:unauthorized", handleUnauthorized)
    return () => window.removeEventListener("kairos:unauthorized", handleUnauthorized)
  }, [])

  if (!authed) {
    return (
      <>
        <AppBackground />
        <LoginPage onSuccess={() => setAuthed(true)} />
      </>
    )
  }

  return <MainApp />
}

export default App
