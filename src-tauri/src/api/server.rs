use crate::api::types::*;
use crate::engine::model_manager::is_ppocr_installed_at;
use crate::engine::ocr::OcrEngine;
use axum::{
    extract::{DefaultBodyLimit, State},
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;
use tower_http::cors::{Any, CorsLayer};

/// Shared state for the API server.
#[derive(Clone)]
pub struct ApiState {
    pub engine: Arc<Mutex<Option<OcrEngine>>>,
    pub active_model: Arc<Mutex<String>>,
    pub model_path: String,
    pub api_key: String,
    pub max_file_size_mb: u32,
    pub app_version: String,
}

/// Handle for controlling the API server lifecycle.
pub struct ServerHandle {
    pub shutdown_tx: Mutex<Option<oneshot::Sender<()>>>,
}

impl ServerHandle {
    /// Send shutdown signal to the server.
    pub fn shutdown(&self) {
        if let Ok(mut tx) = self.shutdown_tx.lock() {
            if let Some(sender) = tx.take() {
                let _ = sender.send(());
            }
        }
    }
}

/// Start the API server. Returns a handle that can be used to shut it down.
pub async fn start_api_server(
    port: u16,
    engine: Arc<Mutex<Option<OcrEngine>>>,
    active_model: Arc<Mutex<String>>,
    model_path: String,
    api_key: String,
    max_file_size_mb: u32,
    app_version: String,
) -> Result<ServerHandle, String> {
    let state = ApiState {
        engine,
        active_model,
        model_path,
        api_key,
        max_file_size_mb,
        app_version,
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let body_limit = DefaultBodyLimit::max(max_file_size_mb as usize * 1024 * 1024);

    let app = Router::new()
        .route("/api/v1/ocr", post(handle_ocr))
        .route("/api/v1/health", get(handle_health))
        .route("/api/v1/info", get(handle_info))
        .layer(cors)
        .layer(body_limit)
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| format!("Failed to bind to port {port}: {e}"))?;

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    let server = axum::serve(listener, app);

    tokio::spawn(async move {
        let graceful = server.with_graceful_shutdown(async {
            let _ = shutdown_rx.await;
        });
        if let Err(e) = graceful.await {
            log::error!("[API] server error: {e}");
        }
        log::info!("[API] server stopped");
    });

    log::info!("[API] server started on port {port}");

    Ok(ServerHandle { shutdown_tx: Mutex::new(Some(shutdown_tx)) })
}

/// Check authentication. Returns an error response if auth fails.
fn check_auth(headers: &HeaderMap, api_key: &str) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    if api_key.is_empty() {
        return Ok(());
    }

    let auth_header = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let expected = format!("Bearer {api_key}");
    if auth_header != expected {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                success: false,
                error: ErrorDetail {
                    code: "UNAUTHORIZED".into(),
                    message: "Invalid or missing API key".into(),
                },
            }),
        ));
    }

    Ok(())
}

/// Get or create the OCR engine for a specific model.
fn get_or_create_engine(
    state: &ApiState,
    model_version: &str,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    let mut guard = state.engine.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                success: false,
                error: ErrorDetail {
                    code: "INTERNAL_ERROR".into(),
                    message: "Failed to acquire engine lock".into(),
                },
            }),
        )
    })?;

    if guard.is_none() {
        let model_dir = PathBuf::from(&state.model_path).join(model_version);
        let eng = OcrEngine::new(&model_dir).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    success: false,
                    error: ErrorDetail {
                        code: "OCR_ERROR".into(),
                        message: format!("Failed to create OCR engine: {e}"),
                    },
                }),
            )
        })?;
        *guard = Some(eng);
    }

    Ok(())
}

/// Validate model version and check installation.
fn validate_model(
    state: &ApiState,
    model_version: &str,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    let valid_models = ["ppocr-v4", "ppocr-v5", "ppocr-v6"];
    if !valid_models.contains(&model_version) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                success: false,
                error: ErrorDetail {
                    code: "INVALID_MODEL".into(),
                    message: format!(
                        "Unknown model: {model_version}. Must be one of: {}",
                        valid_models.join(", ")
                    ),
                },
            }),
        ));
    }

    let model_dir = PathBuf::from(&state.model_path).join(model_version);
    if !is_ppocr_installed_at(&model_dir) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                success: false,
                error: ErrorDetail {
                    code: "MODEL_NOT_INSTALLED".into(),
                    message: format!(
                        "OCR model {model_version} is not installed. Please download it from Model Settings."
                    ),
                },
            }),
        ));
    }

    Ok(())
}

