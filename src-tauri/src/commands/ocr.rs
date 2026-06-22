use crate::engine::mineru::MineruClient;
use crate::engine::model_manager::is_ppocr_installed_at;
use crate::engine::ocr::{types::OcrResult, OcrEngine};
use crate::AppState;
use std::path::Path;
use tauri::{Emitter, Manager};
use tauri_plugin_opener::OpenerExt;

/// Create a Pdfium instance, looking for pdfium.dll in multiple locations.
fn create_pdfium(app: &tauri::AppHandle) -> Result<pdfium_render::prelude::Pdfium, String> {
    let mut candidates: Vec<std::path::PathBuf> = Vec::new();

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            candidates.push(parent.join("pdfium.dll"));
            candidates.push(parent.join("bin").join("pdfium.dll"));
        }
    }

    if let Ok(resource_dir) = app.path().resource_dir() {
        candidates.push(resource_dir.join("pdfium.dll"));
        candidates.push(resource_dir.join("bin").join("pdfium.dll"));
    }

    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("pdfium.dll"));
    }

    for dll_path in &candidates {
        if dll_path.exists() {
            log::info!("[PDF] Loading pdfium.dll from: {}", dll_path.display());
            match pdfium_render::prelude::Pdfium::bind_to_library(dll_path) {
                Ok(bindings) => return Ok(pdfium_render::prelude::Pdfium::new(bindings)),
                Err(e) => {
                    log::warn!("[PDF] Found pdfium.dll at {} but failed to bind: {e}", dll_path.display());
                    continue;
                }
            }
        }
    }

    log::info!("[PDF] Falling back to system pdfium library");
    let bindings = pdfium_render::prelude::Pdfium::bind_to_library(
        pdfium_render::prelude::Pdfium::pdfium_platform_library_name(),
    )
    .map_err(|e| {
        let searched = candidates
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "Failed to find pdfium.dll. Searched: [{}]. System error: {e}",
            searched
        )
    })?;
    Ok(pdfium_render::prelude::Pdfium::new(bindings))
}

/// Get or create the OCR engine from AppState. Reuses cached engine if available.
fn get_or_create_engine<'a>(
    engine_arc: &'a std::sync::Arc<std::sync::Mutex<Option<OcrEngine>>>,
    model_dir: &std::path::Path,
) -> Result<std::sync::MutexGuard<'a, Option<OcrEngine>>, String> {
    let mut guard = engine_arc.lock().map_err(|e| e.to_string())?;
    if guard.is_none() {
        let eng = OcrEngine::new(model_dir)
            .map_err(|e| format!("Failed to create OCR engine: {e}"))?;
        *guard = Some(eng);
    }
    Ok(guard)
}

/// Get the number of pages in a PDF file.
#[tauri::command]
pub async fn pdf_get_page_count(
    app: tauri::AppHandle,
    pdf_path: String,
) -> Result<u32, String> {
    tokio::task::spawn_blocking(move || {
        let pdfium = create_pdfium(&app)?;
        let document = pdfium
            .load_pdf_from_file(&pdf_path, None)
            .map_err(|e| format!("Failed to open PDF: {e}"))?;
        Ok(document.pages().len() as u32)
    })
    .await
    .map_err(|e| format!("PDF task failed: {e}"))?
}

/// Render a specific page of a PDF to a PNG image.
#[tauri::command]
pub async fn pdf_render_page(
    app: tauri::AppHandle,
    pdf_path: String,
    page_index: u32,
    dpi: Option<f32>,
) -> Result<String, String> {
    let dpi = dpi.unwrap_or(200.0);
    let page_index = page_index as u16;

    tokio::task::spawn_blocking(move || {
        let pdfium = create_pdfium(&app)?;
        let document = pdfium
            .load_pdf_from_file(&pdf_path, None)
            .map_err(|e| format!("Failed to open PDF: {e}"))?;

        let pages = document.pages();
        if page_index as usize >= pages.len() as usize {
            return Err(format!(
                "Page index {} out of range (total: {})",
                page_index,
                pages.len()
            ));
        }

        let page = pages
            .get(page_index)
            .map_err(|e| format!("Failed to get page: {e}"))?;

        let scale = dpi / 72.0;
        let target_width = (page.width().value as f32 * scale) as i32;
        let target_height = (page.height().value as f32 * scale) as i32;

        let config = pdfium_render::prelude::PdfRenderConfig::new()
            .set_target_width(target_width)
            .set_target_height(target_height);

        let bitmap = page
            .render_with_config(&config)
            .map_err(|e| format!("Failed to render PDF page: {e}"))?;

        let temp_dir = std::env::temp_dir();
        let png_path = temp_dir.join(format!("lynxocr_pdf_page_{}.png", page_index));
        bitmap
            .as_image()
            .save(&png_path)
            .map_err(|e| format!("Failed to save rendered page: {e}"))?;

        Ok(png_path.to_string_lossy().to_string())
    })
    .await
    .map_err(|e| format!("PDF render task failed: {e}"))?
}

