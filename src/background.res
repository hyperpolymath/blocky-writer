/* SPDX-License-Identifier: AGPL-3.0-or-later WITH Palimpsest */

/**
 * Blocky Writer — Background Service Worker (ReScript).
 *
 * This module handles the background tasks for the Blocky Writer WebExtension.
 * It primarily manages the orchestration of PDF processing and block detection,
 * bridging the communication between the UI (popup/content scripts) and 
 * the high-assurance parsing engine.
 */

type runtime
type onMessage

// FFI: Bindings to the browser.runtime WebExtension API.
@val @scope("browser")
external runtime: runtime = "runtime"

@get
external onMessage: runtime => onMessage = "onMessage"

@send
external addListener: (onMessage, Js.Json.t => Js.Promise.t<Js.Json.t>) => unit = "addListener"

// UTILITY: Coerce any value to a generic JSON object (Unsafe).
let unsafeJson = (value: 'a): Js.Json.t => Obj.magic(value)

/**
 * DETECT PIPELINE: Ingests a URL, fetches the binary content (PDF), 
 * and extracts structural blocks for the editor.
 */
let detectFromUrl = (url: string): Js.Promise.t<Js.Json.t> => {
  let detectPromise =
    Js.Promise2.then(Webapi.Fetch.fetch(url), response =>
        Js.Promise2.then(response->Webapi.Fetch.Response.arrayBuffer, pdfBuffer =>
          // PASS TO TOOL: Hand off the raw buffer to the WASM-based detection engine.
          Js.Promise2.then(PdfTool.detectBlocks(pdfBuffer), blocks =>
            Js.Promise.resolve(makeResponse(~ok=true, ~blocks))
          )
        )
      )

  // ERROR HANDLING: Decodes native browser/network errors into structured app errors.
  Js.Promise2.catch(detectPromise, err => {
    let decoded = decodeError(err)
    Js.log2("Block detection failed", decoded)
    Js.Promise.resolve(makeResponse(~ok=false, ~error=Some(decoded.message), ~code=Some("BW_BG_DETECT_FAILED")))
  })
}

// MAIN LISTENER: Responds to messages from the content script or popup.
let _ =
  addListener(onMessage(runtime), message =>
    switch decodeDetectRequest(message) {
    | Some(url) => detectFromUrl(url)
    | None =>
      Js.Promise.resolve(
        makeResponse(~ok=false, ~error=Some("unsupported action"), ~code=Some("BW_BG_UNSUPPORTED_ACTION"))
      )
    }
  )
