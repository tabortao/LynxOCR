use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Model not downloaded: {0}")]
    ModelNotDownloaded(String),

    #[error("Model download failed: {0}")]
    ModelDownload(String),

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("OCR error: {0}")]
    Ocr(String),
}

pub type AppResult<T> = Result<T, AppError>;