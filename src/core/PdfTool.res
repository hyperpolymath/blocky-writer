/* SPDX-License-Identifier: AGPL-3.0-or-later WITH Palimpsest */

type block = {
  label: string,
  x: float,
  y: float,
  width: float,
  height: float,
}

type arrayBuffer = Webapi.Fetch.arrayBuffer
type uint8Array = Js.Typed_array.Uint8Array.t

@module("../../rust/pdftool_core/pkg/pdftool_core.js")
external initWasm: unit => Js.Promise.t<unit> = "default"

@module("../../rust/pdftool_core/pkg/pdftool_core.js")
external detectBlocksNative: uint8Array => array<block> = "detect_blocks"

@module("../../rust/pdftool_core/pkg/pdftool_core.js")
external fillBlocksNative: (uint8Array, array<block>, Js.Dict.t<string>) => uint8Array = "fill_blocks"

let initState: ref<option<Js.Promise.t<unit>>> = ref(None)

let toNativeArrayBuffer = (buffer: arrayBuffer): Js.Typed_array.ArrayBuffer.t => Obj.magic(buffer)

let fromNativeArrayBuffer = (buffer: Js.Typed_array.ArrayBuffer.t): arrayBuffer => Obj.magic(buffer)

let ensureInitialized = (): Js.Promise.t<unit> => {
  switch initState.contents {
  | Some(promise) => promise
  | None =>
    let promise = initWasm()
    initState.contents = Some(promise)
    promise
  }
}

let detectBlocks = (pdfData: arrayBuffer): Js.Promise.t<array<block>> => {
  let bytes = Js.Typed_array.Uint8Array.fromBuffer(toNativeArrayBuffer(pdfData))
  let detection = Js.Promise2.then(ensureInitialized(), _ => Js.Promise.resolve(detectBlocksNative(bytes)))
  Js.Promise2.catch(detection, err => {
    Js.log2("detectBlocks error", err)
    Js.Promise.resolve([])
  })
}

let fillBlocks = (
  pdfData: arrayBuffer,
  blocks: array<block>,
  fields: Js.Dict.t<string>,
): Js.Promise.t<arrayBuffer> => {
  let bytes = Js.Typed_array.Uint8Array.fromBuffer(toNativeArrayBuffer(pdfData))
  let fill = Js.Promise2.then(ensureInitialized(), _ => {
    let filled = fillBlocksNative(bytes, blocks, fields)
    Js.Promise.resolve(fromNativeArrayBuffer(Js.Typed_array.Uint8Array.buffer(filled)))
  })
  Js.Promise2.catch(fill, err => {
    Js.log2("fillBlocks error", err)
    Js.Promise.resolve(fromNativeArrayBuffer(Js.Typed_array.ArrayBuffer.make(0)))
  })
}
