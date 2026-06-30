import { useEffect, useState, type ReactNode } from "react"
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow"
import { Minus, Maximize2, Minimize2, X } from "lucide-react"
import { cn } from "@/lib/utils"

interface TitleBarProps {
  title?: string
  showMinimize?: boolean
  showMaximize?: boolean
  showClose?: boolean
  leftActions?: ReactNode
  rightActions?: ReactNode
  onDoubleClick?: () => void
}

export function TitleBar({
  title,
  showMinimize = true,
  showMaximize = true,
  showClose = true,
  leftActions,
  rightActions,
  onDoubleClick,
}: TitleBarProps) {
  const [isMaximized, setIsMaximized] = useState(false)

  useEffect(() => {
    if (!showMaximize) return

    const appWindow = getCurrentWebviewWindow()

    appWindow.isMaximized().then(setIsMaximized)

    const unlisten = appWindow.onResized(async () => {
      const maximized = await appWindow.isMaximized()
      setIsMaximized(maximized)
    })

    return () => {
      unlisten.then((fn) => fn())
    }
  }, [showMaximize])

  const handleMinimize = async (e: React.MouseEvent) => {
    e.preventDefault()
    const appWindow = getCurrentWebviewWindow()
    await appWindow.minimize()
  }

  const handleToggleMaximize = async (e: React.MouseEvent) => {
    e.preventDefault()
    const appWindow = getCurrentWebviewWindow()
    await appWindow.toggleMaximize()
  }

  const handleClose = async (e: React.MouseEvent) => {
    e.preventDefault()
    const appWindow = getCurrentWebviewWindow()
    await appWindow.close()
  }

  const handleDragRegionDoubleClick = () => {
    if (onDoubleClick) {
      onDoubleClick()
    } else if (showMaximize) {
      getCurrentWebviewWindow().toggleMaximize()
    }
  }

  return (
    <div
      className={cn(
        "bg-background/95 supports-backdrop-filter:bg-background/60 border-border/40 flex h-8 items-center justify-between border-b backdrop-blur select-none",
        showMaximize && isMaximized ? "" : "rounded-t-lg"
      )}
    >
      {/* Left: Drag region + icon + title + actions */}
      <div
        data-tauri-drag-region
        onDoubleClick={handleDragRegionDoubleClick}
        className="flex grow items-center gap-2 pl-2"
      >
        <img
          src="/icon.png"
          alt="LynxOCR"
          className="size-4 shrink-0"
          draggable={false}
        />
        {title && (
          <span className="text-sm font-medium text-slate-400">{title}</span>
        )}
        {leftActions}
      </div>

      {/* Right: Control buttons */}
      <div className="flex items-center">
        {rightActions}

        {rightActions && (showMinimize || showMaximize || showClose) && (
          <div className="bg-border/40 mx-1 h-4 w-px" />
        )}

        {showMinimize && (
          <button
            onMouseDown={handleMinimize}
            className="title-bar-control"
            aria-label="Minimize"
            tabIndex={-1}
          >
            <Minus className="h-4 w-4" />
          </button>
        )}

        {showMaximize && (
          <button
            onMouseDown={handleToggleMaximize}
            className="title-bar-control"
            aria-label={isMaximized ? "Restore" : "Maximize"}
            tabIndex={-1}
          >
            {isMaximized ? (
              <Minimize2 className="h-4 w-4" />
            ) : (
              <Maximize2 className="h-4 w-4" />
            )}
          </button>
        )}

        {showClose && (
          <button
            onMouseDown={handleClose}
            className="title-bar-control hover:bg-destructive hover:text-destructive-foreground"
            aria-label="Close"
            tabIndex={-1}
          >
            <X className="h-4 w-4" />
          </button>
        )}
      </div>
    </div>
  )
}