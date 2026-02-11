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
let errorToString: 'a => string = %raw(`(error) => {
  if (error && typeof error === "object" && "message" in error && error.message) {
    return String(error.message);
  }
  try {
    return String(error);
  } catch {
    return "unknown error";
  }
}`)

type decodedError = {
  message: string,
  code: option<string>,
  context: option<string>,
}

let decodeError = (value: 'a): decodedError => {
  let payload: Js.Json.t = Obj.magic(value)
  switch Js.Json.decodeObject(payload) {
  | Some(obj) =>
    let message =
      obj
      ->Js.Dict.get("message")
      ->Belt.Option.flatMap(Js.Json.decodeString)
      ->Belt.Option.getWithDefault(errorToString(value))
    let code = obj->Js.Dict.get("code")->Belt.Option.flatMap(Js.Json.decodeString)
    let context = obj->Js.Dict.get("context")->Belt.Option.flatMap(Js.Json.decodeString)
    {message, code, context}
  | None => {message: errorToString(value), code: None, context: None}
  }
}

let makeResponse = (
  ~ok: bool,
  ~blocks: array<PdfTool.block>=[],
  ~error: option<string>=None,
  ~code: option<string>=None,
  ~context: option<string>=None,
): Js.Json.t => {
  let response = Js.Dict.empty()
  Js.Dict.set(response, "ok", Js.Json.boolean(ok))
  Js.Dict.set(response, "blocks", unsafeJson(blocks))
  switch error {
  | Some(message) => Js.Dict.set(response, "error", Js.Json.string(message))
  | None => ()
  }
  switch code {
  | Some(value) => Js.Dict.set(response, "code", Js.Json.string(value))
  | None => ()
  }
  switch context {
  | Some(value) => Js.Dict.set(response, "context", Js.Json.string(value))
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
    let decoded = decodeError(err)
    let code = switch decoded.code {
    | Some(value) => Some(value)
    | None => Some("BW_BG_DETECT_FAILED")
    }
    Js.log2("detect blocks failed", decoded)
    Js.Promise.resolve(makeResponse(~ok=false, ~error=Some(decoded.message), ~code, ~context=decoded.context))
  })
}

let _ =
  addListener(onMessage(runtime), message =>
    switch decodeDetectRequest(message) {
    | Some(url) => detectFromUrl(url)
    | None =>
      Js.Promise.resolve(
        makeResponse(
          ~ok=false,
          ~error=Some("unsupported action"),
          ~code=Some("BW_BG_UNSUPPORTED_ACTION"),
          ~context=Some("expected action=detectBlocks with non-empty url"),
        ),
      )
    }
  )

Js.log("Blocky Writer background service worker loaded")
