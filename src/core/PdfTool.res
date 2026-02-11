/* SPDX-License-Identifier: AGPL-3.0-or-later WITH Palimpsest */

type block = {
  label: string,
  x: float,
  y: float,
  width: float,
  height: float,
}

type wasmModule

@module("../../rust/pdftool_core/pkg/pdftool_core.js")
external wasm: wasmModule = "default"

@send
external detectBlocksRaw: (wasmModule, arrayBuffer) => Js.Promise.t<array<block>> = "detect_blocks"

@send
external fillBlocksRaw: (wasmModule, arrayBuffer, array<block>, Js.Dict.t<string>) => Js.Promise.t<arrayBuffer> = "fill_blocks"

let detectBlocks = (pdfData: arrayBuffer): Js.Promise.t<array<block>> =>
  detectBlocksRaw(wasm, pdfData)
  ->Js.Promise.catch(err => {
    Js.log2("detectBlocks error", err)
    Js.Promise.resolve([||])
  })

let fillBlocks = (
  pdfData: arrayBuffer,
  blocks: array<block>,
  fields: Js.Dict.t<string>,
): Js.Promise.t<arrayBuffer> =>
  fillBlocksRaw(wasm, pdfData, blocks, fields)
  ->Js.Promise.catch(err => {
    Js.log2("fillBlocks error", err)
    Js.Promise.resolve(ArrayBuffer.make(0))
  })
