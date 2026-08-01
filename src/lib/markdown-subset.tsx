import type { ReactNode } from "react"
import { Fragment } from "react"

/**
 * 轻量 Markdown 子集渲染器（React 节点渲染，无 dangerouslySetInnerHTML，天然防 XSS）。
 *
 * 支持：`#` / `##` 标题、`**加粗**`、`*斜体*`、`- ` 无序列表、`---` 分隔线、空行分段。
 * 未知行按 `<p>` 兜底不报错。AI 提示词已约束输出格式，使本子集封闭可预测。
 */
function renderInline(text: string): ReactNode[] {
  // 先按 **加粗** 切分
  const boldParts = text.split(/\*\*(.+?)\*\*/g)
  return boldParts.map((part, i) => {
    if (i % 2 === 1) {
      return <strong key={i}>{part}</strong>
    }
    // 再按 *斜体* 切分（非加粗段内）
    const italicParts = part.split(/\*([^*]+)\*/g)
    return italicParts.map((sub, j) =>
      j % 2 === 1 ? <em key={`${i}-${j}`}>{sub}</em> : sub,
    )
  })
}

export function MarkdownSubset({ text }: { text: string }) {
  const blocks = text.split(/\n{2,}/)

  return (
    <div className="space-y-2 text-sm leading-relaxed">
      {blocks.map((block, i) => {
        const trimmed = block.trim()
        if (!trimmed) return null
        if (trimmed === "---") {
          return <hr key={i} className="border-border/60" />
        }

        const lines = trimmed.split("\n")
        if (trimmed.startsWith("## ")) {
          // 标题与正文可能只隔单个 \n（AI 输出风格），需拆成标题 + 正文段落。
          // 标题只取第一行，正文为其余行，避免整块 slice 导致正文重复渲染。
          const title = lines[0].slice(3)
          const rest = lines.slice(1).map((l) => l.trim()).filter(Boolean)
          return (
            <Fragment key={i}>
              <h3 className="pt-2 text-sm font-semibold text-foreground first:pt-0">
                {renderInline(title)}
              </h3>
              {rest.length > 0 && (
                <p className="text-muted-foreground">
                  {rest.map((line, j) => (
                    <Fragment key={j}>
                      {j > 0 && <br />}
                      {renderInline(line)}
                    </Fragment>
                  ))}
                </p>
              )}
            </Fragment>
          )
        }
        if (trimmed.startsWith("# ")) {
          const title = lines[0].slice(2)
          const rest = lines.slice(1).map((l) => l.trim()).filter(Boolean)
          return (
            <Fragment key={i}>
              <h2 className="text-base font-semibold text-foreground">
                {renderInline(title)}
              </h2>
              {rest.length > 0 && (
                <p className="text-muted-foreground">
                  {rest.map((line, j) => (
                    <Fragment key={j}>
                      {j > 0 && <br />}
                      {renderInline(line)}
                    </Fragment>
                  ))}
                </p>
              )}
            </Fragment>
          )
        }

        const listItems = lines.filter((line) => line.trim().startsWith("- "))
        if (listItems.length === lines.length && listItems.length > 0) {
          return (
            <ul key={i} className="list-disc space-y-0.5 pl-5 text-muted-foreground">
              {listItems.map((line, j) => (
                <li key={j}>{renderInline(line.trim().slice(2))}</li>
              ))}
            </ul>
          )
        }

        return (
          <p key={i} className="text-muted-foreground">
            {lines.map((line, j) => (
              <Fragment key={j}>
                {j > 0 && <br />}
                {renderInline(line)}
              </Fragment>
            ))}
          </p>
        )
      })}
    </div>
  )
}
