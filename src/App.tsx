import { useEffect, useState } from "react"
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
import { WidgetSettings } from "@/components/settings/WidgetSettings"
import { SyncSettings } from "@/components/sync/SyncSettings"
import { WidgetApp } from "@/components/widget/WidgetApp"
import { LzuServicesPage } from "@/pages/lzu/LzuServicesPage"
import { TodayPage } from "@/pages/today/TodayPage"
import type { MainNavigateEvent } from "@/types/widget"
import { listenWithCleanup } from "@/lib/tauri-events"

function MainApp() {
  const [active, setActive] = useState("today")

  useEffect(() => {
    return listenWithCleanup<MainNavigateEvent>(
      "main-navigate",
      (event) => {
        setActive(event.payload.target)
      },
      () => undefined,
    )
  }, [])

  return (
    <>
      <AppBackground />
      <AppShell active={active} onNavigate={setActive}>
        {active === "pomodoro" && (
          <div className="flex min-h-[60vh] md:min-h-[70vh] items-center justify-center">
            <AcrylicPanel className="w-full max-w-md p-5 sm:p-8 animate-in fade-in-0 zoom-in-95 duration-300">
              <PomodoroTimer />
            </AcrylicPanel>
          </div>
        )}
        {active === "today" && <TodayPage onNavigate={setActive} />}
        {active === "todo" && <TaskList />}
        {active === "calendar" && <CalendarView onNavigate={setActive} />}
        {active === "kairos" && <KairosHub onNavigate={setActive} />}
        {active === "courses" && <CourseSchedule onNavigate={setActive} />}
        {active === "exams" && <ExamList onNavigate={setActive} />}
        {active === "lzu-services" && <LzuServicesPage onNavigate={setActive} />}
        {active === "notifications" && <NotificationSettings onNavigate={setActive} />}
        {active === "semester-phases" && <SemesterPhaseSettings onNavigate={setActive} />}
        {active === "ai-settings" && <AiSettings onNavigate={setActive} />}
        {active === "widget" && <WidgetSettings onNavigate={setActive} />}
        {active === "sync" && <SyncSettings onNavigate={setActive} />}
      </AppShell>
    </>
  )
}

function App() {
  const isWidget = new URLSearchParams(window.location.search).get("view") === "widget"
  return isWidget ? <WidgetApp /> : <MainApp />
}

export default App
