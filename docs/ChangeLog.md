# Changelog

All notable changes to LynxOCR will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [v1.0.1] - 2026-06-17

### Added
- Configurable OCR model download URLs — three mirrors with auto-fallback in `config.json` (`modelDownloadUrls`). Use `{model}` placeholder for model name.
- Built-in RESTful OCR API service (`axum` HTTP server) with endpoints: `POST /api/v1/ocr` (multipart + JSON base64), `GET /api/v1/health`, `GET /api/v1/info`
- API settings page with server start/stop, port configuration, optional Bearer token auth, file size limit, and auto-start on launch
- New Rust commands: `api_start_server`, `api_stop_server`, `api_get_server_status`, `write_text_file`, `open_file_with_system`
- English i18n translations for all API settings UI
- New `Switch` UI component
- WebP image format support for OCR (enabled `webp` feature in `image` crate)
- Image URL (图床) input support: `POST /api/v1/ocr` now accepts `{"url": "https://..."}` for remote images
- Comprehensive API usage tutorial (Chinese): `docs/API使用教程.md`
- API quick-start section in README.md and README-zh.md

### Changed
- Default OCR model switched from PaddleOCR V5 to **PaddleOCR V6**
- Model download moved to configurable URL templates (was hardcoded gitcode.com URL)
- Download agent: connect/read/write timeouts + 5 redirects follow
- Updated `rebuild_ocr_zips.rs` example to point to LynxOCR repository

### Fixed
- Critical bug: screenshot OCR event name mismatch (`velocitext:` → `lynxocr:`)
- Missing `model_manager` module declaration in `engine/mod.rs`
- Duplicate i18n labels in sidebar — now uses centralized `t()` function
- Removed stale `exportFormat` and `ffmpegPath` fields from frontend `AppConfig` type

### Removed
- All transcription (ASR) functionality: transcription page, settings, and related code
- FFmpeg integration, dictionary, and ASR model management (SenseVoice, Paraformer, Qwen3-ASR, Silero VAD)
- ASR-related Rust dependencies: `sherpa-onnx`, `symphonia`
- Frontend dependency: `wavesurfer.js`
- Unused modules and dead code (`models/mod.rs`, `debug-events.ts`, `dashboard/data.json`)
- Stale `README-zh.md` ASR content (now OCR-only)

## [v1.0.0] - 2026-06-17

### Changed
- Initial rename: VelociText → LynxOCR
- Updated GitHub URL to `https://github.com/tabortao/LynxOCR`
- Rebranded window titles, tray tooltips, app data directory