/// Run OCR on image bytes and return the result.
fn run_ocr(
    state: &ApiState,
    image_data: &[u8],
    model_version: &str,
) -> Result<OcrSuccessResponse, (StatusCode, Json<ErrorResponse>)> {
    let mut guard = state.engine.lock().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                success: false,
                error: ErrorDetail {
                    code: "INTERNAL_ERROR".into(),
                    message: "Failed to acquire engine lock".into(),
                },
            }),
        )
    })?;

    let eng = guard.as_mut().ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                success: false,
                error: ErrorDetail {
                    code: "INTERNAL_ERROR".into(),
                    message: "OCR engine not initialized".into(),
                },
            }),
        )
    })?;

    let result = eng.recognize_from_bytes(image_data).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                success: false,
                error: ErrorDetail {
                    code: "OCR_ERROR".into(),
                    message: format!("OCR recognition failed: {e}"),
                },
            }),
        )
    })?;

    let text = result
        .text_blocks
        .iter()
        .map(|b| b.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");

    let regions: Vec<RegionInfo> = result
        .text_blocks
        .iter()
        .map(|b| RegionInfo {
            text: b.text.clone(),
            confidence: b.confidence,
            bbox: b.box_points.clone(),
        })
        .collect();

    Ok(OcrSuccessResponse {
        success: true,
        data: OcrData {
            text,
            regions,
            total_time_ms: result.total_time_ms,
        },
        model: model_version.to_string(),
    })
}

/// Parse base64 image, supporting both plain base64 and data URI format.
fn parse_base64_image(input: &str) -> Result<Vec<u8>, String> {
    // Handle data URI format: data:image/png;base64,xxxxx
    let base64 = if let Some(comma_pos) = input.find(',') {
        if input[..comma_pos].contains("base64") {
            &input[comma_pos + 1..]
        } else {
            return Err("Invalid data URI: missing base64 indicator".into());
        }
    } else {
        input
    };

    let cleaned = base64.trim().replace(char::is_whitespace, "");
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(&cleaned)
        .map_err(|e| format!("Failed to decode base64: {e}"))
}

// ── Route Handlers ──

