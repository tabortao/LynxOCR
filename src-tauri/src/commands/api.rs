use crate::api::types::ServerStatus;
use crate::AppState;
use std::sync::Arc;

/// Start the HTTP API server.
#[tauri::command]
pub async fn api_start_server(state: tauri::State<'_, AppState>) -> Result<ServerStatus, String> {
    let (port, api_key, max_file_size_mb, model_path, mineru_token, mineru_base_url, mineru_output_format) = {
        let config = state.config.lock().map_err(|e| e.to_string())?;
        (
            config.api_server_port,
            config.api_key.clone(),
            config.max_file_size_mb,
            config.model_path.clone(),
            config.mineru_api_token.clone(),
            config.mineru_api_base_url.clone(),
            config.mineru_output_format.clone(),
        )
    };

    let app_version = env!("CARGO_PKG_VERSION").to_string();
    let engine_arc = state.ocr_engine.clone();
    let active_model = state.active_ocr_model.clone();

    let handle = crate::api::start_api_server(
        port,
        engine_arc,
        active_model,
        model_path,
        api_key,
        max_file_size_mb,
        app_version,
        mineru_token,
        mineru_base_url,
        mineru_output_format,
    )
    .await
    .map_err(|e| format!("Failed to start API server: {e}"))?;

    let mut server_handle = state
        .api_server_handle
        .lock()
        .map_err(|e| e.to_string())?;

    // Stop existing server if any
    if let Some(ref existing) = *server_handle {
        existing.shutdown();
    }

    *server_handle = Some(Arc::new(handle));

    log::info!("[api_start_server] server started on port {port}");
    Ok(ServerStatus {
        running: true,
        port,
    })
}

/// Stop the HTTP API server.
#[tauri::command]
pub async fn api_stop_server(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut server_handle = state
        .api_server_handle
        .lock()
        .map_err(|e| e.to_string())?;

    if let Some(ref handle) = *server_handle {
        handle.shutdown();
        log::info!("[api_stop_server] shutdown signal sent");
    }

    *server_handle = None;
    Ok(())
}

/// Get the current API server status.
#[tauri::command]
pub async fn api_get_server_status(
    state: tauri::State<'_, AppState>,
) -> Result<ServerStatus, String> {
    let server_handle = state
        .api_server_handle
        .lock()
        .map_err(|e| e.to_string())?;

    let config = state.config.lock().map_err(|e| e.to_string())?;
    let port = config.api_server_port;

    Ok(ServerStatus {
        running: server_handle.is_some(),
        port,
    })
}