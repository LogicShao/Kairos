import {
  listen,
  type EventCallback,
  type EventName,
  type UnlistenFn,
} from "@tauri-apps/api/event"

export function listenWithCleanup<T>(
  event: EventName,
  handler: EventCallback<T>,
  onError: (error: unknown) => void,
): () => void {
  let disposed = false
  let unlisten: UnlistenFn | undefined

  listen<T>(event, handler)
    .then((fn) => {
      if (disposed) {
        fn()
        return
      }
      unlisten = fn
    })
    .catch((error) => {
      if (!disposed) {
        onError(error)
      }
    })

  return () => {
    disposed = true
    unlisten?.()
  }
}
