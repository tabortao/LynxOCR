/** 应用配置 */
export interface AppConfig {
  modelPath: string;
  activeOcrModel: string;
  sidebarCollapsed: boolean;
  ocrScreenshotShortcut: string;
  modelDownloadUrls: string[];
  apiServerPort: number;
  apiKey: string;
  apiServerAutoStart: boolean;
  maxFileSizeMb: number;
}

/** 模型下载进度 */
export interface DownloadProgress {
  modelName: string;
  downloaded: number;
  total: number;
  percentage: number;
  stage: string;
}

export interface ModelInfo {
  name: string;
  displayName: string;
  size: string;
  installed: boolean;
  path: string | null;
}

// ============================================================================
// OCR 类型
// ============================================================================

/** OCR 文本块 */
export interface OcrTextBlock {
  text: string;
  confidence: number;
  boxPoints: [number, number][]; // 4 个角点坐标
}

/** OCR 识别结果 */
export interface OcrResult {
  textBlocks: OcrTextBlock[];
  totalTimeMs: number;
}

export interface ScreenshotCapture {
  imagePath: string;
  width: number;
  height: number;
  boundingBox: {
    minX: number;
    minY: number;
    maxX: number;
    maxY: number;
    width: number;
    height: number;
  };
}