/* SPDX-License-Identifier: AGPL-3.0-or-later WITH Palimpsest */
/* FFI boundary for DOM mounting operations (JS host calls only). */

let validateSelectorCode: (string, int) => int = %raw(`(selector, len) => {
  if (len === 0) return 1;
  if (len > 255) return 2;
  return /[^\w\-#.\[\]():>~+= ]/.test(selector) ? 3 : 0;
}`)

let validateHtmlCode: (string, int) => int = %raw(`(html, len) => {
  if (len > 1048576) return 1;
  if (len === 0) return 0;
  const openTags = (html.match(/<[^\/][^>]*>/g) || []).length;
  const closeTags = (html.match(/<\/[^>]+>/g) || []).length;
  const selfClosing = (html.match(/<[^>]+\/>/g) || []).length;
  return (openTags - selfClosing) === closeTags ? 0 : 2;
}`)

let findElement: string => option<Dom.element> = %raw(`(selector) => {
  const el = document.querySelector(selector);
  return el == null ? undefined : el;
}`)

let mountInnerHtml: (Dom.element, string) => int = %raw(`(element, html) => {
  if (element == null) return 1;
  try {
    element.innerHTML = html;
    return 0;
  } catch {
    return 2;
  }
}`)

let domReadyState: unit => string = %raw(`() => document.readyState`)
let onDOMContentLoaded: (unit => unit) => unit = %raw(`(callback) => document.addEventListener("DOMContentLoaded", callback)`)

let safeMountHTML = (selector: string, html: string): SafeDOMABI.mountResult => {
  let selectorLen = selector->Js.String2.length
  let selectorValidation =
    switch validateSelectorCode(selector, selectorLen) {
    | 0 => None
    | 1 => Some("Invalid selector: Selector cannot be empty")
    | 2 => Some("Invalid selector: Selector exceeds maximum length (255 characters)")
    | 3 => Some("Invalid selector: Selector contains invalid CSS characters")
    | _ => Some("Invalid selector: Unknown validation error")
    }

  switch selectorValidation {
  | Some(error) => SafeDOMABI.Failed(error)
  | None =>
    let htmlLen = html->Js.String2.length
    let htmlValidation =
      switch validateHtmlCode(html, htmlLen) {
      | 0 => None
      | 1 => Some("Invalid HTML: HTML content exceeds maximum size (1MB)")
      | 2 => Some("Invalid HTML: HTML tags are unbalanced")
      | _ => Some("Invalid HTML: Unknown validation error")
      }

    switch htmlValidation {
    | Some(error) => SafeDOMABI.Failed(error)
    | None =>
      switch findElement(selector) {
      | None => SafeDOMABI.NotFound(selector)
      | Some(element) =>
        switch mountInnerHtml(element, html) {
        | 0 => SafeDOMABI.MountedAt(element)
        | 1 => SafeDOMABI.Failed("Null element (impossible under validated lookup)")
        | 2 => SafeDOMABI.Failed("Mount operation failed")
        | _ => SafeDOMABI.Failed("Unknown mount error")
        }
      }
    }
  }
}
