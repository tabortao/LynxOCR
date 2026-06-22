import { useState, useEffect } from "react"
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { Button } from "@/components/ui/button"
import { Progress } from "@/components/ui/progress"
import { Badge } from "@/components/ui/badge"
import { Label } from "@/components/ui/label"
import { CpuIcon, DownloadIcon, CheckCircleIcon, FolderOpenIcon, CloudIcon, EyeIcon, EyeOffIcon } from "lucide-react"
import { invoke } from "@tauri-apps/api/core"
import { listen } from "@tauri-apps/api/event"
import { useAppContext } from "@/lib/app-context"
import type { AppConfig, ModelInfo, DownloadProgress } from "@/types"

const OCR_MODELS = ["ppocr-v4", "ppocr-v5", "ppocr-v6", "mineru"] as const

const MINERU_FORMATS = [
  { value: "md", label: "Markdown (.md)" },
  { value: "html", label: "HTML (.html)" },
  { value: "latex", label: "LaTeX (.tex)" },
  { value: "docx", label: "DOCX (.docx)" },
  { value: "json", label: "JSON (.json)" },
]

export function ModelSettingsPage() {
  const { t } = useAppContext()
  const [config, setConfig] = useState<AppConfig | null>(null)
  const [models, setModels] = useState<ModelInfo[]>([])
  const [downloading, setDownloading] = useState<string | null>(null)
  const [downloadProgress, setDownloadProgress] = useState<DownloadProgress | null>(null)
  const [downloadError, setDownloadError] = useState<string | null>(null)
  const [selectedOcr, setSelectedOcr] = useState<string>("ppocr-v6")
  const [showMineruToken, setShowMineruToken] = useState(false)

  const getModel = (name: string) => models.find((m) => m.name === name)
  const isInstalled = (name: string) => getModel(name)?.installed ?? false

  const loadConfig = async () => {
    try {
      const cfg = await invoke<AppConfig>("get_app_config")
      setConfig(cfg)
    } catch (err) {
      console.error("Failed to load config:", err)
    }
  }

  const loadModels = async () => {
    try {
      const mods = await invoke<ModelInfo[]>("list_models")
      setModels(mods)
    } catch (err) {
      console.error("Failed to load models:", err)
    }
  }

  useEffect(() => {
    loadConfig()
    loadModels()

    let unlisten: (() => void) | undefined
    const setupListener = async () => {
      try {
        unlisten = await listen<DownloadProgress>("model-download-progress", (event) => {
          setDownloadProgress(event.payload)
          if (event.payload.stage === "completed") {
            setDownloading(null)
            loadModels()
          }
        })
      } catch (err) {
        console.error("Failed to listen download events:", err)
      }
    }
    setupListener()

    return () => {
      if (unlisten) unlisten()
    }
  }, [])

  const saveModelPath = async () => {
    if (!config) return
    try {
      await invoke("set_app_config", { newConfig: config })
      loadModels()
    } catch (err) {
      console.error("Failed to save model path:", err)
    }
  }

  const saveMineruConfig = async () => {
    if (!config) return
    try {
      await invoke("set_app_config", { newConfig: config })
    } catch (err) {
      console.error("Failed to save MinerU config:", err)
    }
  }

  const handleDownload = async (modelName: string) => {
    setDownloading(modelName)
    setDownloadError(null)
    setDownloadProgress(null)
    try {
      await invoke<string>("download_specific_model", { modelName })
    } catch (err) {
      setDownloadError(String(err))
      setDownloading(null)
    }
  }

  return (
    <div className="px-4 lg:px-6 space-y-4">
      {/* Model storage path */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <FolderOpenIcon className="size-5" />
            {t("models.title.storage")}
          </CardTitle>
          <CardDescription>{t("models.desc.storage")}</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex gap-2">
            <Input
              value={config?.modelPath || ""}
              onChange={(e) => setConfig((prev) => prev ? { ...prev, modelPath: e.target.value } : prev)}
              placeholder={t("models.desc.storage")}
              className="flex-1"
            />
            <Button variant="outline" onClick={saveModelPath}>
              {t("models.save")}
            </Button>
          </div>
        </CardContent>
      </Card>

      {/* OCR Models */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <CpuIcon className="size-5" />
            {t("models.title.management")}
          </CardTitle>
          <CardDescription>{t("models.desc.management")}</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {/* OCR Model Selection */}
          <div className="space-y-3">
            <h4 className="text-sm font-medium text-muted-foreground">OCR</h4>
            <div className="flex items-center gap-3">
              <select
                value={selectedOcr}
                onChange={(e) => setSelectedOcr(e.target.value)}
                className="h-9 rounded-md border border-input bg-background px-3 py-1 text-sm flex-1"
              >
                {OCR_MODELS.map((key) => {
                  const model = getModel(key)
                  return (
                    <option key={key} value={key}>
                      {model?.displayName ?? key}{isInstalled(key) ? " ✓" : " (not installed)"}
                    </option>
                  )
                })}
              </select>

              {(() => {
                const selectedModel = getModel(selectedOcr)
                if (!selectedModel) return null

                if (!selectedModel.installed) {
                  return (
                    <Button
                      onClick={() => handleDownload(selectedOcr)}
                      disabled={downloading !== null}
                      size="sm"
                    >
                      <DownloadIcon className="size-4 mr-1" />
                      {downloading === selectedOcr ? t("models.downloading") : t("models.download")}
                    </Button>
                  )
                }

                return (
                  <Badge variant="outline" className="h-8 px-3 gap-1 border-green-300 dark:border-green-700">
                    <CheckCircleIcon className="size-3 text-green-600 dark:text-green-400" />
                    <span className="text-green-700 dark:text-green-300">{t("models.installed")}</span>
                  </Badge>
                )
              })()}
            </div>
            {(() => {
              const sm = getModel(selectedOcr)
              if (sm && sm.installed && sm.path) {
                return (
                  <p className="text-xs text-muted-foreground">
                    {sm.path}
                  </p>
                )
              }
              return null
            })()}
          </div>

          {/* Download progress */}
          {downloading && downloadProgress && (
            <div className="space-y-2 p-4 border rounded-lg bg-muted/30">
              <div className="flex items-center justify-between">
                <span className="text-sm font-medium">{downloadProgress.stage}</span>
                <span className="text-sm text-muted-foreground">
                  {downloadProgress.percentage.toFixed(0)}%
                </span>
              </div>
              <Progress value={downloadProgress.percentage} className="h-2" />
              {downloadProgress.total > 0 && (
                <p className="text-xs text-muted-foreground">
                  {(downloadProgress.downloaded / 1024 / 1024).toFixed(1)} MB
                  {" / "}
                  {(downloadProgress.total / 1024 / 1024).toFixed(1)} MB
                </p>
              )}
            </div>
          )}

          {/* Download error */}
          {downloadError && (
            <div className="p-3 border border-red-200 rounded-lg bg-red-50 dark:bg-red-950 dark:border-red-800">
              <p className="text-sm text-red-600 dark:text-red-400">{downloadError}</p>
            </div>
          )}

          {/* Hint */}
          {!models.some((m) => m.installed && OCR_MODELS.includes(m.name as typeof OCR_MODELS[number])) && !downloading && (
            <div className="text-sm text-muted-foreground py-2">
              {t("models.downloadHint")}
            </div>
          )}
        </CardContent>
      </Card>

      {/* MinerU API Settings */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <CloudIcon className="size-5" />
            {t("models.mineru.title")}
          </CardTitle>
          <CardDescription>{t("models.mineru.desc")}</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {/* API Token */}
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="mineru-token">{t("models.mineru.token")}</Label>
            <div className="flex gap-2">
              <Input
                id="mineru-token"
                type={showMineruToken ? "text" : "password"}
                value={config?.mineruApiToken || ""}
                onChange={(e) => setConfig((prev) => prev ? { ...prev, mineruApiToken: e.target.value } : prev)}
                placeholder={t("models.mineru.tokenPlaceholder")}
              />
              <Button
                variant="outline"
                size="icon"
                onClick={() => setShowMineruToken(!showMineruToken)}
                title={showMineruToken ? t("api.hideKey") : t("api.showKey")}
              >
                {showMineruToken ? <EyeOffIcon className="size-4" /> : <EyeIcon className="size-4" />}
              </Button>
              <Button variant="outline" onClick={saveMineruConfig}>
                {t("models.save")}
              </Button>
            </div>
            <p className="text-xs text-muted-foreground">{t("models.mineru.tokenHint")}</p>
          </div>

          {/* API Base URL */}
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="mineru-base-url">{t("models.mineru.baseUrl")}</Label>
            <Input
              id="mineru-base-url"
              value={config?.mineruApiBaseUrl || ""}
              onChange={(e) => setConfig((prev) => prev ? { ...prev, mineruApiBaseUrl: e.target.value || null } : prev)}
              placeholder={t("models.mineru.baseUrlPlaceholder")}
              className="max-w-[400px]"
            />
            <p className="text-xs text-muted-foreground">{t("models.mineru.baseUrlHint")}</p>
          </div>

          {/* Default Output Format */}
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="mineru-format">{t("models.mineru.outputFormat")}</Label>
            <select
              id="mineru-format"
              value={config?.mineruOutputFormat || "md"}
              onChange={(e) => setConfig((prev) => prev ? { ...prev, mineruOutputFormat: e.target.value } : prev)}
              className="h-9 rounded-md border border-input bg-background px-3 py-1 text-sm max-w-[200px]"
            >
              {MINERU_FORMATS.map((fmt) => (
                <option key={fmt.value} value={fmt.value}>
                  {fmt.label}
                </option>
              ))}
            </select>
            <p className="text-xs text-muted-foreground">{t("models.mineru.formatHint")}</p>
          </div>
        </CardContent>
      </Card>
    </div>
  )
}