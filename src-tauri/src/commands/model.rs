use crate::engine::model_manager::{is_ppocr_installed_at, ModelInfo, ModelManager};
use crate::AppState;
use std::path::Path;
use tauri::Emitter;

/// List all available models
#[tauri::command]
pub async fn list_models(state: tauri::State<'_, AppState>) -> Result<Vec<ModelInfo>, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    let manager = ModelManager::new(config.model_path.clone());
    Ok(manager.list_models())
}

/// Download a specific model
#[tauri::command]
pub async fn download_specific_model(
    model_name: String,
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    let (model_path, download_urls) = {
        let config = state.config.lock().map_err(|e| e.to_string())?;
        (config.model_path.clone(), config.model_download_urls.clone())
    };

    let app_handle_clone = app_handle.clone();
    let model_path_clone = model_path.clone();

    let result = tokio::task::spawn_blocking(move || {
        let manager = ModelManager::new(model_path_clone);

        match model_name.as_str() {
            "ppocr-v4" | "ppocr-v5" | "ppocr-v6" => {
                let dir = Path::new(&model_path).join(&model_name);
                if is_ppocr_installed_at(&dir) {
                    return Ok("Model already installed".into());
                }
                manager
                    .download_ppocr(&model_name, &download_urls, &|progress| {
                        let _ = app_handle_clone.emit("model-download-progress", progress.clone());
                    })
                    .map_err(|e| e.to_string())?;
            }
            _ => return Err(format!("Unknown model: {model_name}")),
        }

        Ok(format!("Model {model_name} downloaded"))
    })
    .await
    .map_err(|e| format!("Download task failed: {}", e))?;

    result
}