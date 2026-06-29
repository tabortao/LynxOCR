import { useState, useEffect, useRef, useCallback } from "react"
import ReactMarkdown from "react-markdown"
import remarkGfm from "remark-gfm"
import remarkMath from "remark-math"
import rehypeKatex from "rehype-katex"
import "katex/dist/katex.min.css"
import { Button } from "@/components/ui/button"
import { Card, CardContent } from "@/components/ui/card"
import { Separator } from "@/components/ui/separator"
import { Badge } from "@/components/ui/badge"
import { Progress } from "@/components/ui/progress"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import {
  UploadIcon,
  ImageIcon,
  CopyIcon,
  Trash2Icon,
  FileTextIcon,
  ScanTextIcon,
  AlertTriangleIcon,
  CameraIcon,
  CheckCircleIcon,
  XCircleIcon,
  ChevronDownIcon,
  ChevronRightIcon,
  EyeIcon,
  AlignLeftIcon,
  CloudIcon,
} from "lucide-react"
import { invoke, convertFileSrc } from "@tauri-apps/api/core"
import { open } from "@tauri-apps/plugin-dialog"
import { listen, type UnlistenFn } from "@tauri-apps/api/event"
import { useAppContext } from "@/lib/app-context"
import type { OcrResult, ModelInfo, AppConfig } from "@/types"

type OCRState = "idle" | "loading" | "completed" | "error"

const MODEL_NAMES = ["ppocr-v4", "ppocr-v5", "ppocr-v6", "mineru"] as const
const MODEL_DISPLAY: Record<string, string> = {
  "ppocr-v4": "PaddleOCR V4",
  "ppocr-v5": "PaddleOCR V5",
  "ppocr-v6": "PaddleOCR V6",
  mineru: "MinerU Cloud",
}

const MINERU_FORMATS = [
  { value: "md", label: "Markdown (.md)" },
  { value: "html", label: "HTML (.html)" },
  { value: "latex", label: "LaTeX (.tex)" },
  { value: "docx", label: "DOCX (.docx)" },
  { value: "json", label: "JSON (.json)" },
]

const IMAGE_EXTS = ["png", "jpg", "jpeg", "bmp", "webp", "tiff", "tif"]
const PDF_EXTS = ["pdf"]
const ALL_OCR_EXTS = [...IMAGE_EXTS, ...PDF_EXTS]

interface BatchOcrItem {
  path: string
  fileName: string
  imageUrl: string
  state: OCRState
  result: OcrResult | null
  error: string | null
}

interface OCRPageProps {
  onScreenshotTrigger?: () => void
}

