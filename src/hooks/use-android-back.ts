import { useEffect, useRef, type RefObject } from "react"
import { onBackButtonPress } from "@tauri-apps/api/app"
import type { PluginListener } from "@tauri-apps/api/core"
import { invoke } from "@tauri-apps/api/core"

/**
 * Tauri Android webview 的 UA 恒含 Android 标识；桌面端恒为 false。
 * 同步判断，避免引入 plugin-os 依赖与权限。
 */
const IS_ANDROID =
  typeof navigator !== "undefined" && navigator.userAgent.includes("Android")

/**
 * 注册 Android 系统返回键处理：
 * - 导航栈非空（有上级页面）→ 执行 `onBack`（返回上一页）；
 * - 导航栈为空（主页面）→ 调用后端 `exit_app` 退出应用到桌面。
 *
 * 仅在 Android 生效；桌面端不注册，行为零改动。
 *
 * StrictMode 双 mount 防护：注册是异步 IPC，cleanup 用 mounted 标志避免
 * 首次 mount 的 listener 泄漏；第二次 mount 的 listener 正常保留。
 */
export function useAndroidBack(
  stackRef: RefObject<string[]>,
  onBack: () => void,
) {
  // handler 用 ref 间接调用，避免闭包捕获过期回调。
  const onBackRef = useRef(onBack)
  onBackRef.current = onBack

  useEffect(() => {
    if (!IS_ANDROID) return

    let unlisten: PluginListener | null = null
    let mounted = true

    void onBackButtonPress(() => {
      if (stackRef.current.length > 0) {
        onBackRef.current()
      } else {
        void invoke("exit_app")
      }
    }).then((listener) => {
      if (mounted) {
        unlisten = listener
      } else {
        // 首次 mount 已被 cleanup 标记卸载：立即注销，防止重复监听。
        void listener.unregister()
      }
    })

    return () => {
      mounted = false
      if (unlisten) void unlisten.unregister()
    }
  }, [stackRef])
}
