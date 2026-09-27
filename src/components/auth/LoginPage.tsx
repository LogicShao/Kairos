import { useState, type FormEvent } from "react"
import { Lock, LogIn, User } from "lucide-react"
import { AcrylicPanel } from "@/components/shared/acrylic-panel"
import { Button } from "@/components/ui/button"
import { cn } from "@/lib/utils"
import { userErrorMessage } from "@/lib/errors"
import { login } from "@/lib/api/auth"
import { setToken } from "@/lib/api/client"
import kairosLogo from "@/assets/kairos-logo.svg"

interface LoginPageProps {
  onSuccess: () => void
}

function inputClass(hasError: boolean): string {
  return cn(
    "w-full rounded-lg border bg-muted/40 px-3 py-2 text-sm text-foreground outline-none transition-colors",
    "placeholder:text-muted-foreground/45",
    "focus:ring-2 focus:ring-primary/30",
    hasError
      ? "border-destructive/60 focus:border-destructive focus:ring-destructive/20"
      : "border-border focus:border-primary",
  )
}

/** 登录页：用户名 + 密码，成功后写入 token 并通知 App 放行。 */
export function LoginPage({ onSuccess }: LoginPageProps) {
  const [username, setUsername] = useState("")
  const [password, setPassword] = useState("")
  const [error, setError] = useState<string | null>(null)
  const [submitting, setSubmitting] = useState(false)

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    if (submitting) return

    setSubmitting(true)
    setError(null)
    try {
      const res = await login(username.trim(), password)
      setToken(res.token)
      onSuccess()
    } catch (err) {
      setError(userErrorMessage(err, "登录失败，请检查用户名与密码"))
    } finally {
      setSubmitting(false)
    }
  }

  return (
    <div className="flex min-h-screen items-center justify-center px-4 py-8">
      <AcrylicPanel className="w-full max-w-sm p-6 sm:p-8 animate-in fade-in-0 zoom-in-95 duration-300">
        <div className="flex flex-col items-center text-center">
          <img src={kairosLogo} alt="Kairos" className="h-14 w-14 shrink-0 rounded-xl" />
          <h1 className="mt-3 font-heading text-xl font-semibold text-foreground">登录 Kairos</h1>
          <p className="mt-1 text-sm text-muted-foreground">καιρός——稍纵即逝的，正是此刻</p>
        </div>

        <form onSubmit={handleSubmit} className="mt-6 flex flex-col gap-4">
          <div>
            <label
              htmlFor="login-username"
              className="mb-1 block text-xs font-medium text-muted-foreground"
            >
              用户名
            </label>
            <div className="relative">
              <User className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground/60" />
              <input
                id="login-username"
                type="text"
                autoComplete="username"
                autoFocus
                required
                value={username}
                onChange={(event) => setUsername(event.target.value)}
                placeholder="请输入用户名"
                className={cn(inputClass(error !== null), "pl-9")}
              />
            </div>
          </div>

          <div>
            <label
              htmlFor="login-password"
              className="mb-1 block text-xs font-medium text-muted-foreground"
            >
              密码
            </label>
            <div className="relative">
              <Lock className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground/60" />
              <input
                id="login-password"
                type="password"
                autoComplete="current-password"
                required
                value={password}
                onChange={(event) => setPassword(event.target.value)}
                placeholder="请输入密码"
                className={cn(inputClass(error !== null), "pl-9")}
              />
            </div>
          </div>

          {error !== null && (
            <p role="alert" className="text-center text-sm text-destructive">
              {error}
            </p>
          )}

          <Button type="submit" disabled={submitting} className="mt-1 min-h-11 w-full md:min-h-0">
            <LogIn className="mr-1.5 h-4 w-4" />
            {submitting ? "登录中…" : "登录"}
          </Button>
        </form>
      </AcrylicPanel>
    </div>
  )
}
