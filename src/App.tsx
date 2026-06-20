import { useState } from "react"
import { AppBackground } from "@/components/shared/AppBackground"
import { AppShell } from "@/components/shared/AppShell"
import { AcrylicPanel } from "@/components/shared/acrylic-panel"
import { PomodoroTimer } from "@/components/pomodoro/PomodoroTimer"
import { TaskList } from "@/components/todo/TaskList"
import { CourseSchedule } from "@/components/courses/CourseSchedule"
import { ExamList } from "@/components/exams/ExamList"
import { SyncSettings } from "@/components/sync/SyncSettings"

function App() {
  const [active, setActive] = useState("pomodoro")

  return (
    <>
      <AppBackground />
      <AppShell active={active} onNavigate={setActive}>
        {active === "pomodoro" && (
          <div className="flex min-h-[70vh] items-center justify-center">
            <AcrylicPanel className="w-full max-w-md p-8 animate-in fade-in-0 zoom-in-95 duration-300">
              <PomodoroTimer />
            </AcrylicPanel>
          </div>
        )}
        {active === "todo" && <TaskList />}
        {active === "courses" && <CourseSchedule />}
        {active === "exams" && <ExamList />}
        {active === "sync" && <SyncSettings />}
      </AppShell>
    </>
  )
}

export default App