/// POST /api/v1/ocr — unified handler for both multipart and JSON.
async fn handle_ocr(
    State(state): State<ApiState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> axum::response::Response {
    // Check auth
    if let Err(e) = check_auth(&headers, &state.api_key) {
        return e.into_response();
    }

    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if content_type.starts_with("multipart/form-data") {
        handle_ocr_multipart(state, headers, body).await.into_response()
    } else {
        handle_ocr_json(state, &body).await.into_response()
    }
}

/// Handle multipart/form-data upload.
async fn handle_ocr_multipart(
    state: ApiState,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    // Reconstruct multipart boundary from content-type header
    let boundary = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .and_then(|ct| {
            ct.split(';')
                .map(|p| p.trim())
                .find(|p| p.starts_with("boundary="))
                .map(|p| p.trim_start_matches("boundary=").to_string())
        })
        .unwrap_or_default();

    if boundary.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                success: false,
                error: ErrorDetail {
                    code: "NO_IMAGE".into(),
                    message: "Missing multipart boundary".into(),
                },
            }),
        )
            .into_response();
    }

    // Parse multipart body
    let mut image_data: Option<Vec<u8>> = None;
    let mut model_version: Option<String> = None;

    let body_str = String::from_utf8_lossy(&body);
    let parts: Vec<&str> = body_str.split(&format!("--{boundary}")).collect();

    for part in &parts {
        if part.is_empty() || part.trim() == "--" {
            continue;
        }

        // Find the Content-Disposition header to get field name
        if let Some(header_end) = part.find("\r\n\r\n") {
            let headers_section = &part[..header_end];
            let content = &part[header_end + 4..];

            // Trim trailing boundary markers
            let content = content.trim_end_matches("\r\n");
            let content = content.trim_end_matches("--");

            let field_name = headers_section
                .lines()
                .find(|l| l.contains("name=\""))
                .and_then(|l| {
                    let start = l.find("name=\"")? + 6;
                    let end = l[start..].find('"')?;
                    Some(&l[start..start + end])
                });

            match field_name {
                Some("image") => {
                    // Find the double newline after Content-Type (if present)
                    let body_start = if let Some(ct_pos) = headers_section.find("Content-Type:") {
                        // Content-Type header exists, find double newline after it
                        if let Some(double_nl) = headers_section[ct_pos..].find("\r\n\r\n") {
                            header_end + double_nl + 4 - (headers_section.len() - ct_pos)
                        } else {
                            header_end + 4
                        }
                    } else {
                        header_end + 4
                    };

                    let image_bytes = part[body_start..]
                        .trim_end_matches("\r\n")
                        .trim_end_matches("--")
                        .trim_end_matches("\r\n");
                    image_data = Some(image_bytes.as_bytes().to_vec());
                }
                Some("model") => {
                    model_version = Some(content.trim().to_string());
                }
                _ => {}
            }
        }
    }

    let image_data = match image_data {
        Some(data) if !data.is_empty() => data,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    success: false,
                    error: ErrorDetail {
                        code: "NO_IMAGE".into(),
                        message: "No image file provided".into(),
                    },
                }),
            )
                .into_response();
        }
    };

    let model = model_version.unwrap_or_else(|| {
        state.active_model.lock().map(|m| m.clone()).unwrap_or_default()
    });

    if let Err(e) = validate_model(&state, &model) {
        return e.into_response();
    }

    if let Err(e) = get_or_create_engine(&state, &model) {
        return e.into_response();
    }

    match run_ocr(&state, &image_data, &model) {
        Ok(resp) => (StatusCode::OK, Json(resp)).into_response(),
        Err(e) => e.into_response(),
    }
}

/// Handle JSON base64 request.
async fn handle_ocr_json(
    state: ApiState,
    body: &[u8],
) -> impl IntoResponse {
    let req: OcrJsonRequest = match serde_json::from_slice(body) {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    success: false,
                    error: ErrorDetail {
                        code: "INVALID_IMAGE".into(),
                        message: format!("Invalid JSON: {e}"),
                    },
                }),
            )
                .into_response();
        }
    };

    let image_data = match parse_base64_image(&req.image) {
        Ok(data) => data,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    success: false,
                    error: ErrorDetail {
                        code: "INVALID_IMAGE".into(),
                        message: e,
                    },
                }),
            )
                .into_response();
        }
    };

    let model = req.model.unwrap_or_else(|| {
        state.active_model.lock().map(|m| m.clone()).unwrap_or_default()
    });

    if let Err(e) = validate_model(&state, &model) {
        return e.into_response();
    }

    if let Err(e) = get_or_create_engine(&state, &model) {
        return e.into_response();
    }

    match run_ocr(&state, &image_data, &model) {
        Ok(resp) => (StatusCode::OK, Json(resp)).into_response(),
        Err(e) => e.into_response(),
    }
}

/// GET /api/v1/health
async fn handle_health(State(state): State<ApiState>) -> impl IntoResponse {
    let model_loaded = state
        .engine
        .lock()
        .map(|g| g.is_some())
        .unwrap_or(false);

    let active_model = state
        .active_model
        .lock()
        .map(|m| m.clone())
        .unwrap_or_default();

    (
        StatusCode::OK,
        Json(HealthResponse {
            status: "ok".into(),
            model_loaded,
            active_model,
            version: state.app_version.clone(),
        }),
    )
}

/// GET /api/v1/info
async fn handle_info(State(state): State<ApiState>) -> impl IntoResponse {
    let active_model = state
        .active_model
        .lock()
        .map(|m| m.clone())
        .unwrap_or_default();

    (
        StatusCode::OK,
        Json(InfoResponse {
            name: "LynxOCR API".into(),
            version: state.app_version.clone(),
            engine: "PaddleOCR ONNX".into(),
            available_models: vec![
                "ppocr-v4".into(),
                "ppocr-v5".into(),
                "ppocr-v6".into(),
            ],
            active_model,
            max_file_size_mb: state.max_file_size_mb,
        }),
    )
}