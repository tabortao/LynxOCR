import { useState, useEffect, useCallback, useRef, lazy, Suspense } from "react"
import { AppSidebar } from "@/components/app-sidebar"
import { SidebarInset, SidebarProvider } from "@/components/ui/sidebar"
import { TooltipProvider } from "@/components/ui/tooltip"
import { WindowFrame } from "@/components/window-frame"
import { TitleBar } from "@/components/title-bar"
import { SidebarTrigger } from "@/components/ui/sidebar"
import { Separator } from "@/components/ui/separator"
import { AppProvider, useAppContext } from "@/lib/app-context"
import { invoke } from "@tauri-apps/api/core"
import { listen, type UnlistenFn } from "@tauri-apps/api/event"
import type { AppConfig } from "@/types"

// Lazy-load pages to reduce initial memory usage
const OCRPage = lazy(() =>
  import("@/app/ocr/page").then((m) => ({ default: m.OCRPage }))
)
const SettingsPage = lazy(() =>
  import("@/app/settings/page").then((m) => ({ default: m.SettingsPage }))
)
const ModelSettingsPage = lazy(() =>
  import("@/app/settings/model-settings").then((m) => ({
    default: m.ModelSettingsPage,
  }))
)
const ApiSettingsPage = lazy(() =>
  import("@/app/api-settings/page").then((m) => ({
    default: m.ApiSettingsPage,
  }))
)
const AboutPage = lazy(() =>
  import("@/app/about/page").then((m) => ({ default: m.AboutPage }))
)

function PageFallback() {
  return (
    <div className="flex items-center justify-center py-16">
      <div className="h-6 w-6 animate-spin rounded-full border-b-2 border-primary" />
    </div>
  )
}

export type Page =
  | "ocr"
  | "settings"
  | "model-settings"
  | "api-settings"
  | "about"

function AppTitleBar({ currentPage }: { currentPage: Page }) {
  const { t } = useAppContext()
  const title = t(`header.${currentPage}`)

  return <TitleBar title={title} />
}

