//! OCR engine — wraps paddle-ocr-rs for PaddleOCR ONNX inference.
//!
//! Performance strategy:
//! - Full CPU threads for ONNX Runtime (max parallelism)
//! - ONNX GraphOptimizationLevel::Level3 for best inference speed
//! - Model data cached in memory for fast re-initialization
//! - Image processing: take ownership, drop after inference
//! - Engine released after use to free memory; re-created on next OCR call

pub mod types;

use crate::errors::{AppError, AppResult};
use ort::session::builder::SessionBuilder;
use paddle_ocr_rs::ocr_lite::OcrLite;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::io::Write;
use std::path::{Path, PathBuf};
use types::{OcrResult, TextBlockInfo};

/// Build a performance-optimized ONNX Runtime session.
/// Uses all physical cores and Level 3 graph optimization for maximum speed.
fn build_ocr_session(builder: SessionBuilder) -> Result<SessionBuilder, ort::Error> {
    let num_threads = num_cpus::get_physical();
    builder
        .with_inter_threads(num_threads)?
        .with_intra_threads(num_threads)?
        .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level3)
}

/// Convert RGBA image data to RGB using parallel processing.
fn convert_rgba_to_rgb(image: &[u8]) -> Vec<u8> {
    let pixel_count = image.len() / 4;
    debug_assert!(image.len() % 4 == 0, "RGBA image data must be divisible by 4");
    let mut rgb_data = Vec::with_capacity(pixel_count * 3);

    unsafe {
        rgb_data.set_len(pixel_count * 3);
        let image_ptr = image.as_ptr() as usize;
        let rgb_ptr = rgb_data.as_mut_ptr() as usize;

        (0..pixel_count).into_par_iter().for_each(|i| {
            let image_base = i * 4;
            let rgb_base = i * 3;
            std::ptr::copy_nonoverlapping(
                (image_ptr as *const u8).add(image_base),
                (rgb_ptr as *mut u8).add(rgb_base),
                3,
            );
        });
    }

    rgb_data
}

/// Prepare a corrected dictionary file for `paddle-ocr-rs`.
fn prepare_ocr_dict(original_dict: &Path) -> AppResult<PathBuf> {
    let corrected_path = original_dict.with_file_name("dict_ocr.txt");

    let needs_regenerate = if corrected_path.exists() {
        let orig_modified = std::fs::metadata(original_dict)
            .and_then(|m| m.modified())
            .ok();
        let corrected_modified = std::fs::metadata(&corrected_path)
            .and_then(|m| m.modified())
            .ok();
        match (orig_modified, corrected_modified) {
            (Some(orig), Some(corr)) => orig > corr,
            _ => true,
        }
    } else {
        true
    };

    if needs_regenerate {
        let content = std::fs::read_to_string(original_dict)
            .map_err(|e| AppError::Ocr(format!("Failed to read dict.txt: {e}")))?;

        let mut file = std::fs::File::create(&corrected_path)
            .map_err(|e| AppError::Ocr(format!("Failed to create dict_ocr.txt: {e}")))?;

        writeln!(file, "#")?;
        file.write_all(content.as_bytes())?;
        if !content.ends_with('\n') {
            writeln!(file)?;
        }
        writeln!(file, " ")?;
    }

    Ok(corrected_path)
}

/// OCR engine that wraps paddle-ocr-rs OcrLite.
/// Models are cached in memory for fast re-initialization after release.
pub struct OcrEngine {
    inner: OcrLite,
    /// Cached model data for fast re-initialization.
    #[allow(dead_code)]
    det_model_data: Vec<u8>,
    #[allow(dead_code)]
    cls_model_data: Option<Vec<u8>>,
    #[allow(dead_code)]
    rec_model_data: Vec<u8>,
    /// Whether dict.txt exists (V5/V6 models).
    #[allow(dead_code)]
    has_dict: bool,
    #[allow(dead_code)]
    model_dir: PathBuf,
}

