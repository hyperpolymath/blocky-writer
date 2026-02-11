/* SPDX-License-Identifier: AGPL-3.0-or-later WITH Palimpsest */
/* High-level API over explicit ABI + FFI layers. */

type mountResult = SafeDOMABI.mountResult =
  | MountedAt(Dom.element)
  | NotFound(string)
  | Failed(string)

let mountString = (selector: string, html: string): mountResult => SafeDOMFFI.safeMountHTML(selector, html)

let mountSafe = (
  selector: string,
  html: string,
  ~onSuccess: Dom.element => unit,
  ~onError: string => unit,
): unit => {
  switch mountString(selector, html) {
  | MountedAt(element) => onSuccess(element)
  | NotFound(value) => onError("Mount point not found: " ++ value)
  | Failed(value) => onError(value)
  }
}

let onDOMReady = (callback: unit => unit): unit => {
  let state = SafeDOMFFI.domReadyState()
  if state == "complete" || state == "interactive" {
    callback()
  } else {
    SafeDOMFFI.onDOMContentLoaded(callback)
  }
}

let mountWhenReady = (
  selector: string,
  html: string,
  ~onSuccess: Dom.element => unit,
  ~onError: string => unit,
): unit => onDOMReady(() => mountSafe(selector, html, ~onSuccess, ~onError))
