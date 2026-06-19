# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

LynxOCR is a cross-platform desktop OCR application: a **Tauri v2** shell with a **Rust** backend (`src-tauri/`) and a **React 19 + TypeScript** frontend (`src/`). All OCR runs offline on-device via PaddleOCR ONNX models and ONNX Runtime (`ort`). The package manager is **Bun**, not npm/yarn.

## Commands

```bash
bun install                # install frontend deps
bun run tauri dev          # run the full app (Tauri + Vite dev server on :1420)
bun run tauri build        # production build (installer in src-tauri/target/release/bundle/)
bun run dev                # Vite frontend only (no Rust) — rarely useful standalone
bun run lint               # ESLint over the frontend
bun run typecheck          # tsc --noEmit
bun run format             # Prettier write over **/*.{ts,tsx}

# Rust backend
cargo build                # from src-tauri/
cargo test --lib           # run all Rust tests (from src-tauri/)
cargo test --lib -- ocr_integration --nocapture   # OCR integration tests (src-tauri/src/tests.rs)
```

**OCR integration tests require models installed on disk** at `%APPDATA%/LynxOCR/models/<model>/` (or `~/.local/share/LynxOCR/models/` on non-Windows). Download them in-app first via Settings → Model Management.

## Architecture

### Two-process, command-driven model
The frontend never touches OCR/files directly — it calls Rust via `invoke()` (`@tauri-apps/api/core`). All commands are registered in `src-tauri/src/lib.rs` (`invoke_handler!`) and implemented under `src-tauri/src/commands/` (`ocr.rs`, `model.rs`, `config.rs`, `api.rs`). Adding a backend capability = write the `#[tauri::command]` fn + register it in `lib.rs`.

**Serde casing:** Rust structs use `#[serde(rename_all = "camelCase")]`, so the TS side sees `camelCase` fields (e.g. `textBlocks`, `boxPoints`, `modelPath`). Keep this in mind when wiring new payloads.

### OCR engine lifecycle (memory-sensitive)
`engine/ocr/mod.rs` wraps `paddle-ocr-rs`'s `OcrLite`. The engine is stored as `Arc<Mutex<Option<OcrEngine>>>` in `AppState` and lazily created on first use (`get_or_create_engine`). It is **deliberately released** (`*engine = None`) to free ONNX Runtime memory:
- `ocr_release` — explicit release after the frontend finishes a batch.
- `ocr_set_active_model` — releases so the next call rebuilds with the new model.

Long CPU work (`recognize_*`, PDF render, screenshot capture) always runs inside `tokio::task::spawn_blocking` to keep the UI responsive. `recognize_from_image` takes ownership of the image and resizes the long side into a 960–1920px band before inference.

### Model layout & download
Models live in `{model_path}/<name>/` where `<name>` ∈ `ppocr-v4` | `ppocr-v5` | `ppocr-v6`. A model is "installed" iff `detection.onnx` and `recognition.onnx` exist (`is_ppocr_installed_at`); `cls.onnx` and `dict.txt` are optional. `model_manager.rs` downloads a `.zip` (trying each URL in `config.model_download_urls` in order), extracts only those four filenames. **V5/V6 ship a `dict.txt`** which is rewritten into `dict_ocr.txt` (prepended `#`, appended space line) by `prepare_ocr_dict` before passing to `init_models_with_dict`; V4 (no dict) inits from in-memory bytes.

### Config
`config/app_config.rs` `AppConfig` is the single source of truth, persisted to `{app_data_dir}/config.json`. `app_data_dir()` is `%APPDATA%/LynxOCR` on Windows, `~/.local/share/LynxOCR` elsewhere. All fields have serde defaults so old config files load forward-compatibly. Read/write from the frontend via `get_app_config` / `set_app_config`.

### Screenshot OCR flow (multi-window)
1. Global shortcut (default `Ctrl+Shift+O`, parsed by `parse_shortcut_string` in `lib.rs`) emits `trigger-screenshot-ocr`, OR the OCR page calls it directly.
2. `start_screenshot_selection` captures + stitches all monitors (`xcap`) into one PNG, stashes metadata in `AppState.pending_screenshot`, and spawns a transparent fullscreen `screenshot` window loading `screenshot.html`.
3. `screenshot-main.ts` (a **separate Vite entry**, see `vite.config.ts` `rollupOptions.input`) runs in that window. It uses **`window.__TAURI__.core`** (global Tauri, `withGlobalTauri: true`) — NOT the npm `@tauri-apps/api` import — fetches data via `get_screenshot_data`, lets the user drag-select, calls `ocr_screenshot_region`, then `screenshot_ocr_done` which emits `screenshot-ocr-result` back to the main window and copies to clipboard.

### Built-in HTTP API server
`api/server.rs` is an Axum server (routes `/api/v1/ocr`, `/health`, `/info`) started/stopped via `api_start_server`/`api_stop_server` commands, or auto-started on launch if `api_server_auto_start`. It shares the **same** `ocr_engine` and `active_ocr_model` Arcs from `AppState`. Optional Bearer-token auth (`api_key`); `/health` is always unauthenticated. The multipart parser is **hand-rolled over raw bytes** (`handle_ocr_multipart`) to avoid UTF-8-corrupting binary image data — don't replace it with a string-based parser. JSON requests accept `image` (base64/data-URI) or `url`.

### Frontend structure
- `src/App.tsx` is the shell: state-based routing via a `currentPage` string (no router lib), pages lazy-loaded with `React.lazy`/`Suspense`. The `Page` type enumerates routes.
- Pages live in `src/app/<route>/page.tsx`. UI primitives are shadcn/ui in `src/components/ui/` (generated — avoid hand-editing).
- `src/lib/app-context.tsx` holds theme (light/dark via `next-themes`-style class toggle) + i18n. **i18n is a hand-rolled `dict` object** (zh/en) with a `t(key, vars)` function — there is no i18n library. New UI strings go into both `zh` and `en` maps; `zh` is the default language.
- Path alias `@/` → `src/` (configured in `vite.config.ts` and `tsconfig`).
- Two HTML entry points: `index.html` (main app) and `screenshot.html` (selection overlay).

### PDF
`pdf_render_page` / `ocr_recognize_pdf` use `pdfium-render`. `create_pdfium` searches for `pdfium.dll` next to the exe, in `bin/`, and in the resource dir (bundled via `tauri.conf.json` `resources`). On non-Windows it falls back to the system pdfium library.

## Conventions

- **Prettier:** no semicolons, double quotes, 2-space indent, 80 col, `es5` trailing commas. `cn`/`cva` are registered tailwind functions.
- **Window close:** the main window's close button hides to tray (`prevent_close`) rather than exiting; only the tray "退出" menu or `app.exit(0)` truly quits. Single-instance is enforced.
- Release profile is aggressively size-optimized (`lto = "fat"`, `panic = "abort"`, `strip`, `codegen-units = 1`) — build times are long; prefer `cargo build` (dev) while iterating.
- `docs/` contains design notes (Chinese): `OCR优化总结.md`, `内存优化方案.md`, `截图OCR实现原理.md`, `自定义模型下载地址.md`. `docs/Reference/` holds upstream reference projects (OnnxOCR, snow-shot, etc.).
