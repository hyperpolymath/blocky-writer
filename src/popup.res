/* SPDX-License-Identifier: AGPL-3.0-or-later WITH Palimpsest */

type runtime
type tabsApi
type tab = Js.Json.t
type extensionError = {
  message: string,
  code: option<string>,
  context: option<string>,
}
type urlLookup = result<string, extensionError>
type fillSuccess = {
  bytes: int,
  filename: string,
}
type fillResult = result<fillSuccess, extensionError>

@val @scope("browser")
external runtime: runtime = "runtime"

@val @scope("browser")
external tabsApi: tabsApi = "tabs"

@send
external sendMessage: (runtime, Js.Json.t) => Js.Promise.t<Js.Json.t> = "sendMessage"

@send
external queryTabs: (tabsApi, Js.Json.t) => Js.Promise.t<array<tab>> = "query"

let triggerDownload: (string, string) => unit = %raw(`(url, filename) => {
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  anchor.rel = "noopener";
  anchor.style.display = "none";
  document.body.appendChild(anchor);
  anchor.click();
  anchor.remove();
}`)

let makeFilledFilename: unit => string = %raw(`() => "blocky-writer-filled-" + Date.now() + ".pdf"`)
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

let unsafeJson = (value: 'a): Js.Json.t => Obj.magic(value)

let decodeError = (value: 'a): extensionError => {
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

let withFallbackCode = (error: extensionError, fallbackCode: string): extensionError =>
  switch error.code {
  | Some(_) => error
  | None => {message: error.message, code: Some(fallbackCode), context: error.context}
  }

let makeError = (
  ~message: string,
  ~code: option<string>=None,
  ~context: option<string>=None,
): extensionError => {
  message,
  code,
  context,
}

let formatError = (error: extensionError): string => {
  let codePrefix =
    switch error.code {
    | Some(code) => "[" ++ code ++ "] "
    | None => ""
    }
  let contextSuffix =
    switch error.context {
    | Some(context) => " (" ++ context ++ ")"
    | None => ""
    }
  codePrefix ++ error.message ++ contextSuffix
}

let makeTabQuery = (): Js.Json.t => {
  let payload = Js.Dict.empty()
  Js.Dict.set(payload, "active", Js.Json.boolean(true))
  Js.Dict.set(payload, "currentWindow", Js.Json.boolean(true))
  unsafeJson(payload)
}

let makeDetectMessage = (url: string): Js.Json.t => {
  let payload = Js.Dict.empty()
  Js.Dict.set(payload, "action", Js.Json.string("detectBlocks"))
  Js.Dict.set(payload, "url", Js.Json.string(url))
  unsafeJson(payload)
}

let looksLikePdfUrl = (url: string): bool => {
  let lower = url->Js.String2.toLowerCase
  lower->Js.String2.endsWith(".pdf")
  || lower->Js.String2.includes(".pdf?")
  || lower->Js.String2.includes(".pdf#")
}

let tabUrl = (entry: tab): option<string> =>
  switch Js.Json.decodeObject(entry) {
  | Some(obj) => obj->Js.Dict.get("url")->Belt.Option.flatMap(Js.Json.decodeString)
  | None => None
  }

let getActivePdfUrl = (): Js.Promise.t<urlLookup> => {
  let lookupPromise =
    Js.Promise2.then(queryTabs(tabsApi, makeTabQuery()), entries => {
      if Belt.Array.length(entries) == 0 {
        Js.Promise.resolve(Error(makeError(~message="no active tab found", ~code=Some("BW_POPUP_TAB_NOT_FOUND"))))
      } else {
        switch tabUrl(entries[0]) {
        | None =>
          Js.Promise.resolve(
            Error(makeError(~message="active tab URL unavailable", ~code=Some("BW_POPUP_TAB_URL_MISSING"))),
          )
        | Some(url) =>
          if looksLikePdfUrl(url) {
            Js.Promise.resolve(Ok(url))
          } else {
            Js.Promise.resolve(
              Error(
                makeError(
                  ~message="active tab is not a PDF URL",
                  ~code=Some("BW_POPUP_TAB_NOT_PDF"),
                  ~context=Some(url),
                ),
              ),
            )
          }
        }
      }
    })

  Js.Promise2.catch(lookupPromise, err => {
    let decoded = decodeError(err)->withFallbackCode("BW_POPUP_TAB_QUERY_FAILED")
    Js.log2("active tab lookup failed", decoded)
    Js.Promise.resolve(Error(decoded))
  })
}

type detectResult = {
  ok: bool,
  blocks: array<PdfTool.block>,
  error: option<extensionError>,
}

let decodeDetectResult = (payload: Js.Json.t): detectResult => {
  switch Js.Json.decodeObject(payload) {
  | Some(obj) =>
    let ok = obj->Js.Dict.get("ok")->Belt.Option.flatMap(Js.Json.decodeBoolean)->Belt.Option.getWithDefault(false)
    let blocks: array<PdfTool.block> =
      switch obj->Js.Dict.get("blocks") {
      | Some(rawBlocks) =>
        switch Js.Json.decodeArray(rawBlocks) {
        | Some(_) => Obj.magic(rawBlocks)
        | None => []
        }
      | None => []
      }
    let code = obj->Js.Dict.get("code")->Belt.Option.flatMap(Js.Json.decodeString)
    let context = obj->Js.Dict.get("context")->Belt.Option.flatMap(Js.Json.decodeString)
    let error =
      switch obj->Js.Dict.get("error")->Belt.Option.flatMap(Js.Json.decodeString) {
      | Some(message) => Some({message, code, context})
      | None =>
        if ok {
          None
        } else {
          Some({
            message: "background worker returned an unspecified failure",
            code,
            context,
          })
        }
      }
    {ok, blocks, error}
  | None =>
    {
      ok: false,
      blocks: [],
      error: Some(
        makeError(~message="invalid background response", ~code=Some("BW_POPUP_BG_RESPONSE_INVALID")),
      ),
    }
  }
}

let detectBlocksForActiveTab = (): Js.Promise.t<detectResult> => {
  let detectPromise =
    Js.Promise2.then(getActivePdfUrl(), lookup => {
      switch lookup {
      | Error(error) => Js.Promise.resolve({ok: false, blocks: [], error: Some(error)})
      | Ok(url) =>
        Js.Promise2.then(sendMessage(runtime, makeDetectMessage(url)), response =>
          Js.Promise.resolve(decodeDetectResult(response))
        )
      }
    })

  Js.Promise2.catch(detectPromise, err => {
    let decoded = decodeError(err)->withFallbackCode("BW_POPUP_DETECT_REQUEST_FAILED")
    Js.log2("popup detect failed", decoded)
    Js.Promise.resolve({ok: false, blocks: [], error: Some(decoded)})
  })
}

let fillPdfAndDownload = (
  ~blocks: array<PdfTool.block>,
  ~fields: Js.Dict.t<string>,
): Js.Promise.t<fillResult> => {
  let fillPromise =
    Js.Promise2.then(getActivePdfUrl(), lookup => {
      switch lookup {
      | Error(error) => Js.Promise.resolve(Error(error))
      | Ok(url) =>
        Js.Promise2.then(Webapi.Fetch.fetch(url), response =>
          Js.Promise2.then(response->Webapi.Fetch.Response.arrayBuffer, pdfBuffer =>
            Js.Promise2.then(PdfTool.fillBlocks(pdfBuffer, blocks, fields), filledBuffer => {
              let nativeBuffer: Js.Typed_array.ArrayBuffer.t = Obj.magic(filledBuffer)
              let byteLength = Js.Typed_array.ArrayBuffer.byteLength(nativeBuffer)
              if byteLength == 0 {
                Js.Promise.resolve(
                  Error(makeError(~message="fill_blocks returned an empty PDF payload", ~code=Some("BW_POPUP_FILL_EMPTY"))),
                )
              } else {
                let blobPart = Webapi.Blob.arrayBufferToBlobPart(nativeBuffer)
                let blob = Webapi.Blob.makeWithOptions(
                  [blobPart],
                  Webapi.Blob.makeBlobPropertyBag(~_type="application/pdf", ()),
                )
                let objectUrl = Webapi.Url.createObjectURLFromBlob(blob)
                let filename = makeFilledFilename()
                triggerDownload(objectUrl, filename)
                let _timeoutId = Js.Global.setTimeout(() => Webapi.Url.revokeObjectURL(objectUrl), 60000)
                Js.Promise.resolve(Ok({bytes: byteLength, filename}))
              }
            })
          )
        )
      }
    })

  Js.Promise2.catch(fillPromise, err => {
    let decoded = decodeError(err)->withFallbackCode("BW_POPUP_FILL_REQUEST_FAILED")
    Js.log2("fill and download failed", decoded)
    Js.Promise.resolve(Error(decoded))
  })
}

module App = {
  @react.component
  let make = () => {
    let (blocks, setBlocks) = React.useState(() => [])
    let (status, setStatus) = React.useState(() => "Waiting to detect PDF fields")
    let (isLoading, setIsLoading) = React.useState(() => false)

    let refresh = () => {
      setIsLoading(_ => true)
      setStatus(_ => "Detecting blocks from active PDF tab...")
      let detectPromise =
        Js.Promise2.then(detectBlocksForActiveTab(), result => {
          setIsLoading(_ => false)
          if result.ok {
            let count = Belt.Array.length(result.blocks)
            setBlocks(_ => result.blocks)
            setStatus(_ =>
              if count == 0 {
                "No form widgets detected in this PDF"
              } else {
                "Detected " ++ Belt.Int.toString(count) ++ " blocks"
              }
            )
          } else {
            setBlocks(_ => [])
            let message =
              switch result.error {
              | Some(error) => formatError(error)
              | None => "unknown error"
              }
            setStatus(_ => "Detection failed: " ++ message)
          }
          Js.Promise.resolve(())
        })
      let _ = Js.Promise2.catch(detectPromise, err => {
        let decoded = decodeError(err)->withFallbackCode("BW_POPUP_DETECT_UNEXPECTED")
        setIsLoading(_ => false)
        setStatus(_ => "Detection failed: " ++ formatError(decoded))
        Js.Promise.resolve(())
      })
      ()
    }

    let onFill = (fields: Js.Dict.t<string>): unit => {
      setIsLoading(_ => true)
      setStatus(_ => "Filling PDF and preparing download...")
      let fillPromise =
        Js.Promise2.then(fillPdfAndDownload(~blocks, ~fields), result => {
          setIsLoading(_ => false)
          switch result {
          | Ok(success) =>
            setStatus(_ =>
              "Filled PDF downloaded: "
              ++ Belt.Int.toString(success.bytes)
              ++ " bytes ("
              ++ success.filename
              ++ ")"
            )
          | Error(error) => setStatus(_ => "Fill failed: " ++ formatError(error))
          }
          Js.Promise.resolve(())
        })
      let _ = Js.Promise2.catch(fillPromise, err => {
        let decoded = decodeError(err)->withFallbackCode("BW_POPUP_FILL_UNEXPECTED")
        setIsLoading(_ => false)
        setStatus(_ => "Fill failed: " ++ formatError(decoded))
        Js.Promise.resolve(())
      })
      ()
    }

    React.useEffect0(() => {
      refresh()
      None
    })

    <div>
      <h1 style={ReactDOM.Style.make(~fontSize="16px", ~margin="0 0 8px 0", ())}>
        {React.string("Blocky Writer")}
      </h1>
      <p style={ReactDOM.Style.make(~margin="0 0 10px 0", ~fontSize="12px", ~color="#475467", ())}>
        {React.string(status)}
      </p>
      {
        if isLoading {
          <div style={ReactDOM.Style.make(~fontSize="12px", ~marginBottom="10px", ())}>
            {React.string("Loading...")}
          </div>
        } else {
          React.null
        }
      }
      <button
        onClick={_ => refresh()}
        style={
          ReactDOM.Style.make(
            ~width="100%",
            ~padding="8px",
            ~marginBottom="10px",
            ~borderRadius="6px",
            ~border="1px solid #344054",
            ~backgroundColor="#fff",
            ~cursor="pointer",
            (),
          )
        }>
        {React.string("Refresh Detection")}
      </button>
      <FormFiller blocks={blocks} onFill />
    </div>
  }
}

let mount = () => {
  switch Webapi.Dom.document->Webapi.Dom.Document.querySelector("#root") {
  | Some(root) => ReactDOM.Client.createRoot(root)->ReactDOM.Client.Root.render(<App />)
  | None => Js.log("popup root node missing")
  }
}

mount()
