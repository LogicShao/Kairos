const TOKEN_KEY = "kairos_token"

export const BASE = import.meta.env.VITE_API_BASE_URL ?? ""

export interface StatusResponse {
  status: string
}

export type QueryValue = string | number | boolean | null | undefined

export function getToken(): string | null {
  try {
    return window.localStorage.getItem(TOKEN_KEY)
  } catch {
    return null
  }
}

export function setToken(t: string): void {
  try {
    window.localStorage.setItem(TOKEN_KEY, t)
  } catch {
    return
  }
}

export function clearToken(): void {
  try {
    window.localStorage.removeItem(TOKEN_KEY)
  } catch {
    return
  }
}

export function buildQuery(params: Record<string, QueryValue>): string {
  const search = new URLSearchParams()
  for (const [key, value] of Object.entries(params)) {
    if (value === null || value === undefined) continue
    const text = String(value).trim()
    if (text === "") continue
    search.set(key, text)
  }
  const qs = search.toString()
  return qs ? `?${qs}` : ""
}

function statusFallbackMessage(status: number): string {
  switch (status) {
    case 400:
      return "请求无效"
    case 403:
      return "没有权限执行此操作"
    case 404:
      return "请求的资源不存在"
    case 409:
      return "请求与当前状态冲突"
    case 500:
      return "服务器内部错误"
    case 502:
      return "上游服务不可用"
    default:
      return `请求失败（HTTP ${status}）`
  }
}

export async function readErrorMessage(res: Response): Promise<string> {
  try {
    const data: unknown = await res.json()
    if (data !== null && typeof data === "object" && "error" in data) {
      const value = (data as { error: unknown }).error
      if (typeof value === "string" && value.trim() !== "") {
        return value
      }
    }
  } catch {
    return statusFallbackMessage(res.status)
  }
  return statusFallbackMessage(res.status)
}

export async function apiFetch<T>(path: string, init?: RequestInit): Promise<T> {
  const headers = new Headers(init?.headers)
  if (init?.body !== undefined && !headers.has("Content-Type")) {
    headers.set("Content-Type", "application/json")
  }
  const token = getToken()
  if (token) {
    headers.set("Authorization", `Bearer ${token}`)
  }

  const res = await fetch(`${BASE}${path}`, { ...init, headers })

  if (res.status === 401) {
    clearToken()
    window.dispatchEvent(new Event("kairos:unauthorized"))
    throw new Error("登录已过期，请重新登录")
  }
  if (!res.ok) {
    throw new Error(await readErrorMessage(res))
  }
  if (res.status === 204) {
    return undefined as T
  }
  return (await res.json()) as T
}
