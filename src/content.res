/* SPDX-License-Identifier: AGPL-3.0-or-later WITH Palimpsest */

type runtime

@val @scope("browser")
external runtime: runtime = "runtime"

@send
external sendMessage: (runtime, Js.Json.t) => Js.Promise.t<Js.Json.t> = "sendMessage"

let ensureOverlayMountPoint: unit => unit = %raw(`() => {
  if (document.querySelector("#blocky-writer-overlay") !== null) return;
  const root = document.createElement("div");
  root.id = "blocky-writer-overlay";
  root.style.position = "fixed";
  root.style.right = "16px";
  root.style.bottom = "16px";
  root.style.zIndex = "2147483647";
  root.style.width = "320px";
  root.style.maxWidth = "90vw";
  root.style.fontFamily = "ui-sans-serif, system-ui, -apple-system, Segoe UI, sans-serif";
  document.body.appendChild(root);
}`)

let escapeHtml: string => string = %raw(`(value) =>
  value
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;")
`)

let renderPanel = (~title: string, ~detail: string): unit => {
  let safeTitle = escapeHtml(title)
  let safeDetail = escapeHtml(detail)
  let html =
    "<section style='background:#111827;color:#fff;border-radius:10px;padding:12px;box-shadow:0 10px 30px rgba(0,0,0,.3);'>"
    ++ "<div style='font-size:13px;font-weight:700;margin-bottom:6px;'>Blocky Writer</div>"
    ++ "<div style='font-size:12px;line-height:1.4;'><strong>"
    ++ safeTitle
    ++ "</strong><br />"
    ++ safeDetail
    ++ "</div>"
    ++ "</section>"

  SafeDOM.mountWhenReady(
    "#blocky-writer-overlay",
    html,
    ~onSuccess={_ => ()},
    ~onError={error => Js.log2("Blocky Writer overlay mount error", error)},
  )
}

let unsafeJson = (value: 'a): Js.Json.t => Obj.magic(value)

let makeDetectMessage = (url: string): Js.Json.t => {
  let payload = Js.Dict.empty()
  Js.Dict.set(payload, "action", Js.Json.string("detectBlocks"))
  Js.Dict.set(payload, "url", Js.Json.string(url))
  unsafeJson(payload)
}

let isPdfUrl = (url: string): bool => {
  let lower = url->Js.String2.toLowerCase
  lower->Js.String2.endsWith(".pdf")
  || lower->Js.String2.includes(".pdf?")
  || lower->Js.String2.includes(".pdf#")
}

let findPdfTarget = (): option<string> => {
  let currentUrl = Webapi.Dom.location->Webapi.Dom.Location.href
  if isPdfUrl(currentUrl) {
    Some(currentUrl)
  } else {
    switch Webapi.Dom.document->Webapi.Dom.Document.querySelector("a[href$='.pdf']") {
    | Some(linkElement) =>
      switch Webapi.Dom.Element.asHtmlElement(linkElement) {
      | Some(link) =>
        let linkHref = link->Webapi.Dom.HtmlElement.href
        if isPdfUrl(linkHref) {
          Some(linkHref)
        } else {
          None
        }
      | None => None
      }
    | None => None
    }
  }
}

let handleDetectionResponse = (response: Js.Json.t): unit => {
  switch Js.Json.decodeObject(response) {
  | Some(payload) =>
    let ok =
      payload->Js.Dict.get("ok")->Belt.Option.flatMap(Js.Json.decodeBoolean)->Belt.Option.getWithDefault(false)
    if ok {
      let count =
        payload
        ->Js.Dict.get("blocks")
        ->Belt.Option.flatMap(Js.Json.decodeArray)
        ->Belt.Option.map(Belt.Array.length)
        ->Belt.Option.getWithDefault(0)
      if count == 0 {
        renderPanel(~title="No widgets detected", ~detail="This PDF did not expose form widgets.")
      } else {
        renderPanel(
          ~title="Detection successful",
          ~detail="Detected " ++ Belt.Int.toString(count) ++ " PDF widgets ready for filling.",
        )
      }
    } else {
      let message =
        payload
        ->Js.Dict.get("error")
        ->Belt.Option.flatMap(Js.Json.decodeString)
        ->Belt.Option.getWithDefault("unknown error")
      renderPanel(~title="Detection failed", ~detail=message)
    }
  | None => renderPanel(~title="Detection failed", ~detail="Background returned an invalid payload.")
  }
}

let requestBlockDetection = (targetUrl: string): unit => {
  renderPanel(~title="Scanning PDF", ~detail="Requesting block detection...")
  let request = makeDetectMessage(targetUrl)
  let sendPromise = sendMessage(runtime, request)
  let loggedPromise = Js.Promise2.then(sendPromise, response => {
    handleDetectionResponse(response)
    Js.Promise.resolve(response)
  })
  let _ = Js.Promise2.catch(loggedPromise, err => {
    Js.log2("Blocky Writer message failed", err)
    renderPanel(~title="Detection failed", ~detail="Unable to contact extension background worker.")
    Js.Promise.resolve(Js.Json.null)
  })
  ()
}

let currentUrl = Webapi.Dom.location->Webapi.Dom.Location.href

if currentUrl->Js.String2.includes("gov.uk") {
  ensureOverlayMountPoint()
  switch findPdfTarget() {
  | Some(targetUrl) => requestBlockDetection(targetUrl)
  | None => renderPanel(~title="No PDF target found", ~detail="Open a .pdf URL or include a direct PDF link on this page.")
  }
}
