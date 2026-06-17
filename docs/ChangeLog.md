# Changelog

All notable changes to LynxOCR will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [v1.0.0] - 2026-06-17

### Changed
- Renamed project from VelociText to **LynxOCR** — now an OCR-only application
- Updated GitHub repository URL to `https://github.com/tabortao/LynxOCR`
- Updated all branding, window titles, tray tooltips, and configuration paths to LynxOCR

### Removed
- All transcription (ASR) functionality: transcription page, settings, and related code
- FFmpeg integration, dictionary, and ASR model management (SenseVoice, Paraformer, Qwen3-ASR, Silero VAD)
- ASR-related Rust dependencies: `sherpa-onnx`, `symphonia`
- Frontend dependency: `wavesurfer.js`
- Unused modules and dead code (`models/mod.rs`, `debug-events.ts`, `dashboard/data.json`)

### Fixed
- Critical bug: screenshot OCR event name mismatch (`velocitext:` → `lynxocr:`)
- Missing Rust commands: `write_text_file` and `open_file_with_system` for TXT export
- Missing `model_manager` module declaration in `engine/mod.rs`
- Duplicate i18n labels in sidebar — now uses centralized `t()` function
- Removed stale `exportFormat` and `ffmpegPath` fields from frontend `AppConfig` type

### Optimized
- Simplified sidebar navigation to OCR-only (removed transcription, dictionary, history entries)
- Cleaned up `Cargo.lock` to remove stale dependencies
- Updated `README.md` and `README-zh.md` for OCR-only focus
- Removed ASR roadmap items from `docs/Roadmap.md`


