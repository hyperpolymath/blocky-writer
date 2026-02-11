/* SPDX-License-Identifier: AGPL-3.0-or-later WITH Palimpsest */
/* ABI validation is consumed directly from generated artifact:
   /mnt/eclipse/repos/rescript-ecosystem/packages/web/dom-mounter/src/SafeDOM.res.js
   FFI stays local. */

type mountResult =
  | Mounted(Dom.element)
  | NotFound(string)
  | Failed(string)

type validatedSelector
type validatedHtml

@module("../../../rescript-ecosystem/packages/web/dom-mounter/src/SafeDOM.res.js")
@scope("ProvenSelector")
external validateSelector: string => result<validatedSelector, string> = "validate"

@module("../../../rescript-ecosystem/packages/web/dom-mounter/src/SafeDOM.res.js")
@scope("ProvenSelector")
external selectorToString: validatedSelector => string = "toString"

@module("../../../rescript-ecosystem/packages/web/dom-mounter/src/SafeDOM.res.js")
@scope("ProvenHTML")
external validateHtml: string => result<validatedHtml, string> = "validate"

@module("../../../rescript-ecosystem/packages/web/dom-mounter/src/SafeDOM.res.js")
@scope("ProvenHTML")
external htmlToString: validatedHtml => string = "toString"

let findElement = (selector: string): option<Dom.element> =>
  Webapi.Dom.document->Webapi.Dom.Document.querySelector(selector)

let mountInnerHtml = (element: Dom.element, html: string): mountResult => {
  try {
    element->Webapi.Dom.Element.setInnerHTML(html)
    Mounted(element)
  } catch {
  | _ => Failed("Mount operation failed")
  }
}

let mountValidated = (
  selector: validatedSelector,
  html: validatedHtml,
): mountResult => {
  let selectorValue = selectorToString(selector)
  let htmlValue = htmlToString(html)
  switch findElement(selectorValue) {
  | Some(element) => mountInnerHtml(element, htmlValue)
  | None => NotFound(selectorValue)
  }
}

let mountString = (selector: string, html: string): mountResult => {
  switch validateSelector(selector) {
  | Error(error) => Failed("Invalid selector: " ++ error)
  | Ok(validSelector) =>
    switch validateHtml(html) {
    | Error(error) => Failed("Invalid HTML: " ++ error)
    | Ok(validHtml) => mountValidated(validSelector, validHtml)
    }
  }
}

let mountSafe = (
  selector: string,
  html: string,
  ~onSuccess: Dom.element => unit,
  ~onError: string => unit,
): unit => {
  switch mountString(selector, html) {
  | Mounted(element) => onSuccess(element)
  | NotFound(value) => onError("Mount point not found: " ++ value)
  | Failed(value) => onError(value)
  }
}

let domReadyState: unit => string = %raw(`() => document.readyState`)
let onDOMContentLoaded: (unit => unit) => unit = %raw(`(callback) => document.addEventListener("DOMContentLoaded", callback)`)

let onDOMReady = (callback: unit => unit): unit => {
  let state = domReadyState()
  if state == "complete" || state == "interactive" {
    callback()
  } else {
    onDOMContentLoaded(callback)
  }
}

let mountWhenReady = (
  selector: string,
  html: string,
  ~onSuccess: Dom.element => unit,
  ~onError: string => unit,
): unit => onDOMReady(() => mountSafe(selector, html, ~onSuccess, ~onError))
