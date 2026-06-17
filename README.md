# LynxOCR

> **Offline OCR Text Recognition Tool**

LynxOCR is a blazing-fast, cross-platform desktop application for offline OCR text recognition. Powered by PaddleOCR (PP-OCR V4/V5/V6) and ONNX Runtime — all processing happens entirely on your device. **No internet required, your data stays private.**

## Features

### Text Recognition (OCR)
- **PaddleOCR Models** — Support for PP-OCR V4, V5, and V6 ONNX models with one-click download.
- **Image OCR** — Drag-and-drop or file picker to load images (PNG, JPG, BMP, WEBP, TIFF) for text recognition.
- **PDF OCR** — Render and recognize text from PDF documents.
- **Screenshot OCR** — Press a global shortcut (default `Ctrl+Shift+O`) to capture any screen region, recognize text, and copy to clipboard automatically. Supports multi-monitor setups.
- **Batch Processing** — Process multiple images at once with progress tracking.

### Application
- **System Tray** — Closing the window minimizes to the system tray. Left-click to restore, right-click to quit.
- **Single Instance** — Only one instance can run at a time; launching again activates the existing window.
- **Global Shortcuts** — Screenshot OCR shortcut works even when the app is minimized or in the tray.
- **Multi-language UI** — Interface available in English and Chinese.
- **Model Management** — One-click model download with progress tracking; switch active model at any time.

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Desktop Framework | [Tauri v2](https://v2.tauri.app) (Rust backend) |
| Frontend | React 19 + TypeScript + [shadcn/ui](https://ui.shadcn.com) |
| OCR Engine | [PaddleOCR](https://github.com/PaddlePaddle/PaddleOCR) via [paddle-ocr-rs](https://github.com/mg-chao/paddle-ocr-rs) |
| OCR Models | PP-OCR V4/V5/V6 ONNX (~20MB each) |
| Screenshot Capture | [xcap](https://github.com/nicepkg/xcap) (multi-monitor support) |
| PDF Rendering | [pdfium-render](https://github.com/ajrcarey/pdfium-render) |
| Build Tool | [Bun](https://bun.sh) + Vite |

## Quick Start

### Prerequisites

- [Bun](https://bun.sh) (package manager)
- [Rust](https://rustup.rs) (for Tauri backend compilation)

### Development

```bash
# Install dependencies
bun install

# Run in development mode
bun run tauri dev

# Build for production
bun run tauri build
```

### Models

OCR models can be downloaded from within the app via **Settings -> Model Management -> Download**.

| Model | Size | Description |
|-------|------|-------------|
| PP-OCR V4 | ~20MB | Chinese/English text detection & recognition |
| PP-OCR V5 | ~20MB | Improved Chinese/English accuracy |
| PP-OCR V6 | ~20MB | Latest version, multilingual high accuracy |

Models are stored in a configurable local directory. The default path is `{app_data_dir}/models/`.

## Contributing

LynxOCR is under active development. Contributions, issues, and feature requests are welcome.

## License

MIT

## Acknowledgments

LynxOCR is built on the shoulders of giants. Special thanks to these outstanding open-source projects:

- [PaddleOCR](https://github.com/PaddlePaddle/PaddleOCR) — Outstanding multilingual OCR toolkit
- [OnnxOCR](https://github.com/jingsongliujing/OnnxOCR) — High-performance PaddleOCR ONNX inference engine
- [paddle-ocr-rs](https://github.com/mg-chao/paddle-ocr-rs) — Rust bindings for PaddleOCR ONNX inference
- [xcap](https://github.com/nicepkg/xcap) — Cross-platform screen capture library
- [pdfium-render](https://github.com/ajrcarey/pdfium-render) — Rust bindings for PDFium
- [Tauri](https://tauri.app/) — Cross-platform desktop application framework
- [React](https://react.dev/) — Frontend UI library
- [shadcn/ui](https://ui.shadcn.com/) — Beautifully designed UI components