/// Detect image extension from magic bytes.
fn detect_image_ext(data: &[u8]) -> &'static str {
    if data.len() < 4 {
        return "bin";
    }
    if &data[0..4] == b"\x89PNG" {
        return "png";
    }
    if &data[0..2] == b"\xff\xd8" {
        return "jpg";
    }
    if data.len() >= 4 && &data[0..4] == b"RIFF" && data.len() >= 12 && &data[8..12] == b"WEBP" {
        return "webp";
    }
    if &data[0..2] == b"BM" {
        return "bmp";
    }
    if data.len() >= 4 && &data[0..4] == b"%PDF" {
        return "pdf";
    }
    "bin"
}

/// Run OCR on a PDF file by rendering each page and recognizing text.
#[tauri::command]
pub async fn ocr_recognize_pdf(
    app: tauri::AppHandle,
    pdf_path: String,
    model_version: String,
    dpi: Option<f32>,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    // Route MinerU requests to MinerU client (handles PDF natively)
    if model_version == "mineru" {
        let result = ocr_recognize_mineru_impl(&pdf_path, &state).await?;
        return Ok(vec![serde_json::json!({
            "pageIndex": 0,
            "imagePath": "",
            "ocrResult": result,
        })]);
    }

    let dpi = dpi.unwrap_or(200.0);
    let model_path = {
        let config = state.config.lock().map_err(|e| e.to_string())?;
        config.model_path.clone()
    };

    let model_dir = Path::new(&model_path).join(&model_version);
    if !is_ppocr_installed_at(&model_dir) {
        return Err(format!(
            "OCR model {} is not installed. Please download it from Model Settings.",
            model_version
        ));
    }

    let engine_arc = state.ocr_engine.clone();

    tokio::task::spawn_blocking(move || {
        let pdfium = create_pdfium(&app)?;
        let document = pdfium
            .load_pdf_from_file(&pdf_path, None)
            .map_err(|e| format!("Failed to open PDF: {e}"))?;

        let page_count = document.pages().len() as usize;
        let mut results = Vec::with_capacity(page_count);

        let mut guard = engine_arc.lock().map_err(|e| e.to_string())?;
        if guard.is_none() {
            let eng = OcrEngine::new(&model_dir).map_err(|e| e.to_string())?;
            *guard = Some(eng);
        }
        let eng = guard.as_mut().unwrap();

        for page_idx in 0..page_count {
            let page = document
                .pages()
                .get(page_idx as u16)
                .map_err(|e| format!("Failed to get page {page_idx}: {e}"))?;

            let scale = dpi / 72.0;
            let target_width = (page.width().value as f32 * scale) as i32;
            let target_height = (page.height().value as f32 * scale) as i32;

            let config = pdfium_render::prelude::PdfRenderConfig::new()
                .set_target_width(target_width)
                .set_target_height(target_height);

            let bitmap = page
                .render_with_config(&config)
                .map_err(|e| format!("Failed to render page {page_idx}: {e}"))?;

            let img = bitmap.as_image();
            // Clone for OCR (which takes ownership) and for saving
            let ocr_img = img.clone();
            let ocr_result = eng
                .recognize_from_image(ocr_img)
                .map_err(|e| e.to_string())?;

            // Save rendered page as temp PNG for preview
            let temp_dir = std::env::temp_dir();
            let png_path = temp_dir.join(format!("lynxocr_pdf_page_{page_idx}.png"));
            let _ = img.save(&png_path);
            let png_path_str = png_path.to_string_lossy().to_string();

            results.push(serde_json::json!({
                "pageIndex": page_idx,
                "imagePath": png_path_str,
                "ocrResult": ocr_result,
            }));
        }

        Ok(results)
    })
    .await
    .map_err(|e| format!("PDF OCR task failed: {e}"))?
}

