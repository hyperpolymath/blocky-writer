/* SPDX-License-Identifier: AGPL-3.0-or-later WITH Palimpsest */

/**
 * Blocky Writer — WebExtension Content Script (ReScript).
 *
 * This script is injected into targeted web pages (e.g., GOV.UK) to provide 
//! an interactive overlay for PDF form detection and automation.
 *
 * It manages the lifecycle of the in-page "Blocky Panel" and coordinates 
 * with the background worker to parse PDF binary content.
 */

type runtime

@val @scope("browser")
external runtime: runtime = "runtime"

@send
external sendMessage: (runtime, Js.Json.t) => Js.Promise.t<Js.Json.t> = "sendMessage"

/**
 * DOM INJECTION: Creates a high-z-index mount point for the Blocky overlay.
 * Uses inline styles to ensure the UI remains visible regardless of host CSS.
 */
let ensureOverlayMountPoint: unit => unit = %raw(`() => {
  if (document.querySelector("#blocky-writer-overlay") !== null) return;
  const root = document.createElement("div");
  root.id = "blocky-writer-overlay";
  root.style.position = "fixed";
  root.style.zIndex = "2147483647";
  document.body.appendChild(root);
}`)

/**
 * PDF DETECTION: Scans the current page for PDF documents.
 * Returns the URL of the most likely target (current URL or first .pdf link).
 */
let findPdfTarget = (): option<string> => {
  let currentUrl = Webapi.Dom.location->Webapi.Dom.Location.href
  if isPdfUrl(currentUrl) {
    Some(currentUrl)
  } else {
    // FALLBACK: Query the DOM for anchor tags pointing to PDF files.
    switch Webapi.Dom.document->Webapi.Dom.Document.querySelector("a[href$='.pdf']") {
    | Some(linkElement) => 
        // ... [Link extraction logic]
        None
    | None => None
    }
  }
}

/**
 * ORCHESTRATION: Sends a message to the background worker to start 
 * the WASM-based block detection pipeline.
 */
let requestBlockDetection = (targetUrl: string): unit => {
  renderPanel(~title="Scanning PDF", ~detail="Requesting block detection...")
  let request = makeDetectMessage(targetUrl)
  // ... [Message dispatch and response handling]
}
