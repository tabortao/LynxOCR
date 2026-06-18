import { useState, useEffect, useCallback } from "react"
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card"
import { Label } from "@/components/ui/label"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Switch } from "@/components/ui/switch"
import { NetworkIcon, PlayIcon, SquareIcon, EyeIcon, EyeOffIcon } from "lucide-react"
import { invoke } from "@tauri-apps/api/core"
import { useAppContext } from "@/lib/app-context"
import type { AppConfig } from "@/types"

interface ServerStatus {
  running: boolean
  port: number
}

export function ApiSettingsPage() {
  const { t } = useAppContext()
  const [config, setConfig] = useState<AppConfig | null>(null)
  const [serverStatus, setServerStatus] = useState<ServerStatus>({ running: false, port: 9720 })
  const [saved, setSaved] = useState(false)
  const [showApiKey, setShowApiKey] = useState(false)
  const [loading, setLoading] = useState(false)

  useEffect(() => {
    const load = async () => {
      try {
        const cfg = await invoke<AppConfig>("get_app_config")
        setConfig(cfg)
      } catch {
        // ignore
      }
      try {
        const status = await invoke<ServerStatus>("api_get_server_status")
        setServerStatus(status)
      } catch {
        // ignore
      }
    }
    load()
  }, [])

  const saveConfig = useCallback(async () => {
    if (!config) return
    setLoading(true)
    try {
      await invoke("set_app_config", { newConfig: config })
      setSaved(true)
      setTimeout(() => setSaved(false), 2000)
    } catch (e) {
      console.error("Failed to save config:", e)
    } finally {
      setLoading(false)
    }
  }, [config])

  const toggleServer = useCallback(async () => {
    setLoading(true)
    try {
      if (serverStatus.running) {
        await invoke("api_stop_server")
        setServerStatus({ running: false, port: serverStatus.port })
      } else {
        await saveConfig() // Save config first
        const status = await invoke<ServerStatus>("api_start_server")
        setServerStatus(status)
      }
    } catch (e) {
      console.error("Failed to toggle server:", e)
    } finally {
      setLoading(false)
    }
  }, [serverStatus, saveConfig])

  if (!config) {
    return (
      <div className="flex items-center justify-center py-16">
        <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-primary" />
      </div>
    )
  }

  return (
    <div className="mx-auto w-full max-w-2xl px-4">
      <div className="flex items-center gap-2 mb-6">
        <NetworkIcon className="size-5 text-muted-foreground" />
        <div>
          <h2 className="text-lg font-semibold">{t("api.title")}</h2>
          <p className="text-sm text-muted-foreground">{t("api.desc")}</p>
        </div>
      </div>

      <div className="flex flex-col gap-4">
        {/* Server Status Card */}
        <Card>
          <CardHeader className="pb-3">
            <CardTitle className="text-base">{t("api.serverStatus")}</CardTitle>
            <CardDescription>
              {serverStatus.running
                ? t("api.serverRunning", { port: serverStatus.port })
                : t("api.serverStopped")}
            </CardDescription>
          </CardHeader>
          <CardContent>
            <div className="flex items-center gap-4">
              <div className={`flex items-center gap-2 ${serverStatus.running ? "text-green-600" : "text-muted-foreground"}`}>
                <div className={`size-2.5 rounded-full ${serverStatus.running ? "bg-green-500 animate-pulse" : "bg-gray-400"}`} />
                <span className="text-sm font-medium">
                  {serverStatus.running ? t("api.running") : t("api.stopped")}
                </span>
              </div>
              <Button
                variant={serverStatus.running ? "destructive" : "default"}
                size="sm"
                onClick={toggleServer}
                disabled={loading}
              >
                {serverStatus.running ? (
                  <><SquareIcon className="size-4 mr-1" />{t("api.stop")}</>
                ) : (
                  <><PlayIcon className="size-4 mr-1" />{t("api.start")}</>
                )}
              </Button>
            </div>
          </CardContent>
        </Card>

        {/* Configuration Card */}
        <Card>
          <CardHeader className="pb-3">
            <CardTitle className="text-base">{t("api.configuration")}</CardTitle>
            <CardDescription>{t("api.configDesc")}</CardDescription>
          </CardHeader>
          <CardContent className="flex flex-col gap-4">
            {/* Port */}
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="api-port">{t("api.port")}</Label>
              <Input
                id="api-port"
                type="number"
                min={1024}
                max={65535}
                value={config.apiServerPort || 9720}
                onChange={(e) => setConfig({ ...config, apiServerPort: parseInt(e.target.value) || 9720 })}
                className="max-w-[200px]"
              />
            </div>

            {/* API Key */}
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="api-key">{t("api.key")}</Label>
              <div className="flex gap-2 max-w-[400px]">
                <Input
                  id="api-key"
                  type={showApiKey ? "text" : "password"}
                  value={config.apiKey || ""}
                  onChange={(e) => setConfig({ ...config, apiKey: e.target.value })}
                  placeholder={t("api.keyPlaceholder")}
                />
                <Button
                  variant="outline"
                  size="icon"
                  onClick={() => setShowApiKey(!showApiKey)}
                  title={showApiKey ? t("api.hideKey") : t("api.showKey")}
                >
                  {showApiKey ? <EyeOffIcon className="size-4" /> : <EyeIcon className="size-4" />}
                </Button>
              </div>
              <p className="text-xs text-muted-foreground">{t("api.keyHint")}</p>
            </div>

            {/* Max File Size */}
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="api-max-size">{t("api.maxFileSize")}</Label>
              <Input
                id="api-max-size"
                type="number"
                min={1}
                max={500}
                value={config.maxFileSizeMb || 20}
                onChange={(e) => setConfig({ ...config, maxFileSizeMb: parseInt(e.target.value) || 20 })}
                className="max-w-[200px]"
              />
              <p className="text-xs text-muted-foreground">{t("api.maxFileSizeHint")}</p>
            </div>

            {/* Auto-start */}
            <div className="flex items-center justify-between">
              <div>
                <Label htmlFor="api-auto-start">{t("api.autoStart")}</Label>
                <p className="text-xs text-muted-foreground">{t("api.autoStartHint")}</p>
              </div>
              <Switch
                id="api-auto-start"
                checked={config.apiServerAutoStart || false}
                onCheckedChange={(checked) => setConfig({ ...config, apiServerAutoStart: checked })}
              />
            </div>
          </CardContent>
        </Card>

        {/* Example Usage Card */}
        <Card>
          <CardHeader className="pb-3">
            <CardTitle className="text-base">{t("api.exampleUsage")}</CardTitle>
            <CardDescription>{t("api.exampleDesc")}</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="flex flex-col gap-3">
              <div>
                <p className="text-xs font-medium mb-1">{t("api.healthCheck")}</p>
                <code className="block bg-muted p-2 rounded text-xs font-mono break-all">
                  curl http://localhost:{config.apiServerPort || 9720}/api/v1/health
                </code>
              </div>
              <div>
                <p className="text-xs font-medium mb-1">{t("api.ocrCurl")}</p>
                <code className="block bg-muted p-2 rounded text-xs font-mono break-all">
                  curl -X POST http://localhost:{config.apiServerPort || 9720}/api/v1/ocr \<br/>
                  &nbsp;&nbsp;-F &quot;image=@/path/to/image.png&quot;
                </code>
                {config.apiKey && (
                  <code className="block bg-muted p-2 rounded text-xs font-mono break-all mt-1">
                    curl -X POST http://localhost:{config.apiServerPort || 9720}/api/v1/ocr \<br/>
                    &nbsp;&nbsp;-H &quot;Authorization: Bearer {config.apiKey}&quot; \<br/>
                    &nbsp;&nbsp;-F &quot;image=@/path/to/image.png&quot;
                  </code>
                )}
              </div>
              <div>
                <p className="text-xs font-medium mb-1">{t("api.base64Curl")}</p>
                <code className="block bg-muted p-2 rounded text-xs font-mono break-all">
                  curl -X POST http://localhost:{config.apiServerPort || 9720}/api/v1/ocr \<br/>
                  &nbsp;&nbsp;-H &quot;Content-Type: application/json&quot; \<br/>
                  &nbsp;&nbsp;-d &apos;{'{'}&quot;image&quot;: &quot;base64...&quot;{'}'}&apos;
                </code>
              </div>
              <div>
                <p className="text-xs font-medium mb-1">{t("api.urlCurl")}</p>
                <code className="block bg-muted p-2 rounded text-xs font-mono break-all">
                  curl -X POST http://localhost:{config.apiServerPort || 9720}/api/v1/ocr \<br/>
                  &nbsp;&nbsp;-H &quot;Content-Type: application/json&quot; \<br/>
                  &nbsp;&nbsp;-d &apos;{'{'}&quot;url&quot;: &quot;https://example.com/image.png&quot;{'}'}&apos;
                </code>
              </div>
              <div>
                <p className="text-xs font-medium mb-1">{t("api.specifyModel")}</p>
                <code className="block bg-muted p-2 rounded text-xs font-mono break-all">
                  curl -X POST http://localhost:{config.apiServerPort || 9720}/api/v1/ocr \<br/>
                  &nbsp;&nbsp;-F &quot;image=@/path/to/image.png&quot; \<br/>
                  &nbsp;&nbsp;-F &quot;model=ppocr-v5&quot;
                </code>
              </div>
            </div>
          </CardContent>
        </Card>

        {/* Save Button */}
        <div className="flex justify-end">
          <Button onClick={saveConfig} disabled={loading}>
            {saved ? t("api.saved") : t("api.save")}
          </Button>
        </div>
      </div>
    </div>
  )
}