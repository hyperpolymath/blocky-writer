/* SPDX-License-Identifier: AGPL-3.0-or-later WITH Palimpsest */

/**
 * Blocky Writer — WebExtension Popup (ReScript/React).
 *
 * This module implements the primary user interface for the extension.
 * It coordinates with the background worker to scan the active tab's PDF
 * and provides a form interface for data entry.
 */

type runtime
type tabsApi
// ... [other types]

@val @scope("browser") external runtime: runtime = "runtime"
@val @scope("browser") external tabsApi: tabsApi = "tabs"

/**
 * DOWNLOADER: Triggers a browser-managed file download for the filled PDF.
 * Uses a transient object URL and a hidden anchor element.
 */
let triggerDownload: (string, string) => unit = %raw(`(url, filename) => {
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  anchor.click();
  anchor.remove();
}`)

/**
 * FILL ENGINE: Ingests user-provided fields and generates a new PDF binary.
 *
 * PIPELINE:
 * 1. GET: Fetch the original PDF from the active tab's URL.
 * 2. TRANSFORM: Pass the buffer and field map to the `PdfTool` (WASM).
 * 3. EXPORT: Convert the resulting ArrayBuffer into a Blob/URL for download.
 */
let fillPdfAndDownload = (
  ~blocks: array<PdfTool.block>,
  ~fields: Js.Dict.t<string>,
): Js.Promise.t<fillResult> => {
  // ... [Async chain: Fetch -> fillBlocks -> createObjectURL -> triggerDownload]
}

/**
 * UI ROOT: The primary React component.
 * Manages the state of the detection process (`blocks`, `status`, `isLoading`).
 */
module App = {
  @react.component
  let make = () => {
    // ... [React state and effect hooks]
    <div>
      <h1>{React.string("Blocky Writer")}</h1>
      <p>{React.string(status)}</p>
      <button onClick={_ => refresh()}>{React.string("Refresh Detection")}</button>
      <FormFiller blocks={blocks} onFill />
    </div>
  }
}
