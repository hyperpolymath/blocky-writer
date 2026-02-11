/* SPDX-License-Identifier: AGPL-3.0-or-later WITH Palimpsest */

type runtime
type tabsApi
type tab = Js.Json.t

@val @scope("browser")
external runtime: runtime = "runtime"

@val @scope("browser")
external tabsApi: tabsApi = "tabs"

@send
external sendMessage: (runtime, Js.Json.t) => Js.Promise.t<Js.Json.t> = "sendMessage"

@send
external queryTabs: (tabsApi, Js.Json.t) => Js.Promise.t<array<tab>> = "query"

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
  let tabsPromise = queryTabs(tabsApi, makeTabQuery())
  let detectPromise =
    Js.Promise2.then(tabsPromise, entries => {
      if Belt.Array.length(entries) == 0 {
        Js.Promise.resolve({ok: false, blocks: [], error: Some("no active tab found")})
      } else {
        switch tabUrl(entries[0]) {
        | None => Js.Promise.resolve({ok: false, blocks: [], error: Some("active tab URL unavailable")})
        | Some(url) =>
          if !looksLikePdfUrl(url) {
            Js.Promise.resolve({ok: false, blocks: [], error: Some("active tab is not a PDF URL")})
          } else {
            Js.Promise2.then(sendMessage(runtime, makeDetectMessage(url)), response =>
              Js.Promise.resolve(decodeDetectResult(response))
            )
          }
        }
      }
    })

  Js.Promise2.catch(detectPromise, err => {
    Js.log2("popup detect failed", err)
    Js.Promise.resolve({ok: false, blocks: [], error: Some("request failed")})
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
      <FormFiller
        blocks={blocks}
        onFill={fields => {
          Js.log2("Fill requested", fields)
        }}
      />
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