/// Run OCR on an image file using the specified model version.
/// Engine is cached for reuse; caller should call `ocr_release` when done
/// to free ONNX Runtime memory.
#[tauri::command]
pub async fn ocr_recognize(
    image_path: String,
    model_version: String,
    state: tauri::State<'_, AppState>,
) -> Result<OcrResult, String> {
    // Route MinerU requests to MinerU client
    if model_version == "mineru" {
        return ocr_recognize_mineru_impl(&image_path, &state).await;
    }

    let model_path = {
        let config = state.config.lock().map_err(|e| e.to_string())?;
        config.model_path.clone()
    };

    let model_dir = Path::new(&model_path).join(&model_version);
    if !is_ppocr_installed_at(&model_dir) {
        return Err(format!(
            "OCR model {} is not installed. Please download it from Model Settings.",
            model_version
        ));
    }

    let engine_arc = state.ocr_engine.clone();

    tokio::task::spawn_blocking(move || {
        let mut guard = get_or_create_engine(&engine_arc, &model_dir)?;
        let eng = guard.as_mut().unwrap();
        eng.recognize_from_path(Path::new(&image_path))
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("OCR task failed: {e}"))?
}

/// Run OCR on raw image bytes (PNG/JPEG/etc.) using the specified model version.
#[tauri::command]
pub async fn ocr_recognize_bytes(
    image_data: Vec<u8>,
    model_version: String,
    state: tauri::State<'_, AppState>,
) -> Result<OcrResult, String> {
    // Route MinerU requests to MinerU client
    if model_version == "mineru" {
        let ext = detect_image_ext(&image_data);
        let temp_path = std::env::temp_dir()
            .join(format!("lynxocr_mineru_bytes.{}", ext));
        std::fs::write(&temp_path, &image_data)
            .map_err(|e| format!("Failed to write temp file: {e}"))?;
        let result = ocr_recognize_mineru_impl(
            &temp_path.to_string_lossy(),
            &state,
        ).await;
        let _ = std::fs::remove_file(&temp_path);
        return result;
    }

    let model_path = {
        let config = state.config.lock().map_err(|e| e.to_string())?;
        config.model_path.clone()
    };

    let model_dir = Path::new(&model_path).join(&model_version);
    if !is_ppocr_installed_at(&model_dir) {
        return Err(format!(
            "OCR model {} is not installed. Please download it from Model Settings.",
            model_version
        ));
    }

    let engine_arc = state.ocr_engine.clone();

    tokio::task::spawn_blocking(move || {
        let mut guard = get_or_create_engine(&engine_arc, &model_dir)?;
        let eng = guard.as_mut().unwrap();
        eng.recognize_from_bytes(&image_data)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("OCR task failed: {e}"))?
}

/// Capture all monitors and stitch them into a single screenshot.
#[tauri::command]
pub async fn capture_all_monitors() -> Result<serde_json::Value, String> {
    tokio::task::spawn_blocking(move || capture_all_monitors_inner())
        .await
        .map_err(|e| format!("Screenshot capture failed: {e}"))?
}

/// Run OCR on a cropped region of a captured screenshot.
#[tauri::command]
pub async fn ocr_screenshot_region(
    image_path: String,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    model_version: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    // Route MinerU requests to MinerU client
    if model_version == "mineru" {
        // Crop the region first, then send to MinerU
        let cropped_path = tokio::task::spawn_blocking(move || {
            let img = image::open(&image_path)
                .map_err(|e| format!("Failed to open screenshot: {e}"))?;
            let cropped = img.crop_imm(x, y, width, height);
            let cropped_path = std::env::temp_dir().join("lynxocr_screenshot_crop.png");
            cropped
                .save(&cropped_path)
                .map_err(|e| format!("Failed to save cropped screenshot: {e}"))?;
            Ok::<_, String>(cropped_path.to_string_lossy().to_string())
        })
        .await
        .map_err(|e| format!("Screenshot crop failed: {e}"))??;

        let result = ocr_recognize_mineru_impl(&cropped_path, &state).await?;
        return Ok(serde_json::json!({
            "ocrResult": result,
            "croppedImagePath": cropped_path,
        }));
    }

    let model_path = {
        let config = state.config.lock().map_err(|e| e.to_string())?;
        config.model_path.clone()
    };

    let model_dir = Path::new(&model_path).join(&model_version);
    if !is_ppocr_installed_at(&model_dir) {
        return Err(format!(
            "OCR model {} is not installed. Please download it from Model Settings.",
            model_version
        ));
    }

    let engine_arc = state.ocr_engine.clone();

    tokio::task::spawn_blocking(move || {
        let img = image::open(&image_path)
            .map_err(|e| format!("Failed to open screenshot: {e}"))?;

        // Crop the selected region (crop_imm creates a new view, no copy of full image)
        let cropped = img.crop_imm(x, y, width, height);

        // Save cropped region to temp file for preview
        let cropped_path = std::env::temp_dir().join("lynxocr_screenshot_crop.png");
        cropped
            .save(&cropped_path)
            .map_err(|e| format!("Failed to save cropped screenshot: {e}"))?;
        let cropped_path_str = cropped_path.to_string_lossy().to_string();

        // Drop the full image now to free memory — only cropped region is needed for OCR
        drop(img);

        let mut guard = engine_arc.lock().map_err(|e| e.to_string())?;
        if guard.is_none() {
            let eng = OcrEngine::new(&model_dir).map_err(|e| e.to_string())?;
            *guard = Some(eng);
        }
        let eng = guard.as_mut().unwrap();
        let result = eng
            .recognize_from_image(cropped)
            .map_err(|e| e.to_string())?;

        Ok(serde_json::json!({
            "ocrResult": result,
            "croppedImagePath": cropped_path_str,
        }))
    })
    .await
    .map_err(|e| format!("Screenshot OCR failed: {e}"))?
}

/// Copy text to the system clipboard.
#[tauri::command]
pub async fn copy_text_to_clipboard(text: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let mut clipboard = arboard::Clipboard::new()
            .map_err(|e| format!("Failed to open clipboard: {e}"))?;
        clipboard
            .set_text(&text)
            .map_err(|e| format!("Failed to set clipboard text: {e}"))
    })
    .await
    .map_err(|e| format!("Clipboard task failed: {e}"))?
}

