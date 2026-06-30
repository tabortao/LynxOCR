mod api;
mod commands;
mod config;
mod engine;
mod errors;

#[cfg(test)]
mod tests;

use config::app_config::AppConfig;
use engine::mineru::MineruClient;
use engine::ocr::OcrEngine;
use std::sync::{Arc, Mutex};
use tauri::webview::PageLoadEvent;
use tauri::{Emitter, Manager, WindowEvent};
use tauri::tray::{TrayIconBuilder, MouseButton, MouseButtonState, TrayIconEvent};
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri_plugin_log::{Target, TargetKind};
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

pub struct AppState {
    pub config: Mutex<AppConfig>,
    pub active_ocr_model: Arc<Mutex<String>>,
    /// OCR engine instance (session reuse for performance).
    pub ocr_engine: Arc<Mutex<Option<OcrEngine>>>,
    /// Pending screenshot data for the screenshot selection window.
    pub pending_screenshot: Arc<Mutex<Option<serde_json::Value>>>,
    /// Handle for the HTTP API server (if running).
    pub api_server_handle: Arc<Mutex<Option<Arc<api::ServerHandle>>>>,
    /// MinerU API client (lazily created).
    pub mineru_client: Arc<Mutex<Option<MineruClient>>>,
}

/// Parse a shortcut string like "Ctrl+Shift+O" into (Modifiers, Code).
fn parse_shortcut_string(s: &str) -> (Modifiers, Code) {
    let parts: Vec<&str> = s.split('+').map(|p| p.trim()).collect();
    let mut modifiers = Modifiers::empty();

    for &part in &parts[..parts.len().saturating_sub(1)] {
        match part.to_lowercase().as_str() {
            "ctrl" | "control" => modifiers |= Modifiers::CONTROL,
            "shift" => modifiers |= Modifiers::SHIFT,
            "alt" => modifiers |= Modifiers::ALT,
            "super" | "cmd" | "command" | "win" => modifiers |= Modifiers::SUPER,
            _ => {}
        }
    }

    let key = parts.last().map(|p| p.to_uppercase()).unwrap_or_default();
    let code = match key.as_str() {
        "A" => Code::KeyA, "B" => Code::KeyB, "C" => Code::KeyC, "D" => Code::KeyD,
        "E" => Code::KeyE, "F" => Code::KeyF, "G" => Code::KeyG, "H" => Code::KeyH,
        "I" => Code::KeyI, "J" => Code::KeyJ, "K" => Code::KeyK, "L" => Code::KeyL,
        "M" => Code::KeyM, "N" => Code::KeyN, "O" => Code::KeyO, "P" => Code::KeyP,
        "Q" => Code::KeyQ, "R" => Code::KeyR, "S" => Code::KeyS, "T" => Code::KeyT,
        "U" => Code::KeyU, "V" => Code::KeyV, "W" => Code::KeyW, "X" => Code::KeyX,
        "Y" => Code::KeyY, "Z" => Code::KeyZ,
        "0" => Code::Digit0, "1" => Code::Digit1, "2" => Code::Digit2,
        "3" => Code::Digit3, "4" => Code::Digit4, "5" => Code::Digit5,
        "6" => Code::Digit6, "7" => Code::Digit7, "8" => Code::Digit8,
        "9" => Code::Digit9,
        "F1" => Code::F1, "F2" => Code::F2, "F3" => Code::F3, "F4" => Code::F4,
        "F5" => Code::F5, "F6" => Code::F6, "F7" => Code::F7, "F8" => Code::F8,
        "F9" => Code::F9, "F10" => Code::F10, "F11" => Code::F11, "F12" => Code::F12,
        "SPACE" => Code::Space,
        "TAB" => Code::Tab,
        "ENTER" | "RETURN" => Code::Enter,
        "ESCAPE" | "ESC" => Code::Escape,
        "BACKSPACE" => Code::Backspace,
        _ => {
            log::warn!("[parse_shortcut_string] Unknown shortcut key: {key}, falling back to KeyO");
            Code::KeyO
        }
    };

    (modifiers, code)
}