impl OcrEngine {
    /// Create a new OCR engine with models cached in memory.
    /// Use this for the initial creation — models are loaded from disk
    /// and cached in memory for potential re-initialization.
    pub fn new(model_dir: &Path) -> AppResult<Self> {
        let (det_path, rec_path, cls_path) = Self::model_paths(model_dir);
        let dict_path = model_dir.join("dict.txt");
        let has_dict = dict_path.exists();

        // Read model files into memory for caching
        let det_data = std::fs::read(&det_path)
            .map_err(|e| AppError::Ocr(format!("Failed to read det model: {e}")))?;
        let rec_data = std::fs::read(&rec_path)
            .map_err(|e| AppError::Ocr(format!("Failed to read rec model: {e}")))?;
        let cls_data = if cls_path.exists() {
            Some(
                std::fs::read(&cls_path)
                    .map_err(|e| AppError::Ocr(format!("Failed to read cls model: {e}")))?,
            )
        } else {
            None
        };

        let mut ocr = OcrLite::new();
        let num_threads = num_cpus::get_physical();

        if has_dict {
            let corrected_dict = prepare_ocr_dict(&dict_path)?;
            let det_str = det_path.to_string_lossy().to_string();
            let rec_str = rec_path.to_string_lossy().to_string();
            let cls_str = if cls_path.exists() {
                cls_path.to_string_lossy().to_string()
            } else {
                String::new()
            };
            let dict_str = corrected_dict.to_string_lossy().to_string();
            ocr.init_models_with_dict(
                &det_str, &cls_str, &rec_str, &dict_str, num_threads,
            )
            .map_err(|e| AppError::Ocr(format!("Failed to init OCR models: {e}")))?;
        } else if let Some(ref cls_bytes) = cls_data {
            ocr.init_models_from_memory_custom(&det_data, cls_bytes, &rec_data, build_ocr_session)
                .map_err(|e| {
                    AppError::Ocr(format!("Failed to init OCR models from memory: {e}"))
                })?;
        } else {
            ocr.init_models_from_memory_custom(&det_data, &[], &rec_data, build_ocr_session)
                .map_err(|e| {
                    AppError::Ocr(format!("Failed to init OCR models from memory: {e}"))
                })?;
        }

        Ok(Self {
            inner: ocr,
            det_model_data: det_data,
            cls_model_data: cls_data,
            rec_model_data: rec_data,
            has_dict,
            model_dir: model_dir.to_path_buf(),
        })
    }

    fn model_paths(model_dir: &Path) -> (PathBuf, PathBuf, PathBuf) {
        let det_path = model_dir.join("detection.onnx");
        let rec_path = model_dir.join("recognition.onnx");
        let cls_path = model_dir.join("cls.onnx");
        (det_path, rec_path, cls_path)
    }

    /// Run OCR on an image file path.
    pub fn recognize_from_path(&mut self, image_path: &Path) -> AppResult<OcrResult> {
        let img = image::open(image_path)
            .map_err(|e| AppError::Ocr(format!("Failed to open image: {e}")))?;
        self.recognize_from_image(img)
    }

