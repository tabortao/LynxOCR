# OCR API Service Plan

## Summary

Design and implement a RESTful OCR API service within the LynxOCR desktop application using `axum` (Rust async web framework). The API server runs as a background component within the Tauri app, reusing the existing `OcrEngine` and `AppConfig`. It provides industry-standard OCR endpoints that other applications can call for image text recognition via HTTP.

## Research Findings

### Industry-Standard OCR API Formats Analyzed

| Service                | Input Method        | Auth             | Output Format                                 | Notable Features              |
| ---------------------- | ------------------- | ---------------- | --------------------------------------------- | ----------------------------- |
| PaddleOCR Serving      | JSON base64 array   | None             | `[{text, confidence, text_region}]`           | Predict args for async        |
| gunthercox/ocr-service | multipart/form-data | None             | `{text, regions: [{bbox, text, confidence}]}` | Health check, file size limit |
| NVIDIA NIM OCR         | JSON base64         | Bearer token     | `{data: [{text_detections}]}`                 | merge\_levels, batch input    |
| Azure Computer Vision  | Binary stream       | Subscription key | Hierarchical regions→lines→words              | Orientation detection         |
| Wavespeed PaddleOCR    | JSON base64         | Bearer token     | Async task + polling                          | markdown/json output          |

### Common Patterns to Adopt

1. **Dual input methods**: `multipart/form-data` (file upload) AND `application/json` (base64)
2. **Structured output**: `text` (concatenated) + `regions` (array with bbox, text, confidence)
3. **Health check**: `GET /health`
4. **Status codes**: 200 (success), 400 (bad request), 413 (file too large), 500 (server error)
5. **Error format**: `{ "error": { "code": "...", "message": "..." } }`
6. **Optional API key auth**: Bearer token for security
7. **Configurable port**: via `AppConfig`

## Implementation Status

### Completed (Phases 1-7)

| Phase | Description | Status |
|-------|-------------|--------|
| 1 | Add axum dependencies, create API types | Done |
| 2 | Implement axum server with OCR endpoint | Done |
| 3 | Add API config to AppConfig | Done |
| 4 | Add Tauri commands for start/stop/status | Done |
| 5 | Wire up AppState with server handle | Done |
| 6 | Build API settings frontend page | Done |
| 7 | Add navigation (route, sidebar) and Chinese i18n | Done |

### Files Implemented

| Action | File | Purpose | Status |
| ------ | ---- | ------- | ------ |
| NEW | `src-tauri/src/api/mod.rs` | API module entry | Done |
| NEW | `src-tauri/src/api/types.rs` | API request/response types | Done |
| NEW | `src-tauri/src/api/server.rs` | Axum HTTP server (~557 lines) | Done |
| NEW | `src-tauri/src/commands/api.rs` | Tauri commands for API lifecycle | Done |
| MODIFY | `src-tauri/src/config/app_config.rs` | Add API config fields | Done |
| MODIFY | `src-tauri/src/lib.rs` | Add api module, AppState fields, register commands | Done |
| MODIFY | `src-tauri/src/commands/mod.rs` | Register api commands module | Done |
| MODIFY | `src-tauri/Cargo.toml` | Add axum, tower, tower-http deps | Done |
| NEW | `src/app/api-settings/page.tsx` | API settings UI page (~259 lines) | Done |
| MODIFY | `src/App.tsx` | Add api-settings route | Done |
| MODIFY | `src/components/app-sidebar.tsx` | Add API nav item | Done |
| MODIFY | `src/lib/app-context.tsx` | Chinese i18n strings | Done (Chinese only) |

## Remaining Work

### Issue 1: Missing English i18n keys

**File**: `src/lib/app-context.tsx`

The English dictionary (`en:`) is missing all API-related translation keys. The Chinese dictionary has 27 keys (`api`, `header.api-settings`, `api.title`, `api.desc`, `api.serverStatus`, `api.serverRunning`, `api.serverStopped`, `api.running`, `api.stopped`, `api.start`, `api.stop`, `api.configuration`, `api.configDesc`, `api.port`, `api.key`, `api.keyPlaceholder`, `api.keyHint`, `api.hideKey`, `api.showKey`, `api.maxFileSize`, `api.maxFileSizeHint`, `api.autoStart`, `api.autoStartHint`, `api.exampleUsage`, `api.exampleDesc`, `api.healthCheck`, `api.ocrCurl`, `api.base64Curl`, `api.specifyModel`, `api.save`, `api.saved`), but the English dictionary has zero.

