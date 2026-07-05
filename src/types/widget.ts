export type WidgetMode = "small" | "medium" | "large"

export type MainNavigationTarget = "today" | "pomodoro" | "todo" | "courses" | "exams"

/** 与后端 db::models::WidgetConfig 对齐，是桌面小组件的本地单例配置。 */
export interface WidgetConfig {
  id: number
  /** true = 启动并显示 widget 窗口，false = 不创建/隐藏 widget 窗口。 */
  enabled: boolean
  /** 小组件布局模式，同时决定默认窗口尺寸。 */
  mode: WidgetMode
  /** 前端视觉透明度，范围 0.6..=1.0。 */
  opacity: number
  /** true = widget 窗口置顶。 */
  always_on_top: boolean
  /** true = 禁止拖动保存位置。 */
  locked: boolean
  /** 窗口左上角物理像素 x 坐标；null 表示尚未保存位置。 */
  x: number | null
  /** 窗口左上角物理像素 y 坐标；null 表示尚未保存位置。 */
  y: number | null
  /** 窗口宽度，逻辑像素。 */
  width: number
  /** 窗口高度，逻辑像素。 */
  height: number
  /** UTC ISO 8601 创建时间。 */
  created_at: string
  /** UTC ISO 8601 更新时间。 */
  updated_at: string
}

/** 与后端 db::models::UpdateWidgetConfigRequest 对齐，所有字段可选以支持局部更新。 */
export interface UpdateWidgetConfigRequest {
  enabled?: boolean
  mode?: WidgetMode
  opacity?: number
  always_on_top?: boolean
  locked?: boolean
  x?: number
  y?: number
  width?: number
  height?: number
}

/** 与后端 commands::widget::SaveWidgetPositionRequest 对齐。 */
export interface SaveWidgetPositionRequest {
  /** 窗口左上角物理像素 x 坐标。 */
  x: number
  /** 窗口左上角物理像素 y 坐标。 */
  y: number
  /** 窗口宽度，逻辑像素。 */
  width: number
  /** 窗口高度，逻辑像素。 */
  height: number
}

/** 后端 main-navigate 事件 payload。 */
export interface MainNavigateEvent {
  target: MainNavigationTarget
}
