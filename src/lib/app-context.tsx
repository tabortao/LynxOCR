import { createContext, useContext, useState, useEffect, type ReactNode } from "react"

export type Language = "zh" | "en"

// Comprehensive i18n dictionary
const dict = {
  zh: {
    // Sidebar
    features: "功能",
    ocr: "文本识别",
    api: "API 服务",
    settingsLabel: "设置",
    settings: "设置",
    models: "模型管理",
    about: "关于",

    // SiteHeader
    "header.ocr": "文本识别",
    "header.settings": "设置",
    "header.model-settings": "模型管理",
    "header.api-settings": "API 服务",
    "header.models": "模型管理",
    "header.about": "关于",

    // Settings page
    "settings.title": "应用设置",
    "settings.desc": "配置截图 OCR 快捷键",
    "settings.save": "保存设置",
    "settings.saved": "已保存",
    "settings.loading": "加载中...",
    "settings.ocrScreenshotShortcut": "截图 OCR 快捷键",
    "settings.ocrScreenshotShortcutDesc": "设置截图识别的快捷键，例如 Ctrl+Shift+O",

    // Model settings page
    "models.title.storage": "模型存储路径",
    "models.desc.storage": "设置 OCR 模型的存放目录",
    "models.save": "保存",
    "models.title.management": "模型管理",
    "models.desc.management": "下载和管理 OCR 识别模型。",
    "models.installed": "已安装",
    "models.download": "一键下载",
    "models.downloading": "下载中...",
    "models.ppocrV4Desc": "PaddleOCR V4，中文/英文检测与识别，约 20MB",
    "models.ppocrV5Desc": "PaddleOCR V5，更高中文/英文识别精度，约 20MB",
    "models.ppocrV6Desc": "PaddleOCR V6，最新版本，多语言高精度，约 20MB",
    "models.downloadHint": "下载模型后将自动安装到模型路径中。",

    // MinerU settings
    "models.mineru.title": "MinerU API 设置",
    "models.mineru.desc": "配置 MinerU 云 API，用于高精度文档解析",
    "models.mineru.token": "API Token",
    "models.mineru.tokenPlaceholder": "输入 MinerU API Token（留空使用 Flash 模式）",
    "models.mineru.tokenHint": "填入 Token 后可使用 Extract 模式，支持多格式输出",
    "models.mineru.baseUrl": "API 地址",
    "models.mineru.baseUrlPlaceholder": "默认 https://mineru.net",
    "models.mineru.baseUrlHint": "私有部署时填写自定义地址",
    "models.mineru.outputFormat": "默认输出格式",
    "models.mineru.formatHint": "Extract 模式下支持多格式输出",

    // About page
    "about.title": "关于",
    "about.desc": "离线 OCR 文字识别工具",
    "about.description": "LynxOCR 是一款跨平台离线 OCR 文字识别桌面应用。基于 PaddleOCR (PP-OCR V4/V5/V6) 和 ONNX Runtime，支持截图识别和图片/PDF 文字提取，无需联网，保障数据隐私。",
    "about.techStack": "技术栈",
    "about.version": "版本",

    // OCR page
    "ocr.title": "OCR 文字识别",
    "ocr.desc": "支持 PNG, JPG, JPEG, BMP, WEBP 等图片格式及 PDF 文档",
    "ocr.dropImages": "拖入文件进行识别",
    "ocr.clickOrDrag": "点击选择或拖拽文件（图片/PDF）到此处",
    "ocr.selectImage": "选择文件",
    "ocr.processing": "识别中...",
    "ocr.completed": "识别完成",
    "ocr.noTextFound": "未检测到文字",
    "ocr.modelVersion": "模型版本",
    "ocr.copyText": "复制文本",
    "ocr.exportTxt": "TXT",
    "ocr.confidence": "置信度",
    "ocr.clear": "清除",
    "ocr.textBlocks": "{count} 个文本块",
    "ocr.noModel": "未安装 OCR 模型，请先在模型管理中下载",
    "ocr.failed": "OCR 识别失败",
    "ocr.modelNotInstalled": "OCR 模型 {model} 未安装，请先在模型管理中下载",
    "ocr.completedToast": "文本识别完成 · {blocks} 个文本块 · 耗时 {time}秒",
    "ocr.screenshotBtn": "截图识别",
    "ocr.screenshotDesc": "截取屏幕区域并识别文字",
    "ocr.screenshotDone": "已复制到剪贴板",
    "ocr.startBatch": "开始识别",
    "ocr.retryFailed": "重试失败",
    "ocr.batchProgress": "识别中 {current}/{total}",
    "ocr.batchDone": "完成 {count} 张图片识别，耗时 {time}秒",
    "ocr.copyAllText": "复制全部",
    "ocr.batchExportDone": "已导出 {count} 个文件",
    "ocr.pending": "等待识别",
    "ocr.pdfPage": "第 {page} 页",
    "ocr.viewPlain": "纯文本",
    "ocr.viewMarkdown": "Markdown 预览",
    "ocr.exportMd": "Markdown",
    "ocr.exportAs": "导出",

    // MinerU OCR
    "ocr.mineru": "MinerU Cloud",
    "ocr.mineruFormat": "输出格式",
    "ocr.mineruNoToken": "未配置 MinerU API Token，将使用 Flash 模式（仅支持 Markdown 输出）",
    "ocr.mineruFlashMode": "Flash 模式",
    "ocr.mineruExtractMode": "Extract 模式",
    "ocr.viewHtml": "HTML 预览",
    "ocr.viewLatex": "LaTeX 源码",
    "ocr.exportHtml": "HTML",
    "ocr.exportLatex": "LaTeX",
    "ocr.exportDocx": "DOCX",
    "ocr.exportJson": "JSON",

    // API settings page
    "api.title": "API 服务",
    "api.desc": "启用内置 HTTP API 服务，供其他应用调用 OCR 功能",
    "api.serverStatus": "服务状态",
    "api.serverRunning": "正在运行 · 端口 {port}",
    "api.serverStopped": "服务未启动",
    "api.running": "运行中",
    "api.stopped": "已停止",
    "api.start": "启动服务",
    "api.stop": "停止服务",
    "api.configuration": "服务配置",
    "api.configDesc": "配置 API 服务的端口、认证和限制",
    "api.port": "端口号",
    "api.key": "API 密钥 (可选)",
    "api.keyPlaceholder": "留空则不启用认证",
    "api.keyHint": "设置后，除 /health 外的所有接口都需要 Bearer Token 认证",
    "api.hideKey": "隐藏密钥",
    "api.showKey": "显示密钥",
    "api.maxFileSize": "最大上传文件大小 (MB)",
    "api.maxFileSizeHint": "超过此大小的图片将被拒绝",
    "api.autoStart": "开机自动启动服务",
    "api.autoStartHint": "应用启动时自动开启 API 服务",
    "api.exampleUsage": "使用示例",
    "api.exampleDesc": "以下是调用 API 的 curl 命令示例",
    "api.healthCheck": "健康检查",
    "api.ocrCurl": "图片上传识别 (multipart)",
    "api.base64Curl": "Base64 图片识别 (JSON)",
    "api.urlCurl": "图床链接图片识别 (JSON)",
    "api.specifyModel": "指定模型版本",
    "api.mineruCurl": "MinerU 快读识别",
    "api.mineruExtractCurl": "MinerU 精确提取",
    "api.save": "保存配置",
    "api.saved": "已保存",
  },
  en: {
    // Sidebar
    features: "Features",
    ocr: "Text Recognition",
    settingsLabel: "Settings",
    settings: "Settings",
    models: "Models",
    about: "About",

    // SiteHeader
    "header.ocr": "Text Recognition",
    "header.settings": "Settings",
    "header.model-settings": "Models",
    "header.models": "Models",
    "header.about": "About",

    // Settings page
    "settings.title": "Settings",
    "settings.desc": "Configure screenshot OCR shortcut",
    "settings.save": "Save Settings",
    "settings.saved": "Saved",
    "settings.loading": "Loading...",
    "settings.ocrScreenshotShortcut": "Screenshot OCR Shortcut",
    "settings.ocrScreenshotShortcutDesc": "Set the keyboard shortcut for screenshot OCR, e.g. Ctrl+Shift+O",

    // Model settings page
    "models.title.storage": "Model Storage Path",
    "models.desc.storage": "Set the directory for OCR models",
    "models.save": "Save",
    "models.title.management": "Model Management",
    "models.desc.management": "Download and manage OCR recognition models.",
    "models.installed": "Installed",
    "models.download": "Download",
    "models.downloading": "Downloading...",
    "models.ppocrV4Desc": "PaddleOCR V4, Chinese/English text detection & recognition, ~20MB",
    "models.ppocrV5Desc": "PaddleOCR V5, improved Chinese/English accuracy, ~20MB",
    "models.ppocrV6Desc": "PaddleOCR V6, latest version, multilingual high accuracy, ~20MB",
    "models.downloadHint": "Download models. Automatically installed to the model path after download.",

    // MinerU settings
    "models.mineru.title": "MinerU API Settings",
    "models.mineru.desc": "Configure MinerU cloud API for high-precision document parsing",
    "models.mineru.token": "API Token",
    "models.mineru.tokenPlaceholder": "Enter MinerU API Token (leave empty for Flash mode)",
    "models.mineru.tokenHint": "With token, Extract mode is available with multi-format output",
    "models.mineru.baseUrl": "API Base URL",
    "models.mineru.baseUrlPlaceholder": "Default https://mineru.net",
    "models.mineru.baseUrlHint": "Custom address for private deployments",
    "models.mineru.outputFormat": "Default Output Format",
    "models.mineru.formatHint": "Extract mode supports multi-format output",

    // About page
    "about.title": "About",
    "about.desc": "Offline OCR Text Recognition",
    "about.description": "LynxOCR is a cross-platform desktop application for offline OCR text recognition. Powered by PaddleOCR (PP-OCR V4/V5/V6) and ONNX Runtime, it supports screenshot OCR and image/PDF text extraction without requiring an internet connection, ensuring your data privacy.",
    "about.techStack": "Tech Stack",
    "about.version": "Version",

    // OCR page
    "ocr.title": "OCR Text Recognition",
    "ocr.desc": "Supports PNG, JPG, JPEG, BMP, WEBP image formats and PDF documents",
    "ocr.dropImages": "Drop files to recognize text",
    "ocr.clickOrDrag": "Click to select or drag files (images/PDF) here",
    "ocr.selectImage": "Select File",
    "ocr.processing": "Processing...",
    "ocr.completed": "Recognition complete",
    "ocr.noTextFound": "No text detected",
    "ocr.modelVersion": "Model Version",
    "ocr.copyText": "Copy Text",
    "ocr.exportTxt": "TXT",
    "ocr.confidence": "Confidence",
    "ocr.clear": "Clear",
    "ocr.textBlocks": "{count} text block(s)",
    "ocr.noModel": "No OCR model installed. Please download one in Model Settings.",
    "ocr.failed": "OCR recognition failed",
    "ocr.modelNotInstalled": "OCR model {model} is not installed. Please download it in Model Settings.",
    "ocr.completedToast": "OCR complete · {blocks} text block(s) · {time}s",
    "ocr.screenshotBtn": "Screenshot OCR",
    "ocr.screenshotDesc": "Capture screen region and recognize text",
    "ocr.screenshotDone": "Copied to clipboard",
    "ocr.startBatch": "Start OCR",
    "ocr.retryFailed": "Retry Failed",
    "ocr.batchProgress": "Processing {current}/{total}",
    "ocr.batchDone": "Completed {count} images in {time}s",
    "ocr.copyAllText": "Copy All",
    "ocr.batchExportDone": "Exported {count} files",
    "ocr.pending": "Pending",
    "ocr.pdfPage": "Page {page}",
    "ocr.viewPlain": "Plain Text",
    "ocr.viewMarkdown": "Markdown Preview",
    "ocr.exportMd": "MD",
    "ocr.exportAs": "Export",

    // MinerU OCR
    "ocr.mineru": "MinerU Cloud",
    "ocr.mineruFormat": "Output Format",
    "ocr.mineruNoToken": "No MinerU API token configured. Flash mode will be used (Markdown only).",
    "ocr.mineruFlashMode": "Flash Mode",
    "ocr.mineruExtractMode": "Extract Mode",
    "ocr.viewHtml": "HTML Preview",
    "ocr.viewLatex": "LaTeX Source",
    "ocr.exportHtml": "HTML",
    "ocr.exportLatex": "LaTeX",
    "ocr.exportDocx": "DOCX",
    "ocr.exportJson": "JSON",

    // API settings page
    "api": "API Service",
    "header.api-settings": "API Service",
    "api.title": "API Service",
    "api.desc": "Enable built-in HTTP API service for other applications to call OCR",
    "api.serverStatus": "Server Status",
    "api.serverRunning": "Running · Port {port}",
    "api.serverStopped": "Server not running",
    "api.running": "Running",
    "api.stopped": "Stopped",
    "api.start": "Start Server",
    "api.stop": "Stop Server",
    "api.configuration": "Configuration",
    "api.configDesc": "Configure API server port, authentication and limits",
    "api.port": "Port",
    "api.key": "API Key (optional)",
    "api.keyPlaceholder": "Leave empty to disable authentication",
    "api.keyHint": "When set, all endpoints except /health require Bearer token authentication",
    "api.hideKey": "Hide key",
    "api.showKey": "Show key",
    "api.maxFileSize": "Max upload file size (MB)",
    "api.maxFileSizeHint": "Images exceeding this size will be rejected",
    "api.autoStart": "Auto-start on launch",
    "api.autoStartHint": "Start API server automatically when the app launches",
    "api.exampleUsage": "Usage Examples",
    "api.exampleDesc": "Example curl commands for calling the API",
    "api.healthCheck": "Health Check",
    "api.ocrCurl": "Image Upload OCR (multipart)",
    "api.base64Curl": "Base64 Image OCR (JSON)",
    "api.urlCurl": "Image URL OCR (JSON)",
    "api.specifyModel": "Specify Model Version",
    "api.mineruCurl": "MinerU Flash Extract",
    "api.mineruExtractCurl": "MinerU Extract",
    "api.save": "Save Configuration",
    "api.saved": "Saved",
  },
}