**Fix**: Add the following English translations to the `en:` dictionary (before the closing `}` at line 193):

```typescript
    // API settings page
    "api": "API Service",
    "header.api-settings": "API Service",
    "api.title": "API Service",
    "api.desc": "Enable built-in HTTP API service for other applications to call OCR",
    "api.serverStatus": "Server Status",
    "api.serverRunning": "Running · Port {port}",
    "api.serverStopped": "Server not running",
    "api.running": "Running",
    "api.stopped": "Stopped",
    "api.start": "Start Server",
    "api.stop": "Stop Server",
    "api.configuration": "Configuration",
    "api.configDesc": "Configure API server port, authentication and limits",
    "api.port": "Port",
    "api.key": "API Key (optional)",
    "api.keyPlaceholder": "Leave empty to disable authentication",
    "api.keyHint": "When set, all endpoints except /health require Bearer token authentication",
    "api.hideKey": "Hide key",
    "api.showKey": "Show key",
    "api.maxFileSize": "Max upload file size (MB)",
    "api.maxFileSizeHint": "Images exceeding this size will be rejected",
    "api.autoStart": "Auto-start on launch",
    "api.autoStartHint": "Start API server automatically when the app launches",
    "api.exampleUsage": "Usage Examples",
    "api.exampleDesc": "Example curl commands for calling the API",
    "api.healthCheck": "Health Check",
    "api.ocrCurl": "Image Upload OCR (multipart)",
    "api.base64Curl": "Base64 Image OCR (JSON)",
    "api.specifyModel": "Specify Model Version",
    "api.save": "Save Configuration",
    "api.saved": "Saved",
```

### Issue 2: `api_server_enabled` field unused

**File**: `src-tauri/src/config/app_config.rs` (line 27), `src/types/index.ts`

The `api_server_enabled` field is defined in `AppConfig` but never used in any logic:
- `commands/api.rs` does not check it before starting the server
- `api-settings/page.tsx` does not show a toggle for it

**Decision**: Remove the `api_server_enabled` field entirely since:
- `api_server_auto_start` already controls auto-start behavior
- The server can be manually started/stopped via the UI
- `api_server_enabled` adds unnecessary complexity without clear value

**Fix**:
1. Remove `pub api_server_enabled: bool` from `src-tauri/src/config/app_config.rs` (line 27)
2. Remove `api_server_enabled: false` from the `Default` impl (line 80)
3. Remove `apiServerEnabled?: boolean` from `src/types/index.ts` AppConfig interface

## Proposed Architecture

```
┌─────────────────────────────────────────────────────┐
│                    Tauri App                         │
│  ┌──────────────┐  ┌──────────────────────────────┐ │
│  │  Frontend    │  │  Rust Backend                 │ │
│  │  (React)     │  │  ┌────────────────────────┐  │ │
│  │              │  │  │  Axum HTTP Server       │  │ │
│  │  API Settings│  │  │  (port: 9720)           │  │ │
│  │  UI (new)    │◄─┤  │  POST /api/v1/ocr       │  │ │
│  │              │  │  │  GET  /api/v1/health    │  │ │
│  └──────────────┘  │  │  GET  /api/v1/info      │  │ │
│                    │  └──────────┬─────────────┘  │ │
│                    │             │                 │ │
│                    │  ┌──────────▼─────────────┐  │ │
│                    │  │  Shared OcrEngine      │  │ │
│                    │  │  (Arc<Mutex<Option>>)  │  │ │
│                    │  └────────────────────────┘  │ │
│                    └──────────────────────────────┘ │
└─────────────────────────────────────────────────────┘
```

### Key Design Decisions

1. **Framework**: `axum` — the most mature Rust async web framework, built on `tokio` and `hyper`, already compatible with the project's async runtime.

