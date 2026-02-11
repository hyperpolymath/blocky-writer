/* SPDX-License-Identifier: AGPL-3.0-or-later WITH Palimpsest */

type runtime
type onMessage

@val @scope("browser")
external runtime: runtime = "runtime"

@get
external onMessage: runtime => onMessage = "onMessage"

@send
external addListener: (onMessage, Js.Json.t => Js.Promise.t<Js.Json.t>) => unit = "addListener"

let unsafeJson = (value: 'a): Js.Json.t => Obj.magic(value)

let makeResponse = (
  ~ok: bool,
  ~blocks: array<PdfTool.block>=[],
  ~error: option<string>=None,
): Js.Json.t => {
  let response = Js.Dict.empty()
  Js.Dict.set(response, "ok", Js.Json.boolean(ok))
  Js.Dict.set(response, "blocks", unsafeJson(blocks))
  switch error {
  | Some(message) => Js.Dict.set(response, "error", Js.Json.string(message))
  | None => ()
  }
  unsafeJson(response)
}

let decodeDetectRequest = (message: Js.Json.t): option<string> => {
  switch Js.Json.decodeObject(message) {
  | Some(payload) =>
    let action =
      payload->Js.Dict.get("action")->Belt.Option.flatMap(Js.Json.decodeString)->Belt.Option.getWithDefault("")
    let url =
      payload->Js.Dict.get("url")->Belt.Option.flatMap(Js.Json.decodeString)->Belt.Option.getWithDefault("")
    if action == "detectBlocks" && url != "" {
      Some(url)
    } else {
      None
    }
  | None => None
  }
}

let detectFromUrl = (url: string): Js.Promise.t<Js.Json.t> => {
  let detectPromise =
    Js.Promise2.then(Webapi.Fetch.fetch(url), response =>
        Js.Promise2.then(response->Webapi.Fetch.Response.arrayBuffer, pdfBuffer =>
          Js.Promise2.then(PdfTool.detectBlocks(pdfBuffer), blocks =>
            Js.Promise.resolve(makeResponse(~ok=true, ~blocks))
          )
        )
      )

  Js.Promise2.catch(detectPromise, err => {
    Js.log2("detect blocks failed", err)
    Js.Promise.resolve(makeResponse(~ok=false, ~error=Some("detectBlocks failed")))
  })
}

let _ =
  addListener(onMessage(runtime), message =>
    switch decodeDetectRequest(message) {
    | Some(url) => detectFromUrl(url)
    | None => Js.Promise.resolve(makeResponse(~ok=false, ~error=Some("unsupported action")))
    }
  )

Js.log("Blocky Writer background service worker loaded")
