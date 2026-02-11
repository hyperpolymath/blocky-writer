/* SPDX-License-Identifier: AGPL-3.0-or-later WITH Palimpsest */

let href = Webapi.Dom.window->Webapi.Dom.Window.location##href

if href->Js.String2.includes("gov.uk") {
  Js.log2("Blocky Writer active on", href)
}
