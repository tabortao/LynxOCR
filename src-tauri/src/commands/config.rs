use crate::config::app_config::AppConfig;
use crate::AppState;
use tauri::State;

/// Get application config
#[tauri::command]
pub async fn get_app_config(state: State<'_, AppState>) -> Result<AppConfig, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    Ok(config.clone())
}

/// Update application config
#[tauri::command]
pub async fn set_app_config(
    new_config: AppConfig,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut config = state.config.lock().map_err(|e| e.to_string())?;
    *config = new_config;
    config.save().map_err(|e| e.to_string())?;
    log::info!("app config updated and saved");
    Ok(())
}