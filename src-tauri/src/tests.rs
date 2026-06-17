//! Integration tests for the OCR pipeline.
//!
//! Run with: cargo test --lib -- ocr_integration --nocapture

#[cfg(test)]
mod ocr_integration {
    use crate::engine::ocr::OcrEngine;
    use std::path::Path;

    /// Test OCR on docs/demo.png with each model version.
    #[test]
    fn test_ocr_demo_v4() {
        test_ocr_demo("ppocr-v4");
    }

    #[test]
    fn test_ocr_demo_v5() {
        test_ocr_demo("ppocr-v5");
    }

    #[test]
    fn test_ocr_demo_v6() {
        test_ocr_demo("ppocr-v6");
    }

    fn test_ocr_demo(model_version: &str) {
        let appdata = std::env::var("APPDATA").unwrap_or_default();
        let model_dir = Path::new(&appdata)
            .join("LynxOCR/models")
            .join(model_version);
        let demo_image = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../docs/demo.png"));

        if !model_dir.exists() {
            eprintln!("SKIP [{model_version}]: model not installed at {model_dir:?}");
            return;
        }
        if !demo_image.exists() {
            eprintln!("SKIP [{model_version}]: demo.png not found at {demo_image:?}");
            return;
        }

        let mut engine = OcrEngine::new_with_memory(&model_dir, false)
            .expect(&format!("[{model_version}] Failed to init OCR engine"));

        let result = engine
            .recognize_from_path(demo_image)
            .expect(&format!("[{model_version}] OCR recognition failed"));

        let full_text: String = result
            .text_blocks
            .iter()
            .map(|b| b.text.clone())
            .collect::<Vec<_>>()
            .join("");

        println!("[{model_version}] Recognized text: {}", full_text);
        println!("[{model_version}] Blocks: {}", result.text_blocks.len());

        for (i, block) in result.text_blocks.iter().enumerate() {
            println!(
                "  [{model_version}] block[{i}]: text=\"{}\", confidence={:.4}",
                block.text, block.confidence
            );
        }

        assert!(
            !full_text.trim().is_empty(),
            "[{model_version}] OCR returned empty text"
        );

        let has_keywords = full_text.contains("项目") || full_text.contains("预评价");
        if !has_keywords {
            eprintln!(
                "[{model_version}] WARNING: Expected text contains '项目' or '预评价', got: {}",
                full_text
            );
        }
    }
}