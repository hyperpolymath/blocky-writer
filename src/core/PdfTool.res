/* SPDX-License-Identifier: AGPL-3.0-or-later WITH Palimpsest */

/**
 * PdfTool — WASM-Accelerated PDF Manipulation (ReScript).
 *
 * This module provides the high-level bridge between the ReScript frontend 
 * and the Rust-based `pdftool_core` WASM module. It handles the low-level 
 * buffer conversions and lazy initialization of the WASM runtime.
 */

// SCHEMA: Represents a detected PDF form widget or text block.
type block = {
  label: string,
  x: float,
  y: float,
  width: float,
  height: float,
}

// FFI: Bindings to the generated WASM glue code.
@module("../../rust/pdftool_core/pkg/pdftool_core.js")
external initWasm: unit => Js.Promise.t<unit> = "default"

@module("../../rust/pdftool_core/pkg/pdftool_core.js")
external detectBlocksNative: uint8Array => array<block> = "detect_blocks"

/**
 * DETECTION: Identifies interactive blocks within a PDF binary.
 * Automatically ensures the WASM runtime is initialized before execution.
 */
let detectBlocks = (pdfData: arrayBuffer): Js.Promise.t<array<block>> => {
  let bytes = Js.Typed_array.Uint8Array.fromBuffer(toNativeArrayBuffer(pdfData))
  Js.Promise2.then(ensureInitialized(), _ => Js.Promise.resolve(detectBlocksNative(bytes)))
}

/**
 * FILL: Merges user-provided field data into the PDF blocks.
 * Returns a new ArrayBuffer containing the modified PDF.
 */
let fillBlocks = (
  pdfData: arrayBuffer,
  blocks: array<block>,
  fields: Js.Dict.t<string>,
): Js.Promise.t<arrayBuffer> => {
  // ... [Implementation of buffer-to-wasm-to-buffer transformation]
}