/// Get the currently active OCR model version.
#[tauri::command]
pub async fn ocr_get_active_model(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let active = state.active_ocr_model.lock().map_err(|e| e.to_string())?;
    Ok(active.clone())
}

/// Set the active OCR model version (saves to config, releases cached engine).
#[tauri::command]
pub async fn ocr_set_active_model(
    model_name: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let valid_models = ["ppocr-v4", "ppocr-v5", "ppocr-v6", "mineru"];
    if !valid_models.contains(&model_name.as_str()) {
        return Err(format!("Unknown OCR model: {model_name}"));
    }

    {
        let mut config = state.config.lock().map_err(|e| e.to_string())?;
        config.active_ocr_model = model_name.clone();
        crate::config::app_config::AppConfig::save(&config).map_err(|e| e.to_string())?;
    }

    {
        let mut active = state.active_ocr_model.lock().map_err(|e| e.to_string())?;
        *active = model_name.clone();
    }

    // Release engine — will be re-created with new model on next call
    {
        let mut engine = state.ocr_engine.lock().map_err(|e| e.to_string())?;
        *engine = None;
    }

    log::info!("[ocr_set_active_model] switched to {model_name}, engine released");
    Ok(())
}

/// Release the cached OCR engine to free ONNX Runtime memory.
/// Should be called after OCR operations are complete to keep memory low.
#[tauri::command]
pub async fn ocr_release(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut engine = state.ocr_engine.lock().map_err(|e| e.to_string())?;
    *engine = None;
    log::info!("[ocr_release] engine released — ONNX Runtime memory freed");
    Ok(())
}