export function OCRPage({ onScreenshotTrigger }: OCRPageProps) {
  const { t } = useAppContext()
  const [items, setItems] = useState<BatchOcrItem[]>([])
  const [isDragOver, setIsDragOver] = useState(false)
  const [activeModel, setActiveModel] = useState("ppocr-v6")
  const [installedModels, setInstalledModels] = useState<Set<string>>(new Set())
  const [config, setConfig] = useState<AppConfig | null>(null)
  const [mineruOutputFormat, setMineruOutputFormat] = useState("md")
  const [flashMessage, setFlashMessage] = useState("")
  const [expandedIdx, setExpandedIdx] = useState<number>(-1)
  const [batchProcessing, setBatchProcessing] = useState(false)
  const [batchProgress, setBatchProgress] = useState({ current: 0, total: 0 })
  const [viewModes, setViewModes] = useState<
    Record<number, "plain" | "markdown">
  >({})
  const flashTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const modelInstalledRef = useRef(false)
  const startBatchOCRForPathsRef = useRef<(paths: string[]) => Promise<void>>(
    undefined as unknown as (paths: string[]) => Promise<void>
  )

  const isMineru = activeModel === "mineru"
  const hasMineruToken = (config?.mineruApiToken?.length ?? 0) > 0
  const modelInstalled = isMineru || installedModels.has(activeModel)

  // Keep ref in sync
  useEffect(() => {
    modelInstalledRef.current = modelInstalled
  }, [modelInstalled])

  const showFlash = useCallback((msg: string) => {
    setFlashMessage(msg)
    if (flashTimerRef.current) clearTimeout(flashTimerRef.current)
    flashTimerRef.current = setTimeout(() => setFlashMessage(""), 2000)
  }, [])

  useEffect(() => {
    return () => {
      if (flashTimerRef.current) clearTimeout(flashTimerRef.current)
    }
  }, [])

  // Load active OCR model and check which models are installed
  useEffect(() => {
    const load = async () => {
      try {
        const active = await invoke<string>("ocr_get_active_model")
        setActiveModel(active)

        const cfg = await invoke<AppConfig>("get_app_config")
        setConfig(cfg)
        setMineruOutputFormat(cfg.mineruOutputFormat || "md")

        const models = await invoke<ModelInfo[]>("list_models")
        const installed = new Set<string>()
        for (const m of models) {
          if (
            MODEL_NAMES.includes(m.name as (typeof MODEL_NAMES)[number]) &&
            m.installed
          ) {
            installed.add(m.name)
          }
        }
        setInstalledModels(installed)
      } catch {
        // ignore
      }
    }
    load()

    // Release OCR engine when leaving the OCR page to free ONNX Runtime memory
    return () => {
      invoke("ocr_release").catch(() => {
        // ignore (might already be released)
      })
    }
  }, [])

  // Listen for screenshot OCR results
  useEffect(() => {
    const handler = (e: Event) => {
      const { text, timeMs, croppedImagePath, ocrResult } = (e as CustomEvent)
        .detail
      if (text) {
        const screenshotItem: BatchOcrItem = {
          path: "",
          fileName: t("ocr.screenshotBtn"),
          imageUrl: croppedImagePath ? convertFileSrc(croppedImagePath) : "",
          state: "completed",
          result: ocrResult
            ? {
                textBlocks: ocrResult.textBlocks,
                totalTimeMs: ocrResult.totalTimeMs,
                format: ocrResult.format,
              }
            : {
                textBlocks: [{ text, confidence: 1.0, boxPoints: [] }],
                totalTimeMs: timeMs,
              },
          error: null,
        }
        setItems([screenshotItem])
        setExpandedIdx(0)
        showFlash(
          t("ocr.completedToast", {
            blocks: screenshotItem.result?.textBlocks.length ?? 1,
            time: (timeMs / 1000).toFixed(1),
          })
        )
      }
    }
    window.addEventListener("lynxocr:screenshot-ocr-result", handler)
    return () =>
      window.removeEventListener("lynxocr:screenshot-ocr-result", handler)
  }, [showFlash, t])

  // Handle file drop events
  useEffect(() => {
    let unlistenDrop: UnlistenFn | undefined
    let unlistenHover: UnlistenFn | undefined
    let unlistenHoverLeave: UnlistenFn | undefined

    const setup = async () => {
      try {
        unlistenDrop = await listen<string>("tauri://file-drop", (event) => {
          try {
            const paths: string[] = JSON.parse(event.payload)
            const imagePaths = paths.filter((p) => {
              const ext = p.split(".").pop()?.toLowerCase() ?? ""
              return IMAGE_EXTS.includes(ext)
            })
            const pdfPaths = paths.filter((p) => {
              const ext = p.split(".").pop()?.toLowerCase() ?? ""
              return PDF_EXTS.includes(ext)
            })
            if (imagePaths.length > 0) {
              addFiles(imagePaths)
            }
            if (pdfPaths.length > 0) {
              addPdfFiles(pdfPaths)
            }
          } catch {
            // ignore
          }
        })
        unlistenHover = await listen<boolean>("tauri://file-drop-hover", () => {
          setIsDragOver(true)
        })
        unlistenHoverLeave = await listen<boolean>(
          "tauri://file-drop-hover",
          (event) => {
            if (!event.payload) setIsDragOver(false)
          }
        )
      } catch {
        // ignore
      }
    }
    setup()

    return () => {
      unlistenDrop?.()
      unlistenHover?.()
      unlistenHoverLeave?.()
    }
  }, [])

  const addFiles = useCallback((paths: string[]) => {
    const newItems: BatchOcrItem[] = paths.map((p) => ({
      path: p,
      fileName: p.split(/[/\\]/).pop() || p,
      imageUrl: convertFileSrc(p),
      state: "loading" as OCRState,
      result: null,
      error: null,
    }))
    setItems((prev) => [...prev, ...newItems])
    setExpandedIdx(newItems.length === 1 ? 0 : -1)
    // Auto-start OCR for new items using ref to avoid stale closure
    if (modelInstalledRef.current && startBatchOCRForPathsRef.current) {
      startBatchOCRForPathsRef.current(newItems.map((item) => item.path))
    }
  }, [])

  const handleOpenFile = async () => {
    try {
      const selected = await open({
        multiple: true,
        filters: [
          {
            name: "Images & PDF",
            extensions: ALL_OCR_EXTS,
          },
        ],
      })
      if (selected) {
        const paths = Array.isArray(selected) ? selected : [selected]
        const imagePaths = paths.filter((p) => {
          const ext = p.split(".").pop()?.toLowerCase() ?? ""
          return IMAGE_EXTS.includes(ext)
        })
        const pdfPaths = paths.filter((p) => {
          const ext = p.split(".").pop()?.toLowerCase() ?? ""
          return PDF_EXTS.includes(ext)
        })
        if (imagePaths.length > 0) addFiles(imagePaths)
        if (pdfPaths.length > 0) addPdfFiles(pdfPaths)
      }
    } catch (err) {
      console.error("Failed to open file:", err)
    }
  }

  const addPdfFiles = useCallback(
    (paths: string[]) => {
      if (!modelInstalledRef.current || !startBatchOCRForPathsRef.current)
        return

      for (const pdfPath of paths) {
        const fileName = pdfPath.split(/[/\\]/).pop() || pdfPath
        // Create a placeholder item for the PDF
        const pdfItem: BatchOcrItem = {
          path: pdfPath,
          fileName: `${fileName} (PDF)`,
          imageUrl: "",
          state: "loading" as OCRState,
          result: null,
          error: null,
        }
        setItems((prev) => [...prev, pdfItem])

        // Process PDF asynchronously
        const processPdf = async () => {
          try {
            const results = await invoke<
              Array<{
                pageIndex: number
                imagePath: string
                ocrResult: OcrResult
              }>
            >("ocr_recognize_pdf", {
              pdfPath,
              modelVersion: activeModel,
              dpi: 200,
            })

            if (results.length === 0) {
              setItems((prev) =>
                prev.map((it) =>
                  it.path === pdfPath
                    ? {
                        ...it,
                        state: "completed" as OCRState,
                        result: { textBlocks: [], totalTimeMs: 0 },
                      }
                    : it
                )
              )
              return
            }

            // Remove the placeholder and add individual page items
            setItems((prev) => {
              const withoutPlaceholder = prev.filter(
                (it) => it.path !== pdfPath
              )
              const pageItems: BatchOcrItem[] = results.map((r) => ({
                path: `${pdfPath}#page=${r.pageIndex}`,
                fileName: `${fileName} - ${t("ocr.pdfPage", { page: r.pageIndex + 1 })}`,
                imageUrl: convertFileSrc(r.imagePath),
                state: "completed" as OCRState,
                result: r.ocrResult,
                error: null,
              }))
              return [...withoutPlaceholder, ...pageItems]
            })
          } catch (err) {
            setItems((prev) =>
              prev.map((it) =>
                it.path === pdfPath
                  ? { ...it, state: "error" as OCRState, error: String(err) }
                  : it
              )
            )
          }
        }
        processPdf()
      }
    },
    [activeModel, t]
  )

  const startBatchOCRForPaths = useCallback(
    async (paths: string[]) => {
      if (!modelInstalled || paths.length === 0) return

      setBatchProcessing(true)
      setBatchProgress({ current: 0, total: paths.length })

      let succeeded = 0
      let failed = 0
      let totalTimeMs = 0

      for (let i = 0; i < paths.length; i++) {
        const itemPath = paths[i]
        setBatchProgress({ current: i + 1, total: paths.length })

        // Mark item as loading
        setItems((prev) =>
          prev.map((it) =>
            it.path === itemPath
              ? { ...it, state: "loading" as OCRState, error: null }
              : it
          )
        )

        try {
          const res = await invoke<OcrResult>("ocr_recognize", {
            imagePath: itemPath,
            modelVersion: activeModel,
          })
          setItems((prev) =>
            prev.map((it) =>
              it.path === itemPath
                ? { ...it, state: "completed" as OCRState, result: res }
                : it
            )
          )
          succeeded++
          totalTimeMs += res.totalTimeMs
        } catch (err) {
          setItems((prev) =>
            prev.map((it) =>
              it.path === itemPath
                ? { ...it, state: "error" as OCRState, error: String(err) }
                : it
            )
          )
          failed++
        }
      }

      setBatchProcessing(false)
      showFlash(
        t("ocr.batchDone", {
          count: succeeded,
          time: (totalTimeMs / 1000).toFixed(1),
        })
      )
      // Release engine after batch completes to free memory
      invoke("ocr_release").catch(() => {})
    },
    [modelInstalled, activeModel, showFlash, t]
  )

  // Keep ref in sync so addFiles can call it without stale closure
  useEffect(() => {
    startBatchOCRForPathsRef.current = startBatchOCRForPaths
  }, [startBatchOCRForPaths])

  const startBatchOCR = async () => {
    const pendingItems = items.filter(
      (item) => item.state === "idle" || item.state === "error"
    )
    if (pendingItems.length === 0) return
    await startBatchOCRForPaths(pendingItems.map((item) => item.path))
  }

  const handleModelChange = async (model: string) => {
    setActiveModel(model)
    try {
      await invoke("ocr_set_active_model", { modelName: model })

      const cfg = await invoke<AppConfig>("get_app_config")
      setConfig(cfg)
      setMineruOutputFormat(cfg.mineruOutputFormat || "md")

      const models = await invoke<ModelInfo[]>("list_models")
      const installed = new Set<string>()
      for (const m of models) {
        if (
          MODEL_NAMES.includes(m.name as (typeof MODEL_NAMES)[number]) &&
          m.installed
        ) {
          installed.add(m.name)
        }
      }
      setInstalledModels(installed)
    } catch {
      // ignore
    }
  }

  const handleClear = () => {
    setItems([])
    setExpandedIdx(-1)
    // Release engine after clearing results
    invoke("ocr_release").catch(() => {})
  }

  const handleRemoveItem = (idx: number) => {
    setItems((prev) => prev.filter((_, i) => i !== idx))
    if (expandedIdx === idx) setExpandedIdx(-1)
    else if (expandedIdx > idx) setExpandedIdx(expandedIdx - 1)
  }

  const handleCopyAllText = async () => {
    const allText = items
      .filter((item) => item.state === "completed" && item.result)
      .map((item) => item.result!.textBlocks.map((b) => b.text).join("\n"))
      .join("\n\n")
    if (!allText) return
    try {
      await invoke("copy_text_to_clipboard", { text: allText })
      showFlash(t("ocr.copyText"))
    } catch {
      await navigator.clipboard.writeText(allText)
      showFlash(t("ocr.copyText"))
    }
  }

  const handleCopyItemText = async (item: BatchOcrItem) => {
    if (!item.result) return
    const text = item.result.textBlocks.map((b) => b.text).join("\n")
    try {
      await invoke("copy_text_to_clipboard", { text })
      showFlash(t("ocr.copyText"))
    } catch {
      await navigator.clipboard.writeText(text)
      showFlash(t("ocr.copyText"))
    }
  }

  const handleExportAllTxt = async () => {
    const completedItems = items.filter(
      (item) => item.state === "completed" && item.result
    )
    if (completedItems.length === 0) return

    // Combine all results into a single text
    const allText = completedItems
      .map((item) => item.result!.textBlocks.map((b) => b.text).join("\n"))
      .join("\n\n---\n\n")

    if (!allText) return

    // Determine export path from the first item's path
    // Strip #page=N suffix (for PDF items) and replace extension
    const firstPath = completedItems[0].path.replace(/#page=\d+$/i, "")
    const baseName = firstPath.replace(/\.[^.]+$/, "")
    const exportPath = `${baseName}_ocr.txt`

    try {
      await invoke("write_text_file", { path: exportPath, content: allText })
      await invoke("open_file_with_system", { path: exportPath })
      showFlash(t("ocr.exportTxt"))
    } catch (err) {
      console.error("Export failed:", err)
    }
  }

  const handleExportMd = async () => {
    const completedItems = items.filter(
      (item) => item.state === "completed" && item.result
    )
    if (completedItems.length === 0) return

    const now = new Date().toISOString().replace("T", " ").slice(0, 19)
    const modelDisplay = MODEL_DISPLAY[activeModel] ?? activeModel

    // Build markdown content
    const parts: string[] = []
    parts.push("# OCR 识别结果")
    parts.push("")
    parts.push(`> 识别时间: ${now}`)
    parts.push(`> 识别模型: ${modelDisplay}`)
    parts.push("")

    for (const item of completedItems) {
      const pageMatch = item.path.match(/#page=(\d+)$/i)
      if (pageMatch) {
        parts.push(`## ${item.fileName} (第 ${pageMatch[1]} 页)`)
      } else {
        parts.push(`## ${item.fileName}`)
      }
      parts.push("")
      for (const block of item.result!.textBlocks) {
        parts.push(block.text)
        parts.push("")
      }
      parts.push("---")
      parts.push("")
    }

    const mdContent = parts.join("\n")
    if (!mdContent) return

    const firstPath = completedItems[0].path.replace(/#page=\d+$/i, "")
    const baseName = firstPath.replace(/\.[^.]+$/, "")
    const exportPath = `${baseName}_ocr.md`

    try {
      await invoke("write_text_file", { path: exportPath, content: mdContent })
      await invoke("open_file_with_system", { path: exportPath })
      showFlash(t("ocr.exportMd"))
    } catch (err) {
      console.error("Export MD failed:", err)
    }
  }

  const handleExportMineruFormat = async (format: string) => {
    const completedItems = items.filter(
      (item) => item.state === "completed" && item.result
    )
    if (completedItems.length === 0) return

    const content = completedItems
      .map((item) => item.result!.textBlocks.map((b) => b.text).join("\n"))
      .join("\n\n")

    if (!content) return

    const firstPath = completedItems[0].path.replace(/#page=\d+$/i, "")
    const baseName = firstPath.replace(/\.[^.]+$/, "")
    const extMap: Record<string, string> = {
      html: "html",
      latex: "tex",
      md: "md",
      json: "json",
    }
    const ext = extMap[format] || format
    const exportPath = `${baseName}_ocr.${ext}`

    try {
      if (format === "docx") {
        // DOCX is base64 encoded in the textBlocks
        const base64Data = completedItems[0].result!.textBlocks[0]?.text || ""
        await invoke("write_binary_file", {
          path: exportPath,
          base64Content: base64Data,
        })
      } else {
        await invoke("write_text_file", { path: exportPath, content })
        await invoke("open_file_with_system", { path: exportPath })
      }
      showFlash(format.toUpperCase())
    } catch (err) {
      console.error(`Export ${format} failed:`, err)
    }
  }

  const handleScreenshotOCR = async () => {
    if (!modelInstalled) {
      return
    }
    onScreenshotTrigger?.()
  }

  const hasItems = items.length > 0
  const hasCompleted = items.some((item) => item.state === "completed")
  const hasPending = items.some(
    (item) => item.state === "idle" || item.state === "error"
  )

  return (
    <div className="space-y-4 px-4 lg:px-6">
      {/* Toolbar */}
      <div className="flex flex-wrap items-center gap-2">
        <select
          value={activeModel}
          onChange={(e) => handleModelChange(e.target.value)}
          className="h-9 rounded-md border border-input bg-background px-3 py-1 text-sm"
        >
          {MODEL_NAMES.map((key) => (
            <option key={key} value={key}>
              {MODEL_DISPLAY[key]}
              {installedModels.has(key) || key === "mineru"
                ? ""
                : " (not installed)"}
            </option>
          ))}
        </select>
        {isMineru && (
          <select
            value={mineruOutputFormat}
            onChange={(e) => setMineruOutputFormat(e.target.value)}
            className="h-9 rounded-md border border-input bg-background px-3 py-1 text-sm"
            title={t("ocr.mineruFormat")}
          >
            {MINERU_FORMATS.map((f) => (
              <option
                key={f.value}
                value={f.value}
                disabled={!hasMineruToken && f.value !== "md"}
              >
                {t("ocr.mineruFormat")}: {f.label}
              </option>
            ))}
          </select>
        )}
        {isMineru && !hasMineruToken && (
          <Badge variant="secondary" className="h-9 px-3">
            {t("ocr.mineruFlashMode")}
          </Badge>
        )}
        <Button
          onClick={handleScreenshotOCR}
          variant="default"
          size="sm"
          title={t("ocr.screenshotDesc")}
        >
          <CameraIcon className="mr-1 size-4" />
          {t("ocr.screenshotBtn")}
        </Button>
        <Button onClick={handleOpenFile} variant="outline" size="sm">
          <UploadIcon className="mr-1 size-4" />
          {t("ocr.selectImage")}
        </Button>
        {hasPending && !batchProcessing && (
          <Button
            onClick={startBatchOCR}
            variant="outline"
            size="sm"
            title={t("ocr.retryFailed")}
          >
            <ScanTextIcon className="mr-1 size-4" />
            {t("ocr.retryFailed")}
          </Button>
        )}
        {batchProcessing && (
          <Badge variant="secondary" className="h-9 px-3 tabular-nums">
            {Math.round(
              (batchProgress.current / Math.max(batchProgress.total, 1)) * 100
            )}
            % ({batchProgress.current}/{batchProgress.total})
          </Badge>
        )}
        {hasCompleted && (
          <Button onClick={handleCopyAllText} variant="outline" size="sm">
            <CopyIcon className="mr-1 size-4" />
            {t("ocr.copyAllText")}
          </Button>
        )}
        {hasCompleted && (
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="outline" size="sm">
                <FileTextIcon className="mr-1 size-4" />
                {t("ocr.exportAs")}
                <ChevronDownIcon className="ml-1 size-3" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem onClick={handleExportAllTxt}>
                <FileTextIcon className="mr-2 size-4" />
                {t("ocr.exportTxt")} (.txt)
              </DropdownMenuItem>
              <DropdownMenuItem onClick={handleExportMd}>
                <FileTextIcon className="mr-2 size-4" />
                {t("ocr.exportMd")} (.md)
              </DropdownMenuItem>
              {isMineru && (
                <>
                  <DropdownMenuItem
                    onClick={() => handleExportMineruFormat("html")}
                  >
                    <FileTextIcon className="mr-2 size-4" />
                    {t("ocr.exportHtml")} (.html)
                  </DropdownMenuItem>
                  <DropdownMenuItem
                    onClick={() => handleExportMineruFormat("latex")}
                  >
                    <FileTextIcon className="mr-2 size-4" />
                    {t("ocr.exportLatex")} (.tex)
                  </DropdownMenuItem>
                  <DropdownMenuItem
                    onClick={() => handleExportMineruFormat("docx")}
                  >
                    <FileTextIcon className="mr-2 size-4" />
                    {t("ocr.exportDocx")} (.docx)
                  </DropdownMenuItem>
                  <DropdownMenuItem
                    onClick={() => handleExportMineruFormat("json")}
                  >
                    <FileTextIcon className="mr-2 size-4" />
                    {t("ocr.exportJson")} (.json)
                  </DropdownMenuItem>
                </>
              )}
            </DropdownMenuContent>
          </DropdownMenu>
        )}
        <Button
          onClick={handleClear}
          variant="ghost"
          size="sm"
          disabled={!hasItems}
        >
          <Trash2Icon className="mr-1 size-4" />
          {t("ocr.clear")}
        </Button>
        {flashMessage && (
          <Badge variant="secondary" className="ml-2">
            {flashMessage}
          </Badge>
        )}
      </div>

      {/* Model not installed warning */}
      {!modelInstalled && !isMineru && (
        <div className="flex items-start gap-2 rounded-lg border border-amber-200 bg-amber-50 p-3 dark:border-amber-800 dark:bg-amber-950">
          <AlertTriangleIcon className="mt-0.5 size-4 shrink-0 text-amber-600 dark:text-amber-400" />
          <p className="text-sm text-amber-700 dark:text-amber-300">
            {t("ocr.modelNotInstalled", {
              model: MODEL_DISPLAY[activeModel] ?? activeModel,
            })}
          </p>
        </div>
      )}

      {/* MinerU no-token notice */}
      {isMineru && !hasMineruToken && (
        <div className="flex items-start gap-2 rounded-lg border border-blue-200 bg-blue-50 p-3 dark:border-blue-800 dark:bg-blue-950">
          <CloudIcon className="mt-0.5 size-4 shrink-0 text-blue-600 dark:text-blue-400" />
          <p className="text-sm text-blue-700 dark:text-blue-300">
            {t("ocr.mineruNoToken")}
          </p>
        </div>
      )}

      {!hasItems ? (
        /* Idle state: drag-and-drop zone */
        <Card
          className={`border-2 border-dashed transition-colors ${
            isDragOver
              ? "border-primary bg-primary/5"
              : "border-muted-foreground/25"
          }`}
        >
          <CardContent className="flex flex-col items-center justify-center gap-4 py-16">
            <ImageIcon className="size-16 text-muted-foreground" />
            <div className="space-y-2 text-center">
              <p className="text-lg font-medium">{t("ocr.dropImages")}</p>
              <p className="text-sm text-muted-foreground">
                {t("ocr.clickOrDrag")}
              </p>
              <p className="text-xs text-muted-foreground">{t("ocr.desc")}</p>
            </div>
            <Button onClick={handleOpenFile} variant="default">
              <UploadIcon className="mr-1 size-4" />
              {t("ocr.selectImage")}
            </Button>
          </CardContent>
        </Card>
      ) : (
        /* Batch items list */
        <div className="space-y-2">
          {items.map((item, idx) => {
            const totalInBatch =
              batchProgress.total ||
              items.filter(
                (i) =>
                  i.state === "loading" ||
                  i.state === "completed" ||
                  i.state === "error"
              ).length
            const doneCount = items.filter(
              (i) => i.state === "completed" || i.state === "error"
            ).length
            let progressPct = 0
            if (item.state === "completed" || item.state === "error") {
              progressPct = 100
            } else if (item.state === "loading") {
              progressPct =
                totalInBatch > 0
                  ? Math.round(((doneCount + 0.5) / totalInBatch) * 100)
                  : 0
            } else {
              progressPct =
                totalInBatch > 0
                  ? Math.round((doneCount / totalInBatch) * 100)
                  : 0
            }

            return (
              <Card
                key={idx}
                className={expandedIdx === idx ? "ring-1 ring-primary/30" : ""}
              >
                {/* Item header - always visible */}
                <div
                  className="flex cursor-pointer items-center gap-3 p-3 transition-colors hover:bg-accent/30"
                  onClick={() => setExpandedIdx(expandedIdx === idx ? -1 : idx)}
                >
                  {expandedIdx === idx ? (
                    <ChevronDownIcon className="size-4 shrink-0 text-muted-foreground" />
                  ) : (
                    <ChevronRightIcon className="size-4 shrink-0 text-muted-foreground" />
                  )}

                  {/* Thumbnail */}
                  {item.imageUrl && (
                    <div className="size-10 shrink-0 overflow-hidden rounded border bg-muted">
                      <img
                        src={item.imageUrl}
                        alt=""
                        className="h-full w-full object-cover"
                      />
                    </div>
                  )}

                  <div className="min-w-0 flex-1">
                    <p className="truncate text-sm font-medium">
                      {item.fileName}
                    </p>
                    <div className="mt-0.5 flex items-center gap-2">
                      {item.state === "loading" && (
                        <div className="w-full space-y-1">
                          <div className="flex items-center justify-between">
                            <div className="flex items-center gap-1.5">
                              <div className="h-3 w-3 shrink-0 animate-spin rounded-full border-b-2 border-primary" />
                              <span className="text-xs text-muted-foreground">
                                {t("ocr.processing")}
                              </span>
                            </div>
                            <span className="text-xs text-muted-foreground tabular-nums">
                              {progressPct}% · {doneCount + 1}/{totalInBatch}
                            </span>
                          </div>
                          <Progress value={progressPct} className="h-1.5" />
                        </div>
                      )}
                      {item.state === "completed" && item.result && (
                        <div className="flex items-center gap-1.5">
                          <CheckCircleIcon className="size-3 text-green-600 dark:text-green-400" />
                          <span className="text-xs text-muted-foreground">
                            {t("ocr.textBlocks", {
                              count: item.result.textBlocks.length,
                            })}
                            {" · "}
                            {(item.result.totalTimeMs / 1000).toFixed(1)}s
                          </span>
                        </div>
                      )}
                      {item.state === "error" && (
                        <div className="flex items-center gap-1.5">
                          <XCircleIcon className="size-3 text-red-500" />
                          <span className="truncate text-xs text-red-500">
                            {item.error}
                          </span>
                        </div>
                      )}
                      {item.state === "idle" && (
                        <span className="text-xs text-muted-foreground">
                          {t("ocr.pending")}
                        </span>
                      )}
                    </div>
                  </div>

                  {/* Actions */}
                  <div className="flex shrink-0 items-center gap-1">
                    {item.state === "completed" && (
                      <Button
                        variant="ghost"
                        size="icon"
                        className="size-7"
                        onClick={(e) => {
                          e.stopPropagation()
                          handleCopyItemText(item)
                        }}
                        title={t("ocr.copyText")}
                      >
                        <CopyIcon className="size-3.5" />
                      </Button>
                    )}
                    {!batchProcessing && (
                      <Button
                        variant="ghost"
                        size="icon"
                        className="size-7"
                        onClick={(e) => {
                          e.stopPropagation()
                          handleRemoveItem(idx)
                        }}
                        title={t("ocr.clear")}
                      >
                        <XCircleIcon className="size-3.5" />
                      </Button>
                    )}
                  </div>
                </div>

                {/* Expanded detail */}
                {expandedIdx === idx && (
                  <CardContent className="pt-0 pb-3">
                    <Separator className="mb-3" />
                    <div className="grid grid-cols-1 gap-3 lg:grid-cols-2">
                      {/* Image preview */}
                      {item.imageUrl && (
                        <div className="relative overflow-hidden rounded-lg bg-muted">
                          <img
                            src={item.imageUrl}
                            alt="Preview"
                            className="mx-auto max-h-[300px] max-w-full object-contain"
                          />
                        </div>
                      )}

                      {/* OCR result */}
                      <div>
                        {item.state === "loading" && (
                          <div className="flex flex-col items-center justify-center gap-2 py-8">
                            <Progress
                              value={progressPct}
                              className="h-2 w-48"
                            />
                            <div className="text-xs text-muted-foreground tabular-nums">
                              {progressPct}% · {doneCount + 1}/{totalInBatch}
                            </div>
                          </div>
                        )}

                        {item.state === "completed" && item.result && (
                          <div className="space-y-2">
                            {item.result.textBlocks.length === 0 ? (
                              <p className="py-4 text-center text-sm text-muted-foreground">
                                {t("ocr.noTextFound")}
                              </p>
                            ) : (
                              <>
                                {/* MinerU HTML preview */}
                                {item.result.format === "html" && (
                                  <div className="max-h-[300px] overflow-y-auto rounded-lg border bg-white dark:bg-gray-900">
                                    <iframe
                                      srcDoc={
                                        item.result.textBlocks[0]?.text || ""
                                      }
                                      className="h-[300px] w-full border-0"
                                      title="HTML Preview"
                                      sandbox="allow-scripts"
                                    />
                                  </div>
                                )}

                                {/* MinerU LaTeX view */}
                                {item.result.format === "latex" && (
                                  <div className="max-h-[250px] overflow-y-auto rounded-lg border bg-muted/30 p-3">
                                    <pre className="font-mono text-xs break-all whitespace-pre-wrap">
                                      {item.result.textBlocks[0]?.text}
                                    </pre>
                                  </div>
                                )}

                                {/* MinerU DOCX notice */}
                                {item.result.format === "docx" && (
                                  <div className="rounded-lg border bg-muted/30 p-3 text-center">
                                    <p className="text-sm text-muted-foreground">
                                      {t("ocr.exportDocx")} —{" "}
                                      {t("ocr.mineruFormat")}
                                    </p>
                                    <p className="mt-1 text-xs text-muted-foreground">
                                      DOCX is a binary format. Save to file to
                                      view.
                                    </p>
                                  </div>
                                )}

                                {/* MinerU JSON view */}
                                {item.result.format === "json" && (
                                  <div className="max-h-[250px] overflow-y-auto rounded-lg border bg-muted/30 p-3">
                                    <pre className="font-mono text-xs break-all whitespace-pre-wrap">
                                      {(() => {
                                        try {
                                          return JSON.stringify(
                                            JSON.parse(
                                              item.result.textBlocks[0]?.text ||
                                                ""
                                            ),
                                            null,
                                            2
                                          )
                                        } catch {
                                          return item.result.textBlocks[0]?.text
                                        }
                                      })()}
                                    </pre>
                                  </div>
                                )}

                                {/* Default: plain text/markdown view (PaddleOCR or MinerU md) */}
                                {(!item.result.format ||
                                  item.result.format === "md") && (
                                  <>
                                    {item.result.format === "md" ? (
                                      // MinerU markdown: always render preview with math + table support
                                      <div className="max-h-[350px] overflow-y-auto rounded-lg border bg-background p-3">
                                        <div className="
                                          text-sm leading-relaxed
                                          [&_h1]:mb-2 [&_h1]:mt-4 [&_h1]:text-lg [&_h1]:font-bold
                                          [&_h2]:mb-1.5 [&_h2]:mt-3 [&_h2]:text-base [&_h2]:font-semibold
                                          [&_h3]:mb-1 [&_h3]:mt-2 [&_h3]:text-sm [&_h3]:font-medium
                                          [&_p]:my-1.5 [&_p]:leading-relaxed
                                          [&_pre]:my-2 [&_pre]:overflow-x-auto [&_pre]:rounded [&_pre]:bg-muted [&_pre]:p-2 [&_pre]:text-xs
                                          [&_code]:rounded [&_code]:bg-muted/50 [&_code]:px-1 [&_code]:text-xs [&_code]:font-mono
                                          [&_pre_code]:bg-transparent [&_pre_code]:p-0
                                          [&_table]:my-2 [&_table]:w-full [&_table]:border-collapse [&_table]:text-xs
                                          [&_thead]:border-b [&_thead]:border-border
                                          [&_th]:border [&_th]:border-border [&_th]:bg-muted/50 [&_th]:p-2 [&_th]:text-left [&_th]:font-medium
                                          [&_td]:border [&_td]:border-border [&_td]:p-2
                                          [&_tr]:border-b [&_tr]:border-border
                                          [&_blockquote]:my-2 [&_blockquote]:border-l-2 [&_blockquote]:border-muted-foreground/30 [&_blockquote]:pl-3 [&_blockquote]:text-muted-foreground [&_blockquote]:text-xs
                                          [&_ul]:my-1.5 [&_ul]:list-disc [&_ul]:pl-5
                                          [&_ol]:my-1.5 [&_ol]:list-decimal [&_ol]:pl-5
                                          [&_li]:my-0.5
                                          [&_a]:text-primary [&_a]:underline
                                          [&_hr]:my-3 [&_hr]:border-border
                                          [&_img]:max-w-full [&_img]:rounded
                                          ui-selectable
                                        ">
                                          <ReactMarkdown
                                            remarkPlugins={[
                                              remarkGfm,
                                              remarkMath,
                                            ]}
                                            rehypePlugins={[rehypeKatex]}
                                          >
                                            {item.result.textBlocks
                                              .map((b) => b.text)
                                              .join("\n\n")}
                                          </ReactMarkdown>
                                        </div>
                                      </div>
                                    ) : (
                                      <>
                                        {/* View mode toggle */}
                                        <div className="flex items-center gap-1">
                                          <Button
                                            variant={
                                              viewModes[idx] === "markdown"
                                                ? "ghost"
                                                : "secondary"
                                            }
                                            size="sm"
                                            className="h-7 text-xs"
                                            onClick={() =>
                                              setViewModes((prev) => ({
                                                ...prev,
                                                [idx]: "plain",
                                              }))
                                            }
                                          >
                                            <AlignLeftIcon className="mr-1 size-3.5" />
                                            {t("ocr.viewPlain")}
                                          </Button>
                                          <Button
                                            variant={
                                              viewModes[idx] === "markdown"
                                                ? "secondary"
                                                : "ghost"
                                            }
                                            size="sm"
                                            className="h-7 text-xs"
                                            onClick={() =>
                                              setViewModes((prev) => ({
                                                ...prev,
                                                [idx]: "markdown",
                                              }))
                                            }
                                          >
                                            <EyeIcon className="mr-1 size-3.5" />
                                            {t("ocr.viewMarkdown")}
                                          </Button>
                                        </div>

                                        {/* Plain text view */}
                                        {viewModes[idx] !== "markdown" && (
                                          <div className="max-h-[250px] space-y-1.5 overflow-y-auto">
                                            {item.result.textBlocks.map(
                                              (block, bIdx) => (
                                                <div
                                                  key={bIdx}
                                                  className="rounded-lg border bg-muted/30 p-2 transition-colors hover:bg-muted/50"
                                                >
                                                  <div className="flex items-start justify-between gap-2">
                                                    <p className="text-sm leading-relaxed break-all">
                                                      {block.text}
                                                    </p>
                                                    <Badge
                                                      variant="secondary"
                                                      className="shrink-0 text-xs"
                                                    >
                                                      {(
                                                        block.confidence * 100
                                                      ).toFixed(1)}
                                                      %
                                                    </Badge>
                                                  </div>
                                                </div>
                                              )
                                            )}
                                          </div>
                                        )}

                                        {/* Markdown preview */}
                                        {viewModes[idx] === "markdown" && (
                                          <div className="max-h-[250px] overflow-y-auto rounded-lg border bg-background p-3">
                                            <div className="text-sm leading-relaxed [&_h1]:mb-2 [&_h1]:mt-4 [&_h1]:text-lg [&_h1]:font-bold [&_h2]:mb-1.5 [&_h2]:mt-3 [&_h2]:text-base [&_h2]:font-semibold [&_h3]:mb-1 [&_h3]:mt-2 [&_h3]:text-sm [&_h3]:font-medium [&_p]:my-1.5 [&_p]:leading-relaxed [&_pre]:my-2 [&_pre]:overflow-x-auto [&_pre]:rounded [&_pre]:bg-muted [&_pre]:p-2 [&_pre]:text-xs [&_code]:rounded [&_code]:bg-muted/50 [&_code]:px-1 [&_code]:text-xs [&_code]:font-mono [&_pre_code]:bg-transparent [&_pre_code]:p-0 [&_table]:my-2 [&_table]:w-full [&_table]:border-collapse [&_table]:text-xs [&_th]:border [&_th]:border-border [&_th]:bg-muted/50 [&_th]:p-2 [&_th]:text-left [&_th]:font-medium [&_td]:border [&_td]:border-border [&_td]:p-2 [&_tr]:border-b [&_tr]:border-border [&_blockquote]:my-2 [&_blockquote]:border-l-2 [&_blockquote]:border-muted-foreground/30 [&_blockquote]:pl-3 [&_blockquote]:text-muted-foreground [&_blockquote]:text-xs [&_ul]:my-1.5 [&_ul]:list-disc [&_ul]:pl-5 [&_ol]:my-1.5 [&_ol]:list-decimal [&_ol]:pl-5 [&_li]:my-0.5 [&_a]:text-primary [&_a]:underline [&_hr]:my-3 [&_hr]:border-border [&_img]:max-w-full [&_img]:rounded ui-selectable">
                                              <ReactMarkdown
                                                remarkPlugins={[
                                                  remarkGfm,
                                                  remarkMath,
                                                ]}
                                                rehypePlugins={[rehypeKatex]}
                                              >
                                                {item.result.textBlocks
                                                  .map((b) => b.text)
                                                  .join("\n\n")}
                                              </ReactMarkdown>
                                            </div>
                                          </div>
                                        )}
                                      </>
                                    )}
                                  </>
                                )}
                              </>
                            )}
                          </div>
                        )}

                        {item.state === "error" && (
                          <p className="py-4 text-sm text-red-600 dark:text-red-400">
                            {item.error}
                          </p>
                        )}

                        {item.state === "idle" && (
                          <p className="py-4 text-center text-sm text-muted-foreground">
                            {t("ocr.pending")}
                          </p>
                        )}
                      </div>
                    </div>
                  </CardContent>
                )}
              </Card>
            )
          })}
        </div>
      )}
    </div>
  )
}
