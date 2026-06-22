//! MinerU cloud API client — calls MinerU REST API for document extraction.
//!
//! Two modes:
//! - Flash Extract (no token): Agent Lightweight API, Markdown only, ≤10MB, ≤20 pages
//!   Flow: POST /api/v1/agent/parse/file (get signed upload URL) → PUT file
//!         → GET /api/v1/agent/parse/{task_id} (poll) → download markdown_url
//! - Extract (with token): Precision Extract API, multi-format, ≤200MB, ≤200 pages
//!   Flow: POST /api/v4/file-urls/batch (get signed upload URLs) → PUT file
//!         → system auto-submits task → GET /api/v4/extract-results/batch/{batch_id} (poll)
//!         → download full_zip_url and extract requested format

use crate::errors::{AppError, AppResult};
use serde::Deserialize;
use std::io::Read;
use std::path::Path;
use std::time::Instant;

const DEFAULT_BASE_URL: &str = "https://mineru.net";
const POLL_INTERVAL_MS: u64 = 2000;
const FLASH_TIMEOUT_SECS: u64 = 300;
const EXTRACT_TIMEOUT_SECS: u64 = 600;

/// MinerU API client.
pub struct MineruClient {
    token: String,
    base_url: String,
}

/// Result from MinerU extraction.
pub struct MineruResult {
    pub content: String,
    pub format: String,
    pub total_time_ms: u64,
}

// --- Response types ---

#[derive(Debug, Deserialize)]
struct ApiResponse {
    code: i32,
    msg: Option<String>,
    data: Option<serde_json::Value>,
}

/// Flash extract: response from POST /api/v1/agent/parse/file
#[derive(Debug, Deserialize)]
struct FlashSubmitData {
    task_id: String,
    file_url: String,
}

/// Flash extract poll result from GET /api/v1/agent/parse/{task_id}
#[derive(Debug, Deserialize)]
struct FlashPollData {
    state: String,
    err_msg: Option<String>,
    err_code: Option<i32>,
    #[serde(default)]
    markdown_url: Option<String>,
}

/// Extract batch: response from POST /api/v4/file-urls/batch
#[derive(Debug, Deserialize)]
struct BatchResponse {
    batch_id: String,
    file_urls: Vec<String>,
}

/// Batch result item from GET /api/v4/extract-results/batch/{batch_id}
#[derive(Debug, Deserialize)]
struct BatchResultItem {
    file_name: String,
    state: String,
    err_msg: Option<String>,
    full_zip_url: Option<String>,
}

/// Batch result data from GET /api/v4/extract-results/batch/{batch_id}
#[derive(Debug, Deserialize)]
struct BatchResultData {
    #[serde(default)]
    extract_result: Vec<BatchResultItem>,
}

// --- MineruClient impl ---

