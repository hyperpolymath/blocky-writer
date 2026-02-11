/* SPDX-License-Identifier: AGPL-3.0-or-later WITH Palimpsest */

type block = {
  label: string,
  x: float,
  y: float,
  width: float,
  height: float,
}

type arrayBuffer = Js.Typed_array.ArrayBuffer.t

type wasmModule

@module("../../rust/pdftool_core/pkg/pdftool_core.js")
external wasm: wasmModule = "default"

@send
external detectBlocksRaw: (wasmModule, arrayBuffer) => Js.Promise.t<array<block>> = "detect_blocks"

@send
external fillBlocksRaw: (wasmModule, arrayBuffer, array<block>, Js.Dict.t<string>) => Js.Promise.t<arrayBuffer> = "fill_blocks"

let detectBlocks = (pdfData: arrayBuffer): Js.Promise.t<array<block>> =>
  detectBlocksRaw(wasm, pdfData)

let fillBlocks = (
  pdfData: arrayBuffer,
  blocks: array<block>,
  fields: Js.Dict.t<string>,
): Js.Promise.t<arrayBuffer> =>
  fillBlocksRaw(wasm, pdfData, blocks, fields)
