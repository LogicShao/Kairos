import { useEffect, useMemo, useState } from "react"
import { invoke } from "@tauri-apps/api/core"
import {
  AlertCircle,
  ArrowLeft,
  CreditCard,
  Grid2X2,
  Loader2,
  RefreshCw,
  Search,
} from "lucide-react"
import { AcrylicPanel } from "@/components/shared/acrylic-panel"
import { Button } from "@/components/ui/button"
import type {
  LzuCampusCardOverview,
  LzuServiceCategory,
  LzuServiceDirectory,
} from "@/types/lzu"
import { cn } from "@/lib/utils"

interface LzuServicesPageProps {
  onNavigate: (key: string) => void
}

export function LzuServicesPage({ onNavigate }: LzuServicesPageProps) {
  const [card, setCard] = useState<LzuCampusCardOverview | null>(null)
  const [directory, setDirectory] = useState<LzuServiceDirectory | null>(null)
  const [cardLoading, setCardLoading] = useState(false)
  const [directoryLoading, setDirectoryLoading] = useState(false)
  const [cardError, setCardError] = useState<string | null>(null)
  const [directoryError, setDirectoryError] = useState<string | null>(null)
  const [query, setQuery] = useState("")

  async function loadCard() {
    setCardLoading(true)
    setCardError(null)
    try {
      const result = await invoke<LzuCampusCardOverview>("lzu_get_campus_card_overview")
      setCard(result)
    } catch (error) {
      setCardError(typeof error === "string" ? error : "校园卡余额读取失败。")
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
      setDirectoryError(typeof error === "string" ? error : "服务目录读取失败。")
    } finally {
      setDirectoryLoading(false)
    }
  }

  useEffect(() => {
    const timer = window.setTimeout(() => {
      void loadCard()
      void loadDirectory()
    }, 0)
    return () => window.clearTimeout(timer)
  }, [])

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
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            <Grid2X2 className="h-4 w-4" />
            LZU
          </div>
        </div>

        <div className="grid gap-4 lg:grid-cols-[320px_minmax(0,1fr)]">
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
                        className="min-h-24 rounded-lg border border-border/60 bg-background/60 p-3"
                      >
                        <div className="flex items-start gap-2.5">
                          <ServiceIcon service={service} />
                          <div className="min-w-0 flex-1">
                            <div className="truncate text-sm font-medium text-foreground">
                              {service.name}
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
