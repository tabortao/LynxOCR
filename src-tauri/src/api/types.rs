use serde::{Deserialize, Serialize};

/// JSON body for base64 image OCR request.
#[derive(Debug, Deserialize)]
pub struct OcrJsonRequest {
    /// Base64-encoded image data (without data URI prefix).
    pub image: String,
    /// Optional model version override.
    pub model: Option<String>,
}

/// Success response wrapper.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrSuccessResponse {
    pub success: bool,
    pub data: OcrData,
    pub model: String,
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