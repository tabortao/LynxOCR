//! Model manager — handles model download and cache management.

use crate::errors::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::Path;

/// Download a file from URL to destination, reporting progress.
fn download_file(
    url: &str,
    dest: &Path,
    model_name: &str,
    on_progress: &dyn Fn(DownloadProgress),
) -> AppResult<()> {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_secs(15))
        .timeout_read(std::time::Duration::from_secs(120))
        .timeout_write(std::time::Duration::from_secs(30))
        .redirects(5)
        .build();

    let resp = agent
        .get(url)
        .call()
        .map_err(|e| AppError::ModelDownload(format!("HTTP request failed: {}", e)))?;

    let status = resp.status();
    if status < 200 || status >= 300 {
        return Err(AppError::ModelDownload(format!(
            "Server returned HTTP {} for URL: {}. The model file may not exist on the server yet.",
            status, url
        )));
    }

    let total = resp
        .header("Content-Length")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);

    let mut reader = resp.into_reader();
    let mut buf = [0u8; 8192];
    let mut downloaded: u64 = 0;
    let mut file = std::fs::File::create(dest)
        .map_err(|e| AppError::ModelDownload(format!("Create file failed: {}", e)))?;

    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|e| AppError::ModelDownload(format!("Download interrupted: {}", e)))?;
        if n == 0 {
            break;
        }
        std::io::Write::write_all(&mut file, &buf[..n])
            .map_err(|e| AppError::ModelDownload(format!("Write file failed: {}", e)))?;
        downloaded += n as u64;

        if total > 0 {
            let percentage = (downloaded as f64 / total as f64) * 100.0;
            on_progress(DownloadProgress {
                model_name: model_name.into(),
                downloaded,
                total,
                percentage,
                stage: "Downloading...".to_string(),
            });
        }
    }

    Ok(())
}

/// Model information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub name: String,
    pub display_name: String,
    pub size: String,
    pub installed: bool,
    pub path: Option<String>,
}

/// Model download progress
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub model_name: String,
    pub downloaded: u64,
    pub total: u64,
    pub percentage: f64,
    pub stage: String,
}

/// Check if PaddleOCR models are installed at the given directory.
/// Requires `detection.onnx` and `recognition.onnx`.
pub fn is_ppocr_installed_at(dir: &Path) -> bool {
    dir.join("detection.onnx").exists() && dir.join("recognition.onnx").exists()
}

/// Model manager
pub struct ModelManager {
    models_dir: String,
}

impl ModelManager {
    pub fn new(models_dir: String) -> Self {
        Self { models_dir }
    }

    /// List all available models
    pub fn list_models(&self) -> Vec<ModelInfo> {
        let ppocr_v4_path = Path::new(&self.models_dir).join("ppocr-v4");
        let ppocr_v4_installed = is_ppocr_installed_at(&ppocr_v4_path);

        let ppocr_v5_path = Path::new(&self.models_dir).join("ppocr-v5");
        let ppocr_v5_installed = is_ppocr_installed_at(&ppocr_v5_path);

        let ppocr_v6_path = Path::new(&self.models_dir).join("ppocr-v6");
        let ppocr_v6_installed = is_ppocr_installed_at(&ppocr_v6_path);

        vec![
            ModelInfo {
                name: "ppocr-v4".into(),
                display_name: "PaddleOCR V4".into(),
                size: "~20MB".into(),
                installed: ppocr_v4_installed,
                path: if ppocr_v4_installed {
                    Some(ppocr_v4_path.to_string_lossy().to_string())
                } else {
                    None
                },
            },
            ModelInfo {
                name: "ppocr-v5".into(),
                display_name: "PaddleOCR V5".into(),
                size: "~20MB".into(),
                installed: ppocr_v5_installed,
                path: if ppocr_v5_installed {
                    Some(ppocr_v5_path.to_string_lossy().to_string())
                } else {
                    None
                },
            },
            ModelInfo {
                name: "ppocr-v6".into(),
                display_name: "PaddleOCR V6".into(),
                size: "~20MB".into(),
                installed: ppocr_v6_installed,
                path: if ppocr_v6_installed {
                    Some(ppocr_v6_path.to_string_lossy().to_string())
                } else {
                    None
                },
            },
        ]
    }

