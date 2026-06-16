import { useCallback, useSyncExternalStore } from "react"

const THEME_KEY = "kairos-theme"

function getSnapshot(): "dark" | "light" {
  return document.documentElement.classList.contains("dark") ? "dark" : "light"
}

function subscribe(callback: () => void) {
  const observer = new MutationObserver(callback)
  observer.observe(document.documentElement, { attributes: true, attributeFilter: ["class"] })
  return () => observer.disconnect()
}

function resolveTheme(): "dark" | "light" {
  const stored = localStorage.getItem(THEME_KEY)
  if (stored === "dark" || stored === "light") return stored
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light"
}

export function applyTheme(theme: "dark" | "light") {
  const root = document.documentElement
  if (theme === "dark") {
    root.classList.add("dark")
  } else {
    root.classList.remove("dark")
  }
  localStorage.setItem(THEME_KEY, theme)
  // 触发自定义事件通知 FluentProvider
  window.dispatchEvent(new Event('theme-change'))
}

export function useTheme() {
  const theme = useSyncExternalStore(subscribe, getSnapshot)

  const toggle = useCallback(() => {
    applyTheme(theme === "dark" ? "light" : "dark")
  }, [theme])

  const setTheme = useCallback((t: "dark" | "light") => {
    applyTheme(t)
  }, [])

  return { theme, toggle, setTheme }
}

applyTheme(resolveTheme())