fn external_navigation_plugin<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::<R>::new("external-navigation")
        .on_navigation(|webview, url| {
            let is_internal_host = matches!(
                url.host_str(),
                Some("localhost") | Some("127.0.0.1") | Some("tauri.localhost") | Some("::1")
            );

            let is_internal = url.scheme() == "tauri" || is_internal_host;

            if is_internal {
                return true;
            }

            let is_external_link = matches!(url.scheme(), "http" | "https" | "mailto" | "tel");

            if is_external_link {
                log::info!("opening external link in system browser: {}", url);
                let _ = webview.opener().open_url(url.as_str(), None::<&str>);
                return false;
            }

            true
        })
        .build()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let initial_config = AppConfig::load();

    let active_ocr_model = initial_config.active_ocr_model.clone();
    let screenshot_shortcut = if initial_config.ocr_screenshot_shortcut.is_empty() {
        "Ctrl+Shift+O".to_string()
    } else {
        initial_config.ocr_screenshot_shortcut.clone()
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(
            tauri_plugin_log::Builder::new()
                .targets([
                    Target::new(TargetKind::Stdout),
                    Target::new(TargetKind::LogDir { file_name: None }),
                    Target::new(TargetKind::Webview),
                ])
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
                let _ = window.unminimize();
            }
        }))
        .plugin(external_navigation_plugin())
        .manage(AppState {
            config: Mutex::new(initial_config),
            active_ocr_model: Arc::new(Mutex::new(active_ocr_model)),
            ocr_engine: Arc::new(Mutex::new(None)),
            pending_screenshot: Arc::new(Mutex::new(None)),
            api_server_handle: Arc::new(Mutex::new(None)),
            mineru_client: Arc::new(Mutex::new(None)),
        })
        .invoke_handler(tauri::generate_handler![
            // Model commands
            commands::model::list_models,
            commands::model::download_specific_model,
            // Config commands
            commands::config::get_app_config,
            commands::config::set_app_config,
            // OCR commands
            commands::ocr::ocr_recognize,
            commands::ocr::ocr_recognize_bytes,
            commands::ocr::capture_all_monitors,
            commands::ocr::ocr_screenshot_region,
            commands::ocr::start_screenshot_selection,
            commands::ocr::get_screenshot_data,
            commands::ocr::close_screenshot_window,
            commands::ocr::screenshot_ocr_done,
            commands::ocr::copy_text_to_clipboard,
            commands::ocr::ocr_get_active_model,
            commands::ocr::ocr_set_active_model,
            commands::ocr::ocr_release,
            commands::ocr::ocr_recognize_mineru,
            commands::ocr::pdf_get_page_count,
            commands::ocr::pdf_render_page,
            commands::ocr::ocr_recognize_pdf,
            // Utility commands
            commands::ocr::write_text_file,
            commands::ocr::write_binary_file,
            commands::ocr::open_file_with_system,
            commands::ocr::open_app_url,
            // API server commands
            commands::api::api_start_server,
            commands::api::api_stop_server,
            commands::api::api_get_server_status,
        ])
        .setup(move |app| {
            // System tray
            let show = MenuItemBuilder::with_id("show", "显示 LynxOCR")
                .build(app)?;
            let quit = MenuItemBuilder::with_id("quit", "退出")
                .build(app)?;
            let menu = MenuBuilder::new(app)
                .items(&[&show, &quit])
                .build()?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .tooltip("LynxOCR")
                .on_menu_event(|app, event| {
                    match event.id().as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            // Register global shortcut for screenshot OCR
            {
                let app_handle = app.handle().clone();
                let (modifiers, code) = parse_shortcut_string(&screenshot_shortcut);
                let shortcut = Shortcut::new(Some(modifiers), code);

                app.global_shortcut().on_shortcut(shortcut, move |_app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        log::info!("[global-shortcut] screenshot OCR shortcut pressed");
                        let app = app_handle.clone();
                        tauri::async_runtime::spawn(async move {
                            if let Some(window) = app.get_webview_window("main") {
                                if window.is_visible().unwrap_or(false) {
                                    let _ = window.set_focus();
                                }
                            }
                            let _ = app.emit("trigger-screenshot-ocr", ());
                        });
                    }
                })?;

                log::info!("[global-shortcut] registered shortcut: {}", screenshot_shortcut);
            }

            // Auto-start API server if configured
            {
                let state = app.state::<AppState>();
                let config = state.config.lock().unwrap();
                let auto_start = config.api_server_auto_start;
                let port = config.api_server_port;
                let api_key = config.api_key.clone();
                let max_file_size_mb = config.max_file_size_mb;
                let model_path = config.model_path.clone();
                let mineru_token = config.mineru_api_token.clone();
                let mineru_base_url = config.mineru_api_base_url.clone();
                let mineru_output_format = config.mineru_output_format.clone();
                let app_version = env!("CARGO_PKG_VERSION").to_string();
                drop(config);

                if auto_start {
                    let engine_arc = state.ocr_engine.clone();
                    let active_model = state.active_ocr_model.clone();
                    let app_handle = app.handle().clone();

                    tauri::async_runtime::spawn(async move {
                        match api::start_api_server(
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
                        {
                            Ok(handle) => {
                                let state = app_handle.state::<AppState>();
                                let mut server_handle = state.api_server_handle.lock().unwrap();
                                *server_handle = Some(Arc::new(handle));
                                log::info!("[auto-start] API server started on port {port}");
                            }
                            Err(e) => {
                                log::error!("[auto-start] failed to start API server: {e}");
                            }
                        }
                    });
                }
            }

            Ok(())
        })
        .on_page_load(|webview, payload| {
            if webview.label() == "main" && matches!(payload.event(), PageLoadEvent::Finished) {
                log::info!("LynxOCR main webview loaded");
                let _ = webview.window().show();
            }
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                    return;
                }
            }
            if let WindowEvent::DragDrop(taura_drop_event) = event {
                use tauri::DragDropEvent;
                match taura_drop_event {
                    DragDropEvent::Drop { paths, .. } => {
                        let path_list: Vec<String> = paths
                            .iter()
                            .map(|p| p.to_string_lossy().to_string())
                            .collect();
                        let _ = window.emit(
                            "tauri://file-drop",
                            serde_json::to_string(&path_list).unwrap_or_default(),
                        );
                        log::info!("Files dropped: {:?}", path_list);
                    }
                    DragDropEvent::Enter { .. } => {
                        let _ = window.emit("tauri://file-drop-hover", true);
                    }
                    DragDropEvent::Leave => {
                        let _ = window.emit("tauri://file-drop-hover", false);
                    }
                    _ => {}
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}