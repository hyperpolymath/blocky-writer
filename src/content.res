/* SPDX-License-Identifier: AGPL-3.0-or-later WITH Palimpsest */

type runtime

@val @scope("browser")
external runtime: runtime = "runtime"

@send
external sendMessage: (runtime, Js.Json.t) => Js.Promise.t<Js.Json.t> = "sendMessage"

let unsafeJson = (value: 'a): Js.Json.t => Obj.magic(value)

let makeDetectMessage = (url: string): Js.Json.t => {
  let payload = Js.Dict.empty()
  Js.Dict.set(payload, "action", Js.Json.string("detectBlocks"))
  Js.Dict.set(payload, "url", Js.Json.string(url))
  unsafeJson(payload)
}

let isPdfUrl = (url: string): bool => url->Js.String2.toLowerCase->Js.String2.endsWith(".pdf")

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

let logDetectionResponse = (response: Js.Json.t): unit => {
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
      Js.log2("Blocky Writer detected blocks", count)
    } else {
      let message =
        payload
        ->Js.Dict.get("error")
        ->Belt.Option.flatMap(Js.Json.decodeString)
        ->Belt.Option.getWithDefault("unknown error")
      Js.log2("Blocky Writer detection failed", message)
    }
  | None => Js.log("Blocky Writer received non-object response")
  }
}

let requestBlockDetection = (targetUrl: string): unit => {
  Js.log2("Blocky Writer detectBlocks target", targetUrl)
  let request = makeDetectMessage(targetUrl)
  let sendPromise = sendMessage(runtime, request)
  let loggedPromise = Js.Promise2.then(sendPromise, response => {
    logDetectionResponse(response)
    Js.Promise.resolve(response)
  })
  let _ = Js.Promise2.catch(loggedPromise, err => {
    Js.log2("Blocky Writer message failed", err)
    Js.Promise.resolve(Js.Json.null)
  })
  ()
}

let currentUrl = Webapi.Dom.location->Webapi.Dom.Location.href

if currentUrl->Js.String2.includes("gov.uk") {
  switch findPdfTarget() {
  | Some(targetUrl) => requestBlockDetection(targetUrl)
  | None => Js.log("Blocky Writer found no PDF target on page")
  }
}