impl MineruClient {
    pub fn new(token: String, base_url: Option<String>) -> Self {
        let base_url = base_url
            .filter(|u| !u.is_empty())
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string())
            .trim_end_matches('/')
            .to_string();
        Self { token, base_url }
    }

    pub fn has_token(&self) -> bool {
        !self.token.is_empty()
    }

    // ==================== Flash Extract ====================

    /// Flash extract — no token required, Markdown only.
    pub fn flash_extract(&self, file_path: &Path) -> AppResult<MineruResult> {
        let start = Instant::now();
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file.png");

        let file_data = std::fs::read(file_path)
            .map_err(|e| AppError::Ocr(format!("Failed to read file: {e}")))?;

        let file_size = file_data.len();
        if file_size > 10 * 1024 * 1024 {
            return Err(AppError::Ocr(
                "Flash extract: file exceeds 10MB limit".into(),
            ));
        }

        let agent = build_agent();

        // Step 1: POST /api/v1/agent/parse/file to get signed upload URL + task_id
        let url = format!("{}/api/v1/agent/parse/file", self.base_url);
        let submit_body = serde_json::json!({
            "file_name": file_name,
        });

        let resp = agent
            .post(&url)
            .set("Content-Type", "application/json")
            .send_string(&submit_body.to_string())
            .map_err(|e| AppError::Ocr(format!("MinerU flash submit failed: {e}")))?;

        let status = resp.status();
        let resp_body = resp
            .into_string()
            .map_err(|e| AppError::Ocr(format!("MinerU flash read response: {e}")))?;

        if status < 200 || status >= 300 {
            return Err(AppError::Ocr(format!(
                "MinerU flash submit HTTP {}: {}",
                status,
                truncate_str(&resp_body, 200)
            )));
        }

        let api_resp: ApiResponse = serde_json::from_str(&resp_body)
            .map_err(|e| AppError::Ocr(format!("MinerU flash parse response: {e}")))?;

        if api_resp.code != 0 {
            return Err(AppError::Ocr(format!(
                "MinerU flash API error: {}",
                api_resp.msg.unwrap_or_default()
            )));
        }

        let data = api_resp
            .data
            .ok_or_else(|| AppError::Ocr("MinerU flash: no data in response".into()))?;
        let flash_data: FlashSubmitData = serde_json::from_value(data)
            .map_err(|e| AppError::Ocr(format!("MinerU flash parse submit data: {e}")))?;

        // Step 2: Upload file to the signed URL
        let put_resp = agent
            .put(&flash_data.file_url)
            .send_bytes(&file_data)
            .map_err(|e| AppError::Ocr(format!("MinerU flash file upload failed: {e}")))?;

        let put_status = put_resp.status();
        if put_status < 200 || put_status >= 300 {
            return Err(AppError::Ocr(format!(
                "MinerU flash file upload HTTP {}",
                put_status
            )));
        }

        drop(file_data);

        // Step 3: Poll for result
        let content = self.poll_flash_task(&flash_data.task_id, FLASH_TIMEOUT_SECS)?;

        Ok(MineruResult {
            content,
            format: "md".to_string(),
            total_time_ms: start.elapsed().as_millis() as u64,
        })
    }

    // ==================== Extract (with token) ====================

    /// Extract with token — supports multi-format output.
    /// Flow: POST /api/v4/file-urls/batch → PUT file → auto submit → poll batch results
    pub fn extract(&self, file_path: &Path, format: &str) -> AppResult<MineruResult> {
        if !self.has_token() {
            return Err(AppError::Ocr(
                "MinerU extract requires an API token. Configure it in settings.".into(),
            ));
        }

        let start = Instant::now();
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file.png");

        let file_data = std::fs::read(file_path)
            .map_err(|e| AppError::Ocr(format!("Failed to read file: {e}")))?;

        let agent = build_agent();
        let auth_header = format!("Bearer {}", self.token);

        // Step 1: POST /api/v4/file-urls/batch to get upload URLs
        let batch_url = format!("{}/api/v4/file-urls/batch", self.base_url);
        let batch_body = serde_json::json!({
            "files": [{"name": file_name}],
            "model_version": "vlm"
        });

        let resp = agent
            .post(&batch_url)
            .set("Content-Type", "application/json")
            .set("Authorization", &auth_header)
            .send_string(&batch_body.to_string())
            .map_err(|e| AppError::Ocr(format!("MinerU batch request failed: {e}")))?;

        let status = resp.status();
        let resp_body = resp
            .into_string()
            .map_err(|e| AppError::Ocr(format!("MinerU batch read response: {e}")))?;

        if status < 200 || status >= 300 {
            return Err(AppError::Ocr(format!(
                "MinerU batch HTTP {}: {}",
                status,
                truncate_str(&resp_body, 200)
            )));
        }

        let api_resp: ApiResponse = serde_json::from_str(&resp_body)
            .map_err(|e| AppError::Ocr(format!("MinerU batch parse response: {e}")))?;

        if api_resp.code != 0 {
            return Err(AppError::Ocr(format!(
                "MinerU batch API error: {}",
                api_resp.msg.unwrap_or_default()
            )));
        }

        let data = api_resp
            .data
            .ok_or_else(|| AppError::Ocr("MinerU batch: no data in response".into()))?;
        let batch: BatchResponse = serde_json::from_value(data)
            .map_err(|e| AppError::Ocr(format!("MinerU batch parse data: {e}")))?;

        // Step 2: Upload file to the signed URL
        let upload_url = batch
            .file_urls
            .first()
            .ok_or_else(|| AppError::Ocr("MinerU batch: no upload URL returned".into()))?;

        let put_resp = agent
            .put(upload_url)
            .send_bytes(&file_data)
            .map_err(|e| AppError::Ocr(format!("MinerU file upload failed: {e}")))?;

        let put_status = put_resp.status();
        if put_status < 200 || put_status >= 300 {
            return Err(AppError::Ocr(format!(
                "MinerU file upload HTTP {}",
                put_status
            )));
        }

        drop(file_data);

        // Step 3: Poll batch results (system auto-submits after upload)
        let zip_url = self.poll_batch_results(&batch.batch_id, EXTRACT_TIMEOUT_SECS)?;

        // Step 4: Download result zip and extract requested format
        let content = self.download_and_extract_format(&zip_url, format)?;

        Ok(MineruResult {
            content,
            format: format.to_string(),
            total_time_ms: start.elapsed().as_millis() as u64,
        })
    }

    // ==================== Polling ====================

    /// Poll the flash extract task until completion.
    /// GET /api/v1/agent/parse/{task_id}
    fn poll_flash_task(&self, task_id: &str, timeout_secs: u64) -> AppResult<String> {
        let deadline = Instant::now()
            .checked_add(std::time::Duration::from_secs(timeout_secs))
            .unwrap();

        let poll_url = format!("{}/api/v1/agent/parse/{}", self.base_url, task_id);
        let agent = build_agent();

        loop {
            if Instant::now() > deadline {
                return Err(AppError::Ocr("MinerU flash extract timed out".into()));
            }

            std::thread::sleep(std::time::Duration::from_millis(POLL_INTERVAL_MS));

            let resp = agent
                .get(&poll_url)
                .call()
                .map_err(|e| AppError::Ocr(format!("MinerU flash poll failed: {e}")))?;

            let status = resp.status();
            let body = resp
                .into_string()
                .map_err(|e| AppError::Ocr(format!("MinerU flash poll read: {e}")))?;

            if status != 200 {
                continue;
            }

            let api_resp: ApiResponse = serde_json::from_str(&body)
                .map_err(|e| AppError::Ocr(format!("MinerU flash poll parse: {e}")))?;

            if api_resp.code != 0 {
                continue;
            }

            let data = match api_resp.data {
                Some(d) => d,
                None => continue,
            };

            let flash: FlashPollData = match serde_json::from_value(data) {
                Ok(t) => t,
                Err(_) => continue,
            };

            match flash.state.as_str() {
                "done" => {
                    let md_url = flash
                        .markdown_url
                        .ok_or_else(|| AppError::Ocr("MinerU flash: no markdown URL".into()))?;
                    return self.download_text_content(&md_url);
                }
                "failed" => {
                    return Err(AppError::Ocr(format!(
                        "MinerU flash extract failed ({}): {}",
                        flash.err_code.unwrap_or(0),
                        flash.err_msg.unwrap_or_default()
                    )));
                }
                _ => {
                    // waiting-file, uploading, pending, running — continue polling
                }
            }
        }
    }

    /// Poll batch results until a task completes.
    /// GET /api/v4/extract-results/batch/{batch_id}
    fn poll_batch_results(&self, batch_id: &str, timeout_secs: u64) -> AppResult<String> {
        let deadline = Instant::now()
            .checked_add(std::time::Duration::from_secs(timeout_secs))
            .unwrap();

        let poll_url = format!(
            "{}/api/v4/extract-results/batch/{}",
            self.base_url, batch_id
        );
        let agent = build_agent();
        let auth_header = format!("Bearer {}", self.token);

        loop {
            if Instant::now() > deadline {
                return Err(AppError::Ocr(
                    "MinerU extract timed out waiting for batch results".into(),
                ));
            }

            std::thread::sleep(std::time::Duration::from_millis(POLL_INTERVAL_MS));

            let resp = match agent
                .get(&poll_url)
                .set("Authorization", &auth_header)
                .call()
            {
                Ok(r) => r,
                Err(_) => continue,
            };

            let status = resp.status();
            let body = match resp.into_string() {
                Ok(b) => b,
                Err(_) => continue,
            };

            if status != 200 {
                continue;
            }

            let api_resp: ApiResponse = match serde_json::from_str(&body) {
                Ok(r) => r,
                Err(_) => continue,
            };

            if api_resp.code != 0 {
                continue;
            }

            let data = match api_resp.data {
                Some(d) => d,
                None => continue,
            };

            let batch_result: BatchResultData = match serde_json::from_value(data) {
                Ok(r) => r,
                Err(_) => continue,
            };

            for item in &batch_result.extract_result {
                match item.state.as_str() {
                    "done" => {
                        if let Some(zip_url) = item.full_zip_url.clone() {
                            return Ok(zip_url);
                        }
                    }
                    "failed" => {
                        return Err(AppError::Ocr(format!(
                            "MinerU extract failed for '{}': {}",
                            item.file_name,
                            item.err_msg.clone().unwrap_or_default()
                        )));
                    }
                    _ => {}
                }
            }
        }
    }

    // ==================== Result extraction ====================

    /// Download a text content URL (for flash extract).
    fn download_text_content(&self, url: &str) -> AppResult<String> {
        let agent = build_agent();

        let resp = agent
            .get(url)
            .call()
            .map_err(|e| AppError::Ocr(format!("MinerU download content failed: {e}")))?;

        let status = resp.status();
        if status != 200 {
            return Err(AppError::Ocr(format!(
                "MinerU download content HTTP {}",
                status
            )));
        }

        let mut content = String::new();
        resp.into_reader()
            .read_to_string(&mut content)
            .map_err(|e| AppError::Ocr(format!("MinerU read content: {e}")))?;

        Ok(content)
    }

    /// Download the result zip and extract the requested format.
    fn download_and_extract_format(&self, zip_url: &str, format: &str) -> AppResult<String> {
        let agent = build_agent();

        let resp = agent
            .get(zip_url)
            .call()
            .map_err(|e| AppError::Ocr(format!("MinerU download result failed: {e}")))?;

        let status = resp.status();
        if status != 200 {
            return Err(AppError::Ocr(format!(
                "MinerU download result HTTP {}",
                status
            )));
        }

        let mut zip_data = Vec::new();
        resp.into_reader()
            .read_to_end(&mut zip_data)
            .map_err(|e| AppError::Ocr(format!("MinerU read result zip: {e}")))?;

        let cursor = std::io::Cursor::new(zip_data);
        let mut archive = zip::ZipArchive::new(cursor)
            .map_err(|e| AppError::Ocr(format!("MinerU open result zip: {e}")))?;

        let target_ext = match format {
            "md" | "markdown" => "md",
            "html" => "html",
            "latex" | "tex" => "tex",
            "docx" => "docx",
            "json" => "json",
            _ => "md",
        };

        let mut content = String::new();

        for i in 0..archive.len() {
            let mut entry = archive
                .by_index(i)
                .map_err(|e| AppError::Ocr(format!("MinerU read zip entry: {e}")))?;

            let name = entry.name().to_string();
            let entry_ext = std::path::Path::new(&name)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");

            if entry_ext == target_ext || name.ends_with(&format!(".{}", target_ext)) {
                if target_ext == "docx" {
                    let mut buf = Vec::new();
                    entry
                        .read_to_end(&mut buf)
                        .map_err(|e| AppError::Ocr(format!("MinerU read docx: {e}")))?;
                    use base64::Engine;
                    content = base64::engine::general_purpose::STANDARD.encode(&buf);
                } else {
                    entry
                        .read_to_string(&mut content)
                        .map_err(|e| AppError::Ocr(format!("MinerU read text: {e}")))?;
                }
                break;
            }
        }

        if content.is_empty() {
            // Fallback: return the full.md content
            for i in 0..archive.len() {
                let mut entry = archive
                    .by_index(i)
                    .map_err(|e| AppError::Ocr(format!("MinerU read zip entry: {e}")))?;

                let name = entry.name().to_string();
                if name.ends_with("full.md") || name.ends_with(".md") {
                    entry
                        .read_to_string(&mut content)
                        .map_err(|e| AppError::Ocr(format!("MinerU read md: {e}")))?;
                    break;
                }
            }
        }

        if content.is_empty() {
            return Err(AppError::Ocr(format!(
                "MinerU: no {} content found in result zip",
                format
            )));
        }

        Ok(content)
    }
}

// ==================== Helpers ====================

fn build_agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_secs(15))
        .timeout_read(std::time::Duration::from_secs(120))
        .timeout_write(std::time::Duration::from_secs(60))
        .redirects(5)
        .build()
}

fn truncate_str(s: &str, max_len: usize) -> &str {
    if s.len() <= max_len {
        s
    } else {
        &s[..max_len]
    }
}