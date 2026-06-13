import { Timer, CheckSquare, BookOpen, Clock } from "lucide-react"
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"

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
  return (
    <div className="min-h-screen bg-background flex flex-col items-center justify-center p-8">
      <div className="text-center mb-12">
        <h1 className="text-5xl font-heading font-medium text-foreground tracking-tight">
          Kairos
        </h1>
        <p className="mt-2 text-lg text-muted-foreground">
          καιρός — 恰当时机
        </p>
      </div>

      <div className="grid grid-cols-1 sm:grid-cols-2 gap-4 w-full max-w-2xl">
        {features.map(({ icon: Icon, title, description, color, bgColor }) => (
          <Card key={title} className="transition-colors hover:bg-accent/50">
            <CardHeader>
              <div
                className={`w-10 h-10 rounded-lg ${bgColor} flex items-center justify-center mb-2`}
              >
                <Icon className={`w-5 h-5 ${color}`} />
              </div>
              <CardTitle>{title}</CardTitle>
              <CardDescription>{description}</CardDescription>
            </CardHeader>
            <CardContent />
          </Card>
        ))}
      </div>

      <p className="mt-12 text-sm text-muted-foreground">
        Kairos v0.1.0
      </p>
    </div>
  )
}

export default App
