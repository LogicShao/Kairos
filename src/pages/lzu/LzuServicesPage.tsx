import { useEffect, useMemo, useState } from "react"
import { invoke } from "@tauri-apps/api/core"
import {
  AlertCircle,
  ArrowLeft,
  CreditCard,
  ExternalLink,
  Grid2X2,
  Loader2,
  LogIn,
  LogOut,
  RefreshCw,
  Search,
  User,
} from "lucide-react"
import { AcrylicPanel } from "@/components/shared/acrylic-panel"
import { Button } from "@/components/ui/button"
import type {
  LzuAuthStatus,
  LzuCampusCardOverview,
  LzuServiceCategory,
  LzuServiceDirectory,
} from "@/types/lzu"
import { cn } from "@/lib/utils"

const FIELD_CLASS =
  "w-full rounded-lg border border-border/70 bg-background/70 px-3 py-2.5 text-sm outline-none transition-colors focus:border-primary"

interface LzuServicesPageProps {
  onNavigate: (key: string) => void
}

export function LzuServicesPage({ onNavigate }: LzuServicesPageProps) {
  // ── auth ──
  const [authStatus, setAuthStatus] = useState<LzuAuthStatus | null>(null)
  const [authLoading, setAuthLoading] = useState(true)
  const [username, setUsername] = useState("")
  const [password, setPassword] = useState("")
  const [loginLoading, setLoginLoading] = useState(false)
  const [loginError, setLoginError] = useState<string | null>(null)

  // ── card / directory ──
  const [card, setCard] = useState<LzuCampusCardOverview | null>(null)
  const [directory, setDirectory] = useState<LzuServiceDirectory | null>(null)
  const [cardLoading, setCardLoading] = useState(false)
  const [directoryLoading, setDirectoryLoading] = useState(false)
  const [cardError, setCardError] = useState<string | null>(null)
  const [directoryError, setDirectoryError] = useState<string | null>(null)
  const [query, setQuery] = useState("")

  // ── auth helpers ──
  async function handleLogin() {
    if (!username.trim() || !password.trim()) {
      setLoginError("请输入账号和密码")
      return
    }
    setLoginLoading(true)
    setLoginError(null)
    try {
      const status = await invoke<LzuAuthStatus>("lzu_login", {
        username: username.trim(),
        password,
      })
      setAuthStatus(status)
      setUsername("")
      setPassword("")
    } catch (e) {
      setLoginError(typeof e === "string" ? e : "登录失败，请检查账号和密码。")
    } finally {
      setLoginLoading(false)
    }
  }

  async function handleLogout() {
    setLoginLoading(true)
    try {
      const status = await invoke<LzuAuthStatus>("lzu_logout")
      setAuthStatus(status)
      setCard(null)
      setDirectory(null)
      setCardError(null)
      setDirectoryError(null)
    } catch {
      setAuthStatus({ is_logged_in: false, username: null, profile: null })
    } finally {
      setLoginLoading(false)
    }
  }

  /// 会话失效后的快速恢复：重查认证状态，若已登出则预填用户名提示重登。
  async function recoverSession(prevUsername: string | null) {
    try {
      const status = await invoke<LzuAuthStatus>("lzu_get_auth_status")
      if (!status.is_logged_in) {
        setAuthStatus(status)
        setCard(null)
        setDirectory(null)
        setCardError(null)
        setDirectoryError(null)
        const name = prevUsername ?? status.username
        if (name) {
          setUsername(name)
        }
        setLoginError("登录已过期，请重新输入密码登录。")
      }
    } catch {
      // 静默降级
    }
  }

  // ── card / directory helpers ──
  async function loadCard() {
    setCardLoading(true)
    setCardError(null)
    try {
      const result = await invoke<LzuCampusCardOverview>("lzu_get_campus_card_overview")
      setCard(result)
    } catch (error) {
      const msg = typeof error === "string" ? error : "校园卡余额读取失败。"
      setCardError(msg)
      void recoverSession(authStatus?.username ?? null)
    } finally {
      setCardLoading(false)
    }
  }

  async function loadDirectory() {
    setDirectoryLoading(true)
    setDirectoryError(null)
    try {
      const result = await invoke<LzuServiceDirectory>("lzu_get_service_directory")
      setDirectory(result)
    } catch (error) {
      const msg = typeof error === "string" ? error : "服务目录读取失败。"
      setDirectoryError(msg)
      void recoverSession(authStatus?.username ?? null)
    } finally {
      setDirectoryLoading(false)
    }
  }

  async function handleOpenService(service: { id: string | null; name: string; h5_service_url: string | null }) {
    if (!service.id || !service.h5_service_url) return
    try {
      await invoke("lzu_open_service", {
        serviceId: service.id,
        h5Url: service.h5_service_url,
      })
    } catch (e) {
      console.warn("failed to open service:", e)
    }
  }

  // ── init ──
  useEffect(() => {
    let cancelled = false
    async function load() {
      try {
        const status = await invoke<LzuAuthStatus>("lzu_get_auth_status")
        if (!cancelled) setAuthStatus(status)
      } catch {
        if (!cancelled)
          setAuthStatus({ is_logged_in: false, username: null, profile: null })
      } finally {
        if (!cancelled) setAuthLoading(false)
      }
    }
    void load()
    return () => {
      cancelled = true
    }
  }, [])

  useEffect(() => {
    if (!authStatus?.is_logged_in) return
    const timer = window.setTimeout(() => {
      void loadCard()
      void loadDirectory()
    }, 0)
    return () => window.clearTimeout(timer)
  }, [authStatus?.is_logged_in])

  const filteredCategories = useMemo(() => {
    const normalized = query.trim().toLowerCase()
    const categories = directory?.categories ?? []
    if (!normalized) return categories

    return categories
      .map((category) => ({
        ...category,
        services: category.services.filter((service) => {
          const haystack = [
            category.name,
            service.name,
            service.category_name ?? "",
            service.introduce ?? "",
          ]
            .join(" ")
            .toLowerCase()
          return haystack.includes(normalized)
        }),
      }))
      .filter((category) => category.services.length > 0)
  }, [directory, query])

  const primaryWallet = card?.wallets.find((wallet) => wallet.wallet_money) ?? null
  const profile = authStatus?.profile ?? null
  const displayName = profile?.display_name ?? authStatus?.username ?? "已登录"

  // ── loading auth state ──
  if (authLoading) {
    return (
      <div className="flex min-h-0 flex-1 items-center justify-center">
        <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
      </div>
    )
  }

  // ── not logged in: login form ──
  if (!authStatus?.is_logged_in) {
    return (
      <div className="min-h-0 flex-1 overflow-y-auto pb-4">
        <div className="mx-auto flex w-full max-w-md flex-col gap-4">
          <div className="flex items-start gap-2">
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="h-11 md:h-7"
              onClick={() => onNavigate("kairos")}
            >
              <ArrowLeft className="mr-1.5 h-4 w-4" />
              返回
            </Button>
          </div>

          <AcrylicPanel className="bg-card p-5">
            <div className="mb-4 flex items-center gap-3">
              <span className="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
                <CreditCard className="h-5 w-5" />
              </span>
              <div>
                <h1 className="text-lg font-semibold text-foreground">LZU 校园服务</h1>
                <p className="mt-0.5 text-sm text-muted-foreground">
                  登录以查看校园卡余额与服务目录
                </p>
              </div>
            </div>

            <div className="space-y-3">
              <div>
                <label className="mb-1 block text-xs font-medium text-muted-foreground">账号</label>
                <input
                  type="text"
                  value={username}
                  onChange={(e) => setUsername(e.target.value)}
                  placeholder="学号/工号"
                  className={FIELD_CLASS}
                  autoComplete="username"
                  disabled={loginLoading}
                />
              </div>
              <div>
                <label className="mb-1 block text-xs font-medium text-muted-foreground">密码</label>
                <input
                  type="password"
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  placeholder="统一认证密码"
                  className={FIELD_CLASS}
                  autoComplete="current-password"
                  disabled={loginLoading}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") void handleLogin()
                  }}
                />
              </div>
            </div>

            {loginError && (
              <p className="mt-3 flex items-start gap-1.5 text-sm text-destructive">
                <AlertCircle className="mt-0.5 h-3.5 w-3.5 shrink-0" />
                <span>{loginError}</span>
              </p>
            )}

            <Button
              type="button"
              className="mt-4 w-full"
              size="sm"
              disabled={loginLoading || !username.trim() || !password.trim()}
              onClick={() => void handleLogin()}
            >
              {loginLoading ? (
                <>
                  <Loader2 className="mr-1.5 h-3.5 w-3.5 animate-spin" />
                  登录中...
                </>
              ) : (
                <>
                  <LogIn className="mr-1.5 h-3.5 w-3.5" />
                  登录
                </>
              )}
            </Button>
          </AcrylicPanel>
        </div>
      </div>
    )
  }

  // ── logged in: card + directory ──
  return (
    <div className="min-h-0 flex-1 overflow-y-auto pb-4">
      <div className="mx-auto flex w-full max-w-5xl flex-col gap-4">
        <div className="flex items-center justify-between gap-3">
          <Button
            type="button"
            variant="ghost"
            size="sm"
            className="h-11 md:h-7"
            onClick={() => onNavigate("kairos")}
          >
            <ArrowLeft className="mr-1.5 h-4 w-4" />
            返回
          </Button>
          <div className="flex items-center gap-3">
            {/* identity badge */}
            <div className="hidden items-center gap-2 rounded-lg border border-border/60 bg-background/60 px-2.5 py-1.5 sm:flex">
              <User className="h-3.5 w-3.5 text-muted-foreground" />
              <span className="truncate text-xs font-medium text-foreground">{displayName}</span>
              {profile?.campus_card_tail && (
                <span className="text-xs text-muted-foreground">卡尾号 {profile.campus_card_tail}</span>
              )}
            </div>
            <div className="flex items-center gap-1">
              <span className="text-sm text-muted-foreground">
                <Grid2X2 className="h-4 w-4" />
              </span>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                className="h-11 md:h-7"
                disabled={loginLoading}
                onClick={() => void handleLogout()}
              >
                {loginLoading ? (
                  <Loader2 className="h-3.5 w-3.5 animate-spin" />
                ) : (
                  <>
                    <LogOut className="mr-1 h-3.5 w-3.5" />
                    退出
                  </>
                )}
              </Button>
            </div>
          </div>
        </div>

        <div className="flex flex-col gap-4">
          <AcrylicPanel className="bg-card p-4">
            <div className="flex items-start justify-between gap-3">
              <div className="flex min-w-0 items-center gap-3">
                <span className="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
                  <CreditCard className="h-5 w-5" />
                </span>
                <div className="min-w-0">
                  <h1 className="text-base font-semibold text-foreground">校园卡</h1>
                  <p className="mt-0.5 text-xs text-muted-foreground">
                    {card?.account.card_tail ? `尾号 ${card.account.card_tail}` : "余额只读"}
                  </p>
                </div>
              </div>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                className="h-11 min-w-11 md:h-7 md:min-w-0"
                disabled={cardLoading}
                onClick={() => void loadCard()}
                aria-label="刷新校园卡"
              >
                {cardLoading ? (
                  <Loader2 className="h-4 w-4 animate-spin" />
                ) : (
                  <RefreshCw className="h-4 w-4" />
                )}
              </Button>
            </div>

            <div className="mt-4 rounded-lg border border-border/60 bg-background/60 p-4">
              {cardLoading && !card ? (
                <LoadingInline label="正在读取余额" />
              ) : (
                <>
                  <div className="text-xs text-muted-foreground">当前余额</div>
                  <div className="mt-2 truncate text-3xl font-semibold tabular-nums text-foreground">
                    {primaryWallet?.wallet_money ?? "--"}
                    {primaryWallet?.unit && (
                      <span className="ml-1 text-sm font-medium text-muted-foreground">
                        {primaryWallet.unit}
                      </span>
                    )}
                  </div>
                  <div className="mt-1 truncate text-xs text-muted-foreground">
                    {primaryWallet?.wallet_name ?? "暂无钱包数据"}
                  </div>
                </>
              )}
            </div>

            {!cardLoading && !cardError && card && card.wallets.length === 0 && (
              <div className="mt-3 rounded-lg border border-dashed border-border/70 bg-background/50 px-4 py-6 text-center text-sm text-muted-foreground">
                暂无钱包数据
              </div>
            )}

            {cardError && <InlineError message={cardError} className="mt-3" />}

            <div className="mt-4 space-y-2">
              {(card?.wallets ?? []).map((wallet, index) => (
                <div
                  key={`${wallet.wallet_num ?? "wallet"}-${index}`}
                  className="flex items-center justify-between gap-3 rounded-lg border border-border/50 bg-card/60 px-3 py-2"
                >
                  <div className="min-w-0">
                    <div className="truncate text-sm font-medium text-foreground">
                      {wallet.wallet_name ?? wallet.card_name ?? "钱包"}
                    </div>
                    {wallet.wallet_num && (
                      <div className="text-xs text-muted-foreground">编号 {wallet.wallet_num}</div>
                    )}
                  </div>
                  <div className="shrink-0 text-sm font-semibold tabular-nums">
                    {wallet.wallet_money ?? "--"}
                  </div>
                </div>
              ))}
            </div>
          </AcrylicPanel>

          <AcrylicPanel className="min-w-0 bg-card p-4">
            <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
              <div className="min-w-0">
                <h2 className="text-base font-semibold text-foreground">服务目录</h2>
                <p className="mt-0.5 text-xs text-muted-foreground">
                  {directory ? `${serviceCount(directory.categories)} 项服务` : "低敏摘要"}
                </p>
              </div>
              <div className="flex items-center gap-2">
                <label className="relative block min-w-0">
                  <Search className="pointer-events-none absolute left-2.5 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
                  <input
                    value={query}
                    onChange={(event) => setQuery(event.target.value)}
                    placeholder="搜索服务"
                    className="h-11 w-full rounded-lg border border-border/70 bg-background/70 pl-8 pr-3 text-sm outline-none transition-colors focus:border-primary sm:w-56 md:h-9"
                  />
                </label>
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  className="h-11 min-w-11 md:h-7 md:min-w-0"
                  disabled={directoryLoading}
                  onClick={() => void loadDirectory()}
                  aria-label="刷新服务目录"
                >
                  {directoryLoading ? (
                    <Loader2 className="h-4 w-4 animate-spin" />
                  ) : (
                    <RefreshCw className="h-4 w-4" />
                  )}
                </Button>
              </div>
            </div>

            {directoryError && <InlineError message={directoryError} className="mt-3" />}

            <div className="mt-4 space-y-5">
              {directoryLoading && !directory && <LoadingInline label="正在加载服务目录" />}

              {!directoryLoading && filteredCategories.map((category) => (
                <section key={category.id ?? category.name} className="space-y-2">
                  <div className="flex items-center justify-between gap-2">
                    <h3 className="truncate text-sm font-semibold text-foreground">
                      {category.name}
                    </h3>
                    <span className="shrink-0 text-xs text-muted-foreground">
                      {category.services.length}
                    </span>
                  </div>
                  <div className="grid gap-2 sm:grid-cols-2 xl:grid-cols-3">
                    {category.services.map((service) => (
                      <div
                        key={service.id ?? `${category.name}-${service.name}`}
                        role={service.h5_service_url ? "button" : undefined}
                        tabIndex={service.h5_service_url ? 0 : undefined}
                        onClick={() => void handleOpenService(service)}
                        onKeyDown={(e) => {
                          if (e.key === "Enter") void handleOpenService(service)
                        }}
                        className={cn(
                          "min-h-24 rounded-lg border border-border/60 bg-background/60 p-3",
                          service.h5_service_url && "cursor-pointer transition-colors hover:border-primary/40 hover:bg-background/80",
                        )}
                      >
                        <div className="flex items-start gap-2.5">
                          <ServiceIcon service={service} />
                          <div className="min-w-0 flex-1">
                            <div className="flex items-center gap-1.5 truncate text-sm font-medium text-foreground">
                              <span className="truncate">{service.name}</span>
                              {service.h5_service_url && (
                                <ExternalLink className="h-3 w-3 shrink-0 text-muted-foreground/50" />
                              )}
                            </div>
                            {service.introduce && (
                              <div className="mt-1 line-clamp-2 text-xs leading-5 text-muted-foreground">
                                {service.introduce}
                              </div>
                            )}
                          </div>
                        </div>
                        <div className="mt-3 flex flex-wrap gap-1.5">
                          {service.requires_login && <Badge>登录</Badge>}
                          {service.is_hot && <Badge>热门</Badge>}
                          {service.is_new && <Badge>新</Badge>}
                          {service.is_top && <Badge>置顶</Badge>}
                        </div>
                      </div>
                    ))}
                  </div>
                </section>
              ))}

              {!directoryLoading && !directoryError && filteredCategories.length === 0 && (
                <div className="rounded-lg border border-dashed border-border/70 bg-background/50 px-4 py-8 text-center text-sm text-muted-foreground">
                  暂无匹配服务
                </div>
              )}
            </div>
          </AcrylicPanel>
        </div>
      </div>
    </div>
  )
}

function LoadingInline({ label }: { label: string }) {
  return (
    <div className="flex min-h-16 items-center justify-center gap-2 text-sm text-muted-foreground">
      <Loader2 className="h-4 w-4 animate-spin" />
      {label}
    </div>
  )
}

function InlineError({ message, className }: { message: string; className?: string }) {
  return (
    <div
      className={cn(
        "flex items-start gap-2 rounded-lg border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive",
        className,
      )}
    >
      <AlertCircle className="mt-0.5 h-4 w-4 shrink-0" />
      <span className="min-w-0 break-words">{message}</span>
    </div>
  )
}

function Badge({ children }: { children: string }) {
  return (
    <span className="rounded border border-border/60 bg-muted/60 px-1.5 py-0.5 text-[11px] text-muted-foreground">
      {children}
    </span>
  )
}

function ServiceIcon({
  service,
}: {
  service: { icon_url: string | null; name: string }
}) {
  return (
    <span className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-xs font-semibold text-primary">
      {service.name.trim().slice(0, 1) || <Grid2X2 className="h-4 w-4" />}
    </span>
  )
}

function serviceCount(categories: LzuServiceCategory[]) {
  return categories.reduce((total, category) => total + category.services.length, 0)
}