/// Run OCR via MinerU cloud API.
/// Uses flash-extract (no token) or extract (with token) mode.
#[tauri::command]
pub async fn ocr_recognize_mineru(
    image_path: String,
    _output_format: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<OcrResult, String> {
    ocr_recognize_mineru_impl(&image_path, &state).await
}

/// Shared implementation for MinerU OCR (used by both ocr_recognize and ocr_recognize_mineru).
async fn ocr_recognize_mineru_impl(
    image_path: &str,
    state: &tauri::State<'_, AppState>,
) -> Result<OcrResult, String> {
    let (token, base_url, default_format) = {
        let config = state.config.lock().map_err(|e| e.to_string())?;
        (
            config.mineru_api_token.clone(),
            config.mineru_api_base_url.clone(),
            config.mineru_output_format.clone(),
        )
    };

    let image_path = image_path.to_string();

    tokio::task::spawn_blocking(move || {
        let client = MineruClient::new(token, base_url);

        let result = if client.has_token() {
            client.extract(Path::new(&image_path), &default_format)
        } else {
            client.flash_extract(Path::new(&image_path))
        }
        .map_err(|e| e.to_string())?;

        Ok(OcrResult {
            text_blocks: vec![crate::engine::ocr::types::TextBlockInfo {
                text: result.content,
                confidence: 1.0,
                box_points: vec![],
            }],
            total_time_ms: result.total_time_ms,
            format: Some(result.format),
        })
    })
    .await
    .map_err(|e| format!("MinerU OCR task failed: {e}"))?
}

/// Start screenshot selection by creating a transparent fullscreen window.
#[tauri::command]
pub async fn start_screenshot_selection(
    app: tauri::AppHandle,
    model_version: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let capture_result = capture_all_monitors_inner()?;

    let bbox = capture_result
        .get("boundingBox")
        .ok_or("Missing boundingBox")?;
    let min_x = bbox["minX"].as_i64().ok_or("Missing minX")? as i32;
    let min_y = bbox["minY"].as_i64().ok_or("Missing minY")? as i32;
    let bbox_width = bbox["width"].as_u64().ok_or("Missing bbox width")? as u32;
    let bbox_height = bbox["height"].as_u64().ok_or("Missing bbox height")? as u32;

    {
        let mut pending = state.pending_screenshot.lock().map_err(|e| e.to_string())?;
        *pending = Some(serde_json::json!({
            "imagePath": capture_result["imagePath"],
            "width": capture_result["width"],
            "height": capture_result["height"],
            "modelVersion": model_version,
        }));
    }

    if let Some(existing) = app.get_webview_window("screenshot") {
        let _ = existing.close();
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    let _window = tauri::WebviewWindowBuilder::new(&app, "screenshot", tauri::WebviewUrl::App("screenshot.html".into()))
        .title("LynxOCR Screenshot")
        .inner_size(bbox_width as f64, bbox_height as f64)
        .position(min_x as f64, min_y as f64)
        .transparent(true)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .shadow(false)
        .build()
        .map_err(|e| format!("Failed to create screenshot window: {e}"))?;

    log::info!(
        "[start_screenshot_selection] window created, size={}x{}, pos=({},{})",
        bbox_width, bbox_height, min_x, min_y
    );

    Ok(capture_result)
}

/// Get the pending screenshot data.
#[tauri::command]
pub async fn get_screenshot_data(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mut pending = state.pending_screenshot.lock().map_err(|e| e.to_string())?;
    pending.take().ok_or("No pending screenshot data".to_string())
}

/// Close the screenshot window.
#[tauri::command]
pub async fn close_screenshot_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("screenshot") {
        window
            .close()
            .map_err(|e| format!("Failed to close screenshot window: {e}"))?;
        log::info!("[close_screenshot_window] screenshot window closed");
    }
    Ok(())
}

/// Called from the screenshot window after OCR completes.
#[tauri::command]
pub async fn screenshot_ocr_done(
    app: tauri::AppHandle,
    text: String,
    time_ms: u64,
    cropped_image_path: Option<String>,
    ocr_result: Option<serde_json::Value>,
) -> Result<(), String> {
    let main_visible = app
        .get_webview_window("main")
        .map(|w| w.is_visible().unwrap_or(false))
        .unwrap_or(false);

    if let Some(main_window) = app.get_webview_window("main") {
        main_window
            .emit(
                "screenshot-ocr-result",
                serde_json::json!({
                    "text": text,
                    "timeMs": time_ms,
                    "croppedImagePath": cropped_image_path,
                    "ocrResult": ocr_result,
                }),
            )
            .map_err(|e| format!("Failed to emit screenshot-ocr-result: {e}"))?;
    }

    if !main_visible && !text.is_empty() {
        show_desktop_toast(&app, "文本复制成功");
    }

    Ok(())
}

/// Show a desktop toast notification.
fn show_desktop_toast(app: &tauri::AppHandle, message: &str) {
    let html_content = format!(
        r#"<!DOCTYPE html>
<html>
<head><meta charset="utf-8"><style>
* {{ margin: 0; padding: 0; box-sizing: border-box; }}
body {{
  background: transparent;
  overflow: hidden;
  -webkit-user-select: none;
  user-select: none;
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100vh;
}}
.toast {{
  background: #dcfce7;
  color: #166534;
  padding: 10px 24px;
  border-radius: 8px;
  font-size: 14px;
  font-weight: 500;
  font-family: system-ui, -apple-system, sans-serif;
  box-shadow: 0 4px 16px rgba(0,0,0,0.12);
  border: 1px solid #bbf7d0;
  text-align: center;
  white-space: nowrap;
  animation: fadeIn 0.3s ease;
}}
@keyframes fadeIn {{ from {{ opacity: 0; }} to {{ opacity: 1; }} }}
</style></head>
<body><div class="toast">{message}</div></body></html>"#
    );

    let temp_html = std::env::temp_dir().join("lynxocr_toast.html");
    if std::fs::write(&temp_html, &html_content).is_ok() {
        let monitors = xcap::Monitor::all().ok();
        let (screen_w, screen_h) = if let Some(ref mons) = monitors {
            if let Some(primary) = mons.first() {
                (primary.width() as f64, primary.height() as f64)
            } else {
                (1920.0, 1080.0)
            }
        } else {
            (1920.0, 1080.0)
        };

        let toast_w = 300.0;
        let toast_h = 50.0;
        let pos_x = (screen_w - toast_w) / 2.0;
        let pos_y = screen_h * 0.18;

        let url = tauri::WebviewUrl::External(
            format!("file:///{}", temp_html.to_string_lossy())
                .parse()
                .unwrap(),
        );

        if let Ok(_toast_window) = tauri::WebviewWindowBuilder::new(app, "toast", url)
            .title("LynxOCR Toast")
            .inner_size(toast_w, toast_h)
            .position(pos_x, pos_y)
            .transparent(true)
            .decorations(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .shadow(false)
            .focused(false)
            .build()
        {
            log::info!("[show_desktop_toast] toast window created");

            let app_clone = app.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(2300));
                if let Some(window) = app_clone.get_webview_window("toast") {
                    let _ = window.close();
                }
            });
        }
    }
}