export function t(language: Language, key: string, vars?: Record<string, string | number>): string {
  const langDict = dict[language] as Record<string, string>
  const fallbackDict = dict["en"] as Record<string, string>
  let text = langDict[key] ?? fallbackDict[key] ?? key
  if (vars) {
    for (const [k, v] of Object.entries(vars)) {
      text = text.replace(`{${k}}`, String(v))
    }
  }
  return text
}

interface AppContextType {
  theme: "light" | "dark"
  toggleTheme: () => void
  language: Language
  setLanguage: (lang: Language) => void
  t: (key: string, vars?: Record<string, string | number>) => string
}

const AppContext = createContext<AppContextType>({
  theme: "dark",
  toggleTheme: () => {},
  language: "zh",
  setLanguage: () => {},
  t: (key) => key,
})

export function useAppContext() {
  return useContext(AppContext)
}

export function AppProvider({ children }: { children: ReactNode }) {
  const [theme, setTheme] = useState<"light" | "dark">(() => {
    if (typeof window !== "undefined") {
      return (localStorage.getItem("lynxocr-theme") as "light" | "dark") || "dark"
    }
    return "dark"
  })

  const [language, setLanguage] = useState<Language>(() => {
    if (typeof window !== "undefined") {
      return (localStorage.getItem("lynxocr-lang") as Language) || "zh"
    }
    return "zh"
  })

  useEffect(() => {
    const root = document.documentElement
    if (theme === "dark") {
      root.classList.add("dark")
    } else {
      root.classList.remove("dark")
    }
    localStorage.setItem("lynxocr-theme", theme)
  }, [theme])

  useEffect(() => {
    localStorage.setItem("lynxocr-lang", language)
  }, [language])

  const toggleTheme = () => {
    setTheme((prev) => (prev === "dark" ? "light" : "dark"))
  }

  const translate = (key: string, vars?: Record<string, string | number>) => t(language, key, vars)

  return (
    <AppContext.Provider value={{ theme, toggleTheme, language, setLanguage, t: translate }}>
      {children}
    </AppContext.Provider>
  )
}