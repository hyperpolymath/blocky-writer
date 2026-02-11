/* SPDX-License-Identifier: AGPL-3.0-or-later WITH Palimpsest */
/* Adapted from /mnt/eclipse/repos/rescript-ecosystem/packages/web/dom-mounter/src/SafeDOM.res */

type mountResult =
  | Mounted(Dom.element)
  | MountPointNotFound(string)
  | InvalidSelector(string)
  | InvalidHTML(string)

module ProvenSelector = {
  type validated = ValidSelector(string)

  let hasInvalidSelectorChars: string => bool = %raw(`(selector) => /[^\w\-#.\[\]():>~+= ]/.test(selector)`)

  let validate = (selector: string): result<validated, string> => {
    let len = selector->Js.String2.length
    if len == 0 {
      Error("Selector cannot be empty")
    } else if len > 255 {
      Error("Selector exceeds maximum length (255 characters)")
    } else if hasInvalidSelectorChars(selector) {
      Error("Selector contains invalid CSS characters")
    } else {
      Ok(ValidSelector(selector))
    }
  }

  let toString = (ValidSelector(selector)) => selector
}

module ProvenHTML = {
  type validated = ValidHTML(string)

  let countOpenTags: string => int = %raw(`(html) => (html.match(/<[^\/][^>]*>/g) || []).length`)
  let countCloseTags: string => int = %raw(`(html) => (html.match(/<\/[^>]+>/g) || []).length`)
  let countSelfClosing: string => int = %raw(`(html) => (html.match(/<[^>]+\/>/g) || []).length`)

  let validate = (html: string): result<validated, string> => {
    let len = html->Js.String2.length
    if len == 0 {
      Ok(ValidHTML(""))
    } else if len > 1048576 {
      Error("HTML content exceeds maximum size (1MB)")
    } else {
      let openTags = countOpenTags(html)
      let closeTags = countCloseTags(html)
      let selfClosing = countSelfClosing(html)
      if openTags - selfClosing != closeTags {
        Error(
          "Unbalanced HTML tags: "
          ++ Belt.Int.toString(openTags - selfClosing)
          ++ " open, "
          ++ Belt.Int.toString(closeTags)
          ++ " close",
        )
      } else {
        Ok(ValidHTML(html))
      }
    }
  }

  let toString = (ValidHTML(html)) => html
}

let findMountPoint = (selector: ProvenSelector.validated): option<Dom.element> =>
  Webapi.Dom.document->Webapi.Dom.Document.querySelector(selector->ProvenSelector.toString)

let mount = (selector: ProvenSelector.validated, html: ProvenHTML.validated): mountResult => {
  switch findMountPoint(selector) {
  | None => MountPointNotFound(selector->ProvenSelector.toString)
  | Some(element) =>
    element->Webapi.Dom.Element.setInnerHTML(html->ProvenHTML.toString)
    Mounted(element)
  }
}

let mountString = (selector: string, html: string): mountResult => {
  switch ProvenSelector.validate(selector) {
  | Error(error) => InvalidSelector(error)
  | Ok(validSelector) =>
    switch ProvenHTML.validate(html) {
    | Error(error) => InvalidHTML(error)
    | Ok(validHtml) => mount(validSelector, validHtml)
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
  | MountPointNotFound(value) => onError("Mount point not found: " ++ value)
  | InvalidSelector(value) => onError("Invalid selector: " ++ value)
  | InvalidHTML(value) => onError("Invalid HTML: " ++ value)
  }
}

type mountSpec = {
  selector: string,
  html: string,
}

let mountBatch = (specs: array<mountSpec>): result<array<Dom.element>, string> => {
  let mounted = []
  let rec loop = (index: int): result<array<Dom.element>, string> => {
    if index >= Belt.Array.length(specs) {
      Ok(mounted)
    } else {
      switch specs[index] {
      | spec =>
        switch mountString(spec.selector, spec.html) {
        | Mounted(element) =>
          mounted->Belt.Array.push(element)
          loop(index + 1)
        | MountPointNotFound(value) => Error("Mount point not found: " ++ value)
        | InvalidSelector(value) => Error("Invalid selector: " ++ value)
        | InvalidHTML(value) => Error("Invalid HTML: " ++ value)
        }
      }
    }
  }
  loop(0)
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
