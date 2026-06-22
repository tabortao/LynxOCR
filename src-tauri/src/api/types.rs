use serde::{Deserialize, Serialize};

/// JSON body for OCR request (base64 image or image URL).
#[derive(Debug, Deserialize)]
pub struct OcrJsonRequest {
    /// Base64-encoded image data (without data URI prefix).
    /// Mutually exclusive with `url` — provide one or the other.
    pub image: Option<String>,
    /// URL of the image to download and OCR.
    /// Mutually exclusive with `image` — provide one or the other.
    pub url: Option<String>,
    /// Optional model version override.
    pub model: Option<String>,
    /// Output format for MinerU (md, html, latex, docx, json).
    /// Ignored for PaddleOCR models.
    #[serde(default)]
    pub format: Option<String>,
    /// MinerU mode override: "flash" or "extract".
    /// When empty, mode is determined by whether token is configured.
    #[serde(default)]
    pub mineru_mode: Option<String>,
}

/// Success response wrapper.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrSuccessResponse {
    pub success: bool,
    pub data: OcrData,
    pub model: String,
    /// Output format for MinerU results (md, html, latex, docx, json).
    /// Omitted for PaddleOCR results.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

/// OCR data payload.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrData {
    /// Concatenated text from all detected regions.
    pub text: String,
    /// List of detected text regions.
    pub regions: Vec<RegionInfo>,
    /// Total recognition time in milliseconds.
    pub total_time_ms: u64,
    /// Output format for MinerU results (md, html, latex, docx, json).
    /// Omitted for PaddleOCR results.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

/// Single text region with bounding box and confidence.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegionInfo {
    pub text: String,
    pub confidence: f32,
    /// 4 corner points of the bounding box (clockwise from top-left).
    pub bbox: Vec<[f32; 2]>,
}

/// Error response.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
    pub success: bool,
    pub error: ErrorDetail,
}

/// Error detail.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorDetail {
    pub code: String,
    pub message: String,
}

/// Health check response.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthResponse {
    pub status: String,
    pub model_loaded: bool,
    pub active_model: String,
    pub version: String,
}

/// Service info response.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InfoResponse {
    pub name: String,
    pub version: String,
    pub engine: String,
    pub available_models: Vec<String>,
    pub active_model: String,
    pub max_file_size_mb: u32,
}

/// Server status for Tauri commands.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerStatus {
    pub running: bool,
    pub port: u16,
}