2. **Server lifecycle**: The HTTP server runs in a background `tokio::task` within the Tauri process. It is started/stopped via Tauri commands from the frontend or auto-started on app launch based on config.

3. **Shared state**: The axum server gets its own `Arc<Mutex<Option<OcrEngine>>>` (cloned from `AppState`), so it can share the same engine instance with Tauri commands. This avoids duplicate model loading.

4. **Port**: Default `9720`, configurable in `AppConfig`.

5. **Auth**: Optional API key in `AppConfig`. When set, all endpoints (except `/health`) require `Authorization: Bearer <key>` header.

6. **File size limit**: 20MB default, configurable.

## API Specification

### `POST /api/v1/ocr`

Perform OCR on an uploaded image.

**Request** (multipart/form-data):

```
POST /api/v1/ocr
Content-Type: multipart/form-data
Authorization: Bearer <api_key>   (optional)

Form fields:
  image:    (file, required) Image file (PNG/JPEG/BMP/WEBP/TIFF)
  model:    (string, optional) Model version: "ppocr-v6" (default), "ppocr-v5", "ppocr-v4"
```

**Request** (application/json, base64):

```
POST /api/v1/ocr
Content-Type: application/json
Authorization: Bearer <api_key>   (optional)

{
  "image": "base64_encoded_image_data",
  "model": "ppocr-v6"          (optional)
}
```

**Response** (200 OK):

```json
{
  "success": true,
  "data": {
    "text": "所有识别文本的拼接结果",
    "regions": [
      {
        "text": "识别的文本行",
        "confidence": 0.987,
        "bbox": [[10, 20], [100, 20], [100, 40], [10, 40]]
      }
    ],
    "total_time_ms": 156
  },
  "model": "ppocr-v6"
}
```

**Error Response** (400):

```json
{
  "success": false,
  "error": {
    "code": "INVALID_IMAGE",
    "message": "Failed to decode image: unsupported format"
  }
}
```

**Error Response** (413):

```json
{
  "success": false,
  "error": {
    "code": "FILE_TOO_LARGE",
    "message": "File size exceeds 20MB limit"
  }
}
```

**Error Response** (401):

```json
{
  "success": false,
  "error": {
    "code": "UNAUTHORIZED",
    "message": "Invalid or missing API key"
  }
}
```

### `GET /api/v1/health`

Health check endpoint.

**Response** (200 OK):

```json
{
  "status": "ok",
  "model_loaded": true,
  "active_model": "ppocr-v6",
  "version": "1.1.0"
}
```

### `GET /api/v1/info`

Service information.

**Response** (200 OK):

```json
{
  "name": "LynxOCR API",
  "version": "1.1.0",
  "engine": "PaddleOCR ONNX",
  "available_models": ["ppocr-v4", "ppocr-v5", "ppocr-v6"],
  "active_model": "ppocr-v6",
  "max_file_size_mb": 20
}
```

### Error Codes Reference

| HTTP Status | Code                  | Description             |
| ----------- | --------------------- | ----------------------- |
| 400         | `INVALID_IMAGE`       | Image decode failed     |
| 400         | `NO_IMAGE`            | No image provided       |
| 400         | `INVALID_MODEL`       | Unknown model version   |
| 400         | `MODEL_NOT_INSTALLED` | Model not downloaded    |
| 401         | `UNAUTHORIZED`        | Missing/invalid API key |
| 413         | `FILE_TOO_LARGE`      | File exceeds size limit |
| 500         | `OCR_ERROR`           | OCR engine error        |
| 500         | `INTERNAL_ERROR`      | Unexpected server error |

## Verification

1. `cargo check` passes in `src-tauri/`
2. `bun run tauri build` succeeds
3. API server starts via frontend toggle
4. `curl -X POST http://localhost:9720/api/v1/ocr -F "image=@test.png"` returns JSON with text
5. `curl http://localhost:9720/api/v1/health` returns `{"status": "ok"}`
6. API key auth blocks unauthorized requests
7. File size limit rejects oversized uploads
8. Model switching works (specify different model in request)
9. English i18n displays correctly when language is switched