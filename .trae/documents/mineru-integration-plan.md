# MinerU Integration Plan

## Summary

The core MinerU integration (engine, config, API server, OCR routing, model settings, i18n) has already been implemented. This plan covers the remaining gaps: OCR page export dropdown, API settings page examples, DOCX binary export, and MinerU routing for bytes/PDF/screenshot OCR commands.

## Current State Analysis

### Already Implemented
| Component | File | Status |
|---|---|---|
| MinerU API client (flash_extract + extract) | `src-tauri/src/engine/mineru/mod.rs` | Done |
| AppConfig MinerU fields | `src-tauri/src/config/app_config.rs` | Done |
| API server MinerU routing | `src-tauri/src/api/server.rs` | Done |
| API types (format, mineru_mode) | `src-tauri/src/api/types.rs` | Done |
| OCR commands MinerU routing | `src-tauri/src/commands/ocr.rs` | Done |
| API commands MinerU config pass-through | `src-tauri/src/commands/api.rs` | Done |
| AppState mineru_client | `src-tauri/src/lib.rs` | Done |
| OCR page: model dropdown, format selector, result display | `src/app/ocr/page.tsx` | Done |
| Model settings: MinerU card | `src/app/settings/model-settings.tsx` | Done |
| Frontend types (AppConfig, OcrResult) | `src/types/index.ts` | Done |
| i18n strings (zh + en) | `src/lib/app-context.tsx` | Done |
| Model manager: mineru entry | `src-tauri/src/engine/model_manager.rs` | Done |
| Model commands: mineru skip | `src-tauri/src/commands/model.rs` | Done |

### Gaps to Fill
1. **OCR page export dropdown** — `handleExportMineruFormat` exists but is not wired to the dropdown menu. Need to add MinerU-specific format options (HTML, LaTeX, DOCX, JSON) when MinerU is active model.
2. **API settings page** — no MinerU example usage curl commands.
3. **DOCX binary export** — `write_text_file` writes a `String`, but DOCX is binary. Current `handleExportMineruFormat` writes base64 as-is (broken). Need a proper binary file write command.
4. **`ocr_recognize_bytes`** — does not route to MinerU (only `ocr_recognize` does).
5. **`ocr_recognize_pdf`** — does not route to MinerU (would fail on `is_ppocr_installed_at` check).
6. **`ocr_screenshot_region`** — does not route to MinerU.

## Proposed Changes

### Phase 1: OCR Page Export Dropdown

**File:** `src/app/ocr/page.tsx`

**What:** Add MinerU-specific export options to the `<DropdownMenuContent>` when `isMineru` is true.

**Why:** Users need to export MinerU results in HTML, LaTeX, DOCX, and JSON formats directly from the UI. The `handleExportMineruFormat` function already exists but is not wired to any UI element.

**How:**
- After the existing `.md` export `<DropdownMenuItem>`, add conditional items for MinerU formats:
  ```tsx
  {isMineru && hasCompleted && (
    <>
      <DropdownMenuItem onClick={() => handleExportMineruFormat("html")}>
        <FileTextIcon className="size-4 mr-2" />
        {t("ocr.exportHtml")} (.html)
      </DropdownMenuItem>
      <DropdownMenuItem onClick={() => handleExportMineruFormat("latex")}>
        <FileTextIcon className="size-4 mr-2" />
        {t("ocr.exportLatex")} (.tex)
      </DropdownMenuItem>
      <DropdownMenuItem onClick={() => handleExportMineruFormat("docx")}>
        <FileTextIcon className="size-4 mr-2" />
        {t("ocr.exportDocx")} (.docx)
      </DropdownMenuItem>
      <DropdownMenuItem onClick={() => handleExportMineruFormat("json")}>
        <FileTextIcon className="size-4 mr-2" />
        {t("ocr.exportJson")} (.json)
      </DropdownMenuItem>
    </>
  )}
  ```

### Phase 2: Binary File Write Command

