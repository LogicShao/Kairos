export interface RgbColor {
  r: number
  g: number
  b: number
}

/** 解析 #RGB / #RRGGBB，失败返回 null。 */
export function hexToRgb(hex: string): RgbColor | null {
  const raw = hex.trim().replace(/^#/, "")
  const normalized =
    raw.length === 3
      ? raw
          .split("")
          .map((char) => char + char)
          .join("")
      : raw
  const match = /^([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(normalized)
  if (!match) return null
  return {
    r: parseInt(match[1], 16),
    g: parseInt(match[2], 16),
    b: parseInt(match[3], 16),
  }
}

export function hexToRgba(hex: string, alpha: number): string {
  const rgb = hexToRgb(hex)
  if (!rgb) return hex
  return `rgba(${rgb.r}, ${rgb.g}, ${rgb.b}, ${alpha})`
}

export function isDarkColor(hex: string): boolean {
  const rgb = hexToRgb(hex)
  if (!rgb) return false
  const luminance = (0.299 * rgb.r + 0.587 * rgb.g + 0.114 * rgb.b) / 255
  return luminance <= 0.55
}

function channelToHex(value: number): string {
  const channel = Math.max(0, Math.min(255, Math.round(value)))
  return channel.toString(16).padStart(2, "0")
}

/** 将 color 与 base 混合，amount 为 base 的占比（0–1）。 */
export function blendHex(color: string, base: string, amount: number): string {
  const from = hexToRgb(color)
  const to = hexToRgb(base)
  if (!from || !to) return color
  const ratio = Math.max(0, Math.min(1, amount))
  const mix = (a: number, b: number) => channelToHex(a * (1 - ratio) + b * ratio)
  return `#${mix(from.r, to.r)}${mix(from.g, to.g)}${mix(from.b, to.b)}`
}

export interface BlockPalette {
  /** 色块底色：事件色与卡片色的浅混色。 */
  background: string
  /** 描边色。 */
  border: string
  /** 正文墨色：事件色与前景色的混合，保证深浅主题下都可读。 */
  ink: string
  /** 左侧书脊使用的纯事件色。 */
  accent: string
}

/** 生成时间块配色：事件色只做浅底与书脊，文字使用混墨色。 */
export function blockPalette(color: string, dark: boolean): BlockPalette {
  const paper = dark ? "#3d3d3d" : "#ffffff"
  const ink = dark ? "#ececec" : "#232323"
  return {
    background: blendHex(color, paper, dark ? 0.82 : 0.84),
    border: blendHex(color, paper, dark ? 0.55 : 0.6),
    ink: blendHex(color, ink, dark ? 0.62 : 0.68),
    accent: color,
  }
}