export default function App() {
  const [currentPage, setCurrentPage] = useState<Page>("ocr")
  const [sidebarOpen, setSidebarOpen] = useState(true)
  const ocrModelVersionRef = useRef("ppocr-v5")
  const toastRef = useRef<HTMLDivElement | null>(null)

  // Load sidebar state from config on mount
  useEffect(() => {
    const load = async () => {
      try {
        const config = await invoke<AppConfig>("get_app_config")
        setSidebarOpen(!config.sidebarCollapsed)
      } catch {
        // Use default (open)
      }
    }
    load()
  }, [])

  // Load OCR model version
  useEffect(() => {
    const load = async () => {
      try {
        const model = await invoke<string>("ocr_get_active_model")
        ocrModelVersionRef.current = model
      } catch {
        // ignore
      }
    }
    load()
  }, [])

  // Show green toast notification
  const showGreenToast = useCallback((message: string) => {
    const existing = document.getElementById("lynxocr-green-toast")
    if (existing) existing.remove()

    const toast = document.createElement("div")
    toast.id = "lynxocr-green-toast"
    toast.textContent = message
    Object.assign(toast.style, {
      position: "fixed",
      top: "18%",
      left: "50%",
      transform: "translateX(-50%)",
      background: "#dcfce7",
      color: "#166534",
      padding: "10px 24px",
      borderRadius: "8px",
      fontSize: "14px",
      fontWeight: "500",
      fontFamily: "system-ui, -apple-system, sans-serif",
      boxShadow: "0 4px 16px rgba(0,0,0,0.12)",
      zIndex: "99999",
      pointerEvents: "none",
      transition: "opacity 0.3s ease",
      border: "1px solid #bbf7d0",
    })
    document.body.appendChild(toast)
    toastRef.current = toast

    setTimeout(() => {
      if (toastRef.current) {
        toastRef.current.style.opacity = "0"
        setTimeout(() => {
          if (toastRef.current) {
            toastRef.current.remove()
            toastRef.current = null
          }
        }, 300)
      }
    }, 2000)
  }, [])

  // Trigger screenshot: capture + open transparent fullscreen window
  const triggerScreenshot = useCallback(async () => {
    try {
      // Refresh OCR model version
      try {
        const model = await invoke<string>("ocr_get_active_model")
        ocrModelVersionRef.current = model
      } catch {
        // keep current
      }

      await invoke("start_screenshot_selection", {
        modelVersion: ocrModelVersionRef.current,
      })
    } catch (err) {
      console.error("Screenshot capture failed:", err)
    }
  }, [])

  // Listen for screenshot OCR result events from the main window
  useEffect(() => {
    let unlistenResult: UnlistenFn | undefined
    let unlistenTrigger: UnlistenFn | undefined

    const setup = async () => {
      try {
        // Listen for OCR results from screenshot window
        unlistenResult = await listen<{
          text: string
          timeMs: number
          croppedImagePath?: string
          ocrResult?: {
            textBlocks: Array<{
              text: string
              confidence: number
              boxPoints: unknown
            }>
            totalTimeMs: number
          }
        }>("screenshot-ocr-result", (event) => {
          const { text, timeMs, croppedImagePath, ocrResult } = event.payload
          // Only navigate to OCR page and show result if window is visible
          if (document.visibilityState === "visible") {
            setCurrentPage("ocr")
            window.dispatchEvent(
              new CustomEvent("lynxocr:screenshot-ocr-result", {
                detail: { text, timeMs, croppedImagePath, ocrResult },
              })
            )
          }
          if (text) {
            showGreenToast("文本复制成功")
          }
        })

        // Listen for trigger from Rust-side global shortcut
        unlistenTrigger = await listen("trigger-screenshot-ocr", () => {
          triggerScreenshot()
        })
      } catch {
        // ignore
      }
    }
    setup()

    return () => {
      unlistenResult?.()
      unlistenTrigger?.()
    }
  }, [showGreenToast, triggerScreenshot])

  // Persist sidebar state to config
  const handleSidebarOpenChange = useCallback(async (open: boolean) => {
    setSidebarOpen(open)
    try {
      const config = await invoke<AppConfig>("get_app_config")
      config.sidebarCollapsed = !open
      await invoke("set_app_config", { newConfig: config })
    } catch {
      // Silently ignore
    }
  }, [])

  const renderPage = () => {
    return (
      <Suspense fallback={<PageFallback />}>
        {(() => {
          switch (currentPage) {
            case "ocr":
              return <OCRPage onScreenshotTrigger={triggerScreenshot} />
            case "settings":
              return <SettingsPage />
            case "model-settings":
              return <ModelSettingsPage />
            case "api-settings":
              return <ApiSettingsPage />
            case "about":
              return <AboutPage />
            default:
              return <OCRPage onScreenshotTrigger={triggerScreenshot} />
          }
        })()}
      </Suspense>
    )
  }

  return (
    <AppProvider>
      <TooltipProvider>
        <WindowFrame
          titleBar={<AppTitleBar currentPage={currentPage} />}
          contentClassName="flex flex-1 overflow-hidden"
        >
          <SidebarProvider
            open={sidebarOpen}
            onOpenChange={handleSidebarOpenChange}
            className="max-h-full overflow-hidden"
            style={
              {
                "--sidebar-width": "calc(var(--spacing) * 52)",
              } as React.CSSProperties
            }
          >
            <AppSidebar
              variant="inset"
              currentPage={currentPage}
              onNavigate={setCurrentPage}
            />
            <SidebarInset className="overflow-hidden">
              <div className="flex items-center gap-1 px-4 py-1">
                <SidebarTrigger className="-ml-1" />
                <Separator
                  orientation="vertical"
                  className="mx-2 data-[orientation=vertical]:h-4"
                />
              </div>
              <div className="flex flex-1 flex-col overflow-hidden">
                <div className="@container/main flex flex-1 flex-col gap-2 overflow-hidden">
                  <div className="flex flex-col gap-4 overflow-y-auto py-4 md:gap-6 md:py-6">
                    {renderPage()}
                  </div>
                </div>
              </div>
            </SidebarInset>
          </SidebarProvider>
        </WindowFrame>
      </TooltipProvider>
    </AppProvider>
  )
}