/// Inner implementation of capture_all_monitors.
fn capture_all_monitors_inner() -> Result<serde_json::Value, String> {
    let monitors =
        xcap::Monitor::all().map_err(|e| format!("Failed to enumerate monitors: {e}"))?;

    if monitors.is_empty() {
        return Err("No monitors found".to_string());
    }

    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;

    for m in &monitors {
        min_x = min_x.min(m.x());
        min_y = min_y.min(m.y());
        max_x = max_x.max(m.x() + m.width() as i32);
        max_y = max_y.max(m.y() + m.height() as i32);
    }

    let bbox_width = (max_x - min_x) as u32;
    let bbox_height = (max_y - min_y) as u32;

    let captures: Vec<_> = monitors
        .iter()
        .map(|m| (m.x(), m.y(), m.capture_image()))
        .collect();

    let mut canvas = image::RgbaImage::new(bbox_width, bbox_height);

    for (x, y, cap) in captures {
        let cap = cap.map_err(|e| format!("Failed to capture monitor: {e}"))?;
        let offset_x = (x - min_x) as u32;
        let offset_y = (y - min_y) as u32;
        image::imageops::overlay(&mut canvas, &cap, offset_x as i64, offset_y as i64);
    }

    let temp_path = std::env::temp_dir().join("lynxocr_screenshot_all.png");
    canvas
        .save(&temp_path)
        .map_err(|e| format!("Failed to save screenshot: {e}"))?;

    Ok(serde_json::json!({
        "imagePath": temp_path.to_string_lossy(),
        "width": bbox_width,
        "height": bbox_height,
        "boundingBox": {
            "minX": min_x,
            "minY": min_y,
            "maxX": max_x,
            "maxY": max_y,
            "width": bbox_width,
            "height": bbox_height,
        }
    }))
}

// ============================================================================
// Utility commands
// ============================================================================

/// Write text content to a file.
#[tauri::command]
pub fn write_text_file(path: String, content: String) -> Result<(), String> {
    std::fs::write(&path, &content).map_err(|e| format!("Failed to write file: {e}"))
}

/// Write base64-encoded binary content to a file.
#[tauri::command]
pub fn write_binary_file(path: String, base64_content: String) -> Result<(), String> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&base64_content)
        .map_err(|e| format!("Failed to decode base64: {e}"))?;
    std::fs::write(&path, &bytes).map_err(|e| format!("Failed to write file: {e}"))
}

/// Open a file with the system default application.
#[tauri::command]
pub fn open_file_with_system(app: tauri::AppHandle, path: String) -> Result<(), String> {
    app.opener()
        .open_path(&path, None::<&str>)
        .map_err(|e| format!("Failed to open file: {e}"))
}