**File:** `src-tauri/src/commands/ocr.rs` (new command)

**What:** Add a `write_binary_file` Tauri command that accepts base64-encoded data and writes the decoded binary to disk.

**Why:** The current `write_text_file` uses `String` content which corrupts binary data. DOCX export requires binary file writing.

**How:**
```rust
#[tauri::command]
pub fn write_binary_file(path: String, base64_content: String) -> Result<(), String> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&base64_content)
        .map_err(|e| format!("Failed to decode base64: {e}"))?;
    std::fs::write(&path, &bytes)
        .map_err(|e| format!("Failed to write file: {e}"))
}
```

**File:** `src-tauri/src/lib.rs` — register `write_binary_file` in `invoke_handler!`.

**File:** `src/app/ocr/page.tsx` — update `handleExportMineruFormat` to:
- For DOCX: use `invoke("write_binary_file", ...)` instead of `write_text_file`
- For non-DOCX: keep using `write_text_file`

### Phase 3: API Settings Page MinerU Examples

**File:** `src/app/api-settings/page.tsx`

**What:** Add MinerU-specific curl example usage in the example usage card.

**Why:** Users need to know how to call MinerU through the API. The API server already supports `model=mineru`, `format`, and `mineru_mode` parameters.

**How:**
- Add new i18n keys: `api.mineruCurl` (zh: "MinerU 快读识别", en: "MinerU Flash Extract"), `api.mineruExtractCurl` (zh: "MinerU 精确提取", en: "MinerU Extract")
- Add two new example blocks after the existing "specify model" example:
  1. MinerU flash extract (multipart, no auth needed):
     ```
     curl -X POST http://localhost:{port}/api/v1/ocr \
       -F "image=@/path/to/file.png" \
       -F "model=mineru"
     ```
  2. MinerU extract with format (JSON, with auth):
     ```
     curl -X POST http://localhost:{port}/api/v1/ocr \
       -H "Content-Type: application/json" \
       -d '{"image":"base64...","model":"mineru","format":"html","mineru_mode":"extract"}'
     ```

### Phase 4: MinerU Routing for Bytes/PDF/Screenshot OCR

**File:** `src-tauri/src/commands/ocr.rs`

**What:**
1. `ocr_recognize_bytes` — add MinerU routing at the top (same pattern as `ocr_recognize`).
2. `ocr_recognize_pdf` — add MinerU routing at the top (MinerU handles PDFs natively, no need to render pages).
3. `ocr_screenshot_region` — add MinerU routing at the top.

**Why:** These commands are callable from the frontend and should support MinerU model selection consistently.

**How:**
- `ocr_recognize_bytes`: Add early return for `model_version == "mineru"` that saves bytes to temp file, then calls `ocr_recognize_mineru_impl`.
- `ocr_recognize_pdf`: Add early return for `model_version == "mineru"` that calls MinerU extract directly on the PDF file (single result, not per-page).
- `ocr_screenshot_region`: Add early return for `model_version == "mineru"` that saves cropped image to temp file, then calls `ocr_recognize_mineru_impl`.

## Assumptions & Decisions

1. **MinerU PDF handling**: For `ocr_recognize_pdf` with MinerU, the entire PDF is sent to MinerU and returns a single result (not per-page). This is consistent with MinerU's native PDF support.
2. **MinerU screenshot OCR**: This is a corner case — MinerU is cloud-based, so screenshot OCR with MinerU requires network. The UI already shows a warning when no token is configured.
3. **DOCX export**: The Rust backend already has `base64` as a dependency (in Cargo.toml), so `write_binary_file` can decode base64 without adding new deps.
4. **No new dependencies needed**: All required crates are already in Cargo.toml.

## Verification

1. `cargo check` from `src-tauri/` — verify Rust code compiles
2. `bun run typecheck` — verify TypeScript types
3. `bun run lint` — verify ESLint passes
4. `bun run tauri build` — verify full production build
5. Update `docs/ChangeLog.md` with the changes