    /// Run OCR on an in-memory image (takes ownership to avoid clone copies).
    pub fn recognize_from_image(
        &mut self,
        img: image::DynamicImage,
    ) -> AppResult<OcrResult> {
        let start = std::time::Instant::now();

        let long_side = img.width().max(img.height());
        const OCR_TARGET_SIZE: u32 = 960;
        const OCR_MAX_SIZE: u32 = 1920;

        let need_resize = long_side < OCR_TARGET_SIZE || long_side > OCR_MAX_SIZE;
        let img = if need_resize {
            let (w, h) = (img.width(), img.height());
            if long_side < OCR_TARGET_SIZE {
                let factor = 1.5;
                img.resize_exact(
                    (w as f32 * factor) as u32,
                    (h as f32 * factor) as u32,
                    image::imageops::FilterType::Lanczos3,
                )
            } else {
                let scale = OCR_MAX_SIZE as f32 / long_side as f32;
                img.resize_exact(
                    (w as f32 * scale) as u32,
                    (h as f32 * scale) as u32,
                    image::imageops::FilterType::Lanczos3,
                )
            }
        } else {
            img
        };

        let max_size = img.height().max(img.width());

        // Convert to RgbImage — use into_raw() to avoid clone
        let image_buffer = match img {
            image::DynamicImage::ImageRgb8(rgb) => rgb,
            image::DynamicImage::ImageRgba8(rgba) => {
                let (w, h) = (rgba.width(), rgba.height());
                let raw = rgba.into_raw();
                let rgb_data = convert_rgba_to_rgb(&raw);
                drop(raw);
                image::RgbImage::from_raw(w, h, rgb_data)
                    .ok_or_else(|| AppError::Ocr("Failed to convert RGBA to RGB".into()))?
            }
            other => {
                let rgba = other.to_rgba8();
                let (w, h) = (rgba.width(), rgba.height());
                let raw = rgba.into_raw();
                let rgb_data = convert_rgba_to_rgb(&raw);
                drop(raw);
                image::RgbImage::from_raw(w, h, rgb_data)
                    .ok_or_else(|| AppError::Ocr("Failed to convert image to RGB".into()))?
            }
        };

        let result = self
            .inner
            .detect_angle_rollback(
                &image_buffer,
                50,       // padding
                max_size, // max side len
                0.5,      // box score threshold
                0.3,      // box threshold
                1.6,      // unclip ratio
                true,     // do angle classification
                false,    // most angle (only 0/180)
                0.9,      // rollback threshold
            )
            .map_err(|e| AppError::Ocr(format!("OCR recognition failed: {e}")))?;

        drop(image_buffer);

        let text_blocks: Vec<TextBlockInfo> = result
            .text_blocks
            .iter()
            .map(|block| TextBlockInfo {
                text: block.text.clone(),
                confidence: block.text_score,
                box_points: block
                    .box_points
                    .iter()
                    .map(|p| [p.x as f32, p.y as f32])
                    .collect(),
            })
            .collect();

        Ok(OcrResult {
            text_blocks,
            total_time_ms: start.elapsed().as_millis() as u64,
        })
    }

    /// Run OCR on raw image bytes (PNG/JPEG/etc.).
    pub fn recognize_from_bytes(&mut self, data: &[u8]) -> AppResult<OcrResult> {
        let img = image::load_from_memory(data)
            .map_err(|e| AppError::Ocr(format!("Failed to decode image: {e}")))?;
        self.recognize_from_image(img)
    }

    /// Run OCR on raw RGBA pixel data (bypasses PNG encode/decode).
    pub fn recognize_from_raw_rgba(
        &mut self,
        rgba_data: &[u8],
        width: u32,
        height: u32,
    ) -> AppResult<OcrResult> {
        let start = std::time::Instant::now();

        let rgb_data = convert_rgba_to_rgb(rgba_data);
        let image_buffer = image::RgbImage::from_raw(width, height, rgb_data)
            .ok_or_else(|| AppError::Ocr("Failed to create RGB image from raw data".into()))?;

        let max_size = height.max(width);

        let result = self
            .inner
            .detect_angle_rollback(
                &image_buffer, 50, max_size, 0.5, 0.3, 1.6, true, false, 0.9,
            )
            .map_err(|e| AppError::Ocr(format!("OCR recognition failed: {e}")))?;

        drop(image_buffer);

        let text_blocks: Vec<TextBlockInfo> = result
            .text_blocks
            .iter()
            .map(|block| TextBlockInfo {
                text: block.text.clone(),
                confidence: block.text_score,
                box_points: block
                    .box_points
                    .iter()
                    .map(|p| [p.x as f32, p.y as f32])
                    .collect(),
            })
            .collect();

        Ok(OcrResult {
            text_blocks,
            total_time_ms: start.elapsed().as_millis() as u64,
        })
    }
}