    /// Download PaddleOCR ONNX model.
    /// `url_templates` are tried in order; `{model}` is replaced with `model_name`.
    pub fn download_ppocr(
        &self,
        model_name: &str,
        url_templates: &[String],
        on_progress: &dyn Fn(DownloadProgress),
    ) -> AppResult<String> {
        let model_dir = Path::new(&self.models_dir).join(model_name);
        std::fs::create_dir_all(&model_dir)?;

        if is_ppocr_installed_at(&model_dir) {
            on_progress(DownloadProgress {
                model_name: model_name.into(),
                downloaded: 100,
                total: 100,
                percentage: 100.0,
                stage: "completed".into(),
            });
            return Ok(model_dir.to_string_lossy().to_string());
        }

        let urls: Vec<String> = url_templates
            .iter()
            .map(|t| t.replace("{model}", model_name))
            .collect();

        on_progress(DownloadProgress {
            model_name: model_name.into(),
            downloaded: 0,
            total: 0,
            percentage: 0.0,
            stage: format!("Downloading {} model archive...", model_name),
        });

        let temp_dir = std::env::temp_dir();
        let archive_path = temp_dir.join(format!("{}.zip", model_name));

        let mut last_err: Option<String> = None;
        for url in &urls {
            match download_file(url, &archive_path, model_name, on_progress) {
                Ok(_) => {
                    log::info!("[download_ppocr] downloaded from {}", url);
                    last_err = None;
                    break;
                }
                Err(e) => {
                    let msg = format!("{}", e);
                    log::warn!("[download_ppocr] mirror failed {}: {}", url, msg);
                    last_err = Some(msg);
                }
            }
        }
        if let Some(e) = last_err {
            return Err(AppError::ModelDownload(format!(
                "All download mirrors failed. Last error: {e}. Please check your network connection."
            )));
        }

        on_progress(DownloadProgress {
            model_name: model_name.into(),
            downloaded: 100,
            total: 100,
            percentage: 90.0,
            stage: "Extracting...".into(),
        });

        // Extract zip
        let file = std::fs::File::open(&archive_path)
            .map_err(|e| AppError::ModelDownload(format!("Open archive failed: {}", e)))?;
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| AppError::ModelDownload(format!("Read zip archive failed: {}", e)))?;

        for i in 0..archive.len() {
            let mut entry = archive
                .by_index(i)
                .map_err(|e| AppError::ModelDownload(format!("Read zip entry failed: {}", e)))?;

            let name = entry.name().to_string();
            if entry.is_dir() {
                continue;
            }

            let entry_path = std::path::Path::new(&name);
            let filename = entry_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            let is_model_file = filename == "detection.onnx"
                || filename == "recognition.onnx"
                || filename == "cls.onnx"
                || filename == "dict.txt";

            if is_model_file {
                let dest = model_dir.join(&filename);
                let mut file_content = Vec::new();
                std::io::Read::read_to_end(&mut entry, &mut file_content)
                    .map_err(|e| AppError::ModelDownload(format!("Read entry failed: {}", e)))?;
                std::fs::write(&dest, &file_content)
                    .map_err(|e| AppError::ModelDownload(format!("Write file failed: {}", e)))?;
                log::info!("[download_ppocr] extracted {} -> {}", filename, dest.display());
            }
        }

        let _ = std::fs::remove_file(&archive_path);

        if !is_ppocr_installed_at(&model_dir) {
            return Err(AppError::ModelDownload(
                "Download completed but model file check failed".into(),
            ));
        }

        on_progress(DownloadProgress {
            model_name: model_name.into(),
            downloaded: 100,
            total: 100,
            percentage: 100.0,
            stage: "completed".into(),
        });

        Ok(model_dir.to_string_lossy().to_string())
    }
}