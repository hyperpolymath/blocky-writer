/* SPDX-License-Identifier: AGPL-3.0-or-later WITH Palimpsest */

type runtime
type tabsApi
type tab = Js.Json.t
type urlLookup = result<string, string>
type fillSuccess = {
  bytes: int,
  filename: string,
}
type fillResult = result<fillSuccess, string>

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
let errorDetails: 'a => string = %raw(`(error) => {
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
        Js.Promise.resolve(Error("no active tab found"))
      } else {
        switch tabUrl(entries[0]) {
        | None => Js.Promise.resolve(Error("active tab URL unavailable"))
        | Some(url) =>
          if looksLikePdfUrl(url) {
            Js.Promise.resolve(Ok(url))
          } else {
            Js.Promise.resolve(Error("active tab is not a PDF URL"))
          }
        }
      }
    })

  Js.Promise2.catch(lookupPromise, err => {
    Js.log2("active tab lookup failed", err)
    Js.Promise.resolve(Error("failed to query active tab: " ++ errorDetails(err)))
  })
}

type detectResult = {
  ok: bool,
  blocks: array<PdfTool.block>,
  error: option<string>,
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
    let error = obj->Js.Dict.get("error")->Belt.Option.flatMap(Js.Json.decodeString)
    {ok, blocks, error}
  | None => {ok: false, blocks: [], error: Some("invalid background response")}
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
    Js.log2("popup detect failed", err)
    Js.Promise.resolve({ok: false, blocks: [], error: Some("request failed: " ++ errorDetails(err))})
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
                Js.Promise.resolve(Error("fill_blocks returned an empty PDF payload"))
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
    Js.log2("fill and download failed", err)
    Js.Promise.resolve(Error("fill and download request failed: " ++ errorDetails(err)))
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
            let message = result.error->Belt.Option.getWithDefault("unknown error")
            setStatus(_ => "Detection failed: " ++ message)
          }
          Js.Promise.resolve(())
        })
      let _ = Js.Promise2.catch(detectPromise, _ => {
        setIsLoading(_ => false)
        setStatus(_ => "Detection failed: unexpected error")
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
          | Error(message) => setStatus(_ => "Fill failed: " ++ message)
          }
          Js.Promise.resolve(())
        })
      let _ = Js.Promise2.catch(fillPromise, err => {
        setIsLoading(_ => false)
        setStatus(_ => "Fill failed: unexpected error: " ++ errorDetails(err))
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
