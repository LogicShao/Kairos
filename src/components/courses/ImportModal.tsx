import { useState } from "react"
import type { LzuCourseImportResult } from "@/types/lzu"
import { Modal } from "@/components/shared/modal"
import { ClipboardImportPanel, type ClipboardImportPanelProps } from "./import/ClipboardImportPanel"
import { LzuImportPanel } from "./import/LzuImportPanel"
import { cn } from "@/lib/utils"

type ImportTab = "clipboard" | "lzu"

interface ImportModalProps extends ClipboardImportPanelProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  onLzuImportSuccess?: (result: LzuCourseImportResult) => Promise<void> | void
}

const TABS: { key: ImportTab; label: string }[] = [
  { key: "clipboard", label: "剪贴板导入" },
  { key: "lzu", label: "LZU API 导入" },
]

export function ImportModal({
  open,
  onOpenChange,
  onLzuImportSuccess,
  ...clipboardImportProps
}: ImportModalProps) {
  const [activeTab, setActiveTab] = useState<ImportTab>("clipboard")

  return (
    <Modal
      open={open}
      onOpenChange={onOpenChange}
      title="导入课表"
      description="从剪贴板或 LZU API 导入课程"
      className="max-w-2xl"
    >
      {/* Tab bar */}
      <div className="mb-4 flex border-b border-border/60">
        {TABS.map((tab) => (
          <button
            key={tab.key}
            type="button"
            onClick={() => setActiveTab(tab.key)}
            className={cn(
              "relative px-4 py-2 text-sm font-medium transition-colors",
              activeTab === tab.key
                ? "text-foreground"
                : "text-muted-foreground hover:text-foreground/80",
            )}
            aria-selected={activeTab === tab.key}
          >
            {tab.label}
            {activeTab === tab.key && (
              <span className="absolute inset-x-2 bottom-0 h-0.5 rounded-full bg-primary" />
            )}
          </button>
        ))}
      </div>

      {/* Panel content */}
      {activeTab === "clipboard" ? (
        <ClipboardImportPanel {...clipboardImportProps} />
      ) : (
        <LzuImportPanel onImportSuccess={onLzuImportSuccess} />
      )}
    </Modal>
  )
}
