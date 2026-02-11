/* SPDX-License-Identifier: AGPL-3.0-or-later WITH Palimpsest */

module Template = {
  type t = {
    id: string,
    name: string,
    blocks: array<PdfTool.block>,
  }
}

let key = "blocky-writer:templates"

let serialize = (templates: array<Template.t>): string =>
  templates->Js.Json.stringifyAny->Belt.Option.getWithDefault("[]")

let parseTemplates = (raw: string): array<Template.t> => {
  switch Js.Json.parseExn(raw) {
  | json =>
    switch Js.Json.decodeArray(json) {
    | Some(_) => [||]
    | None => [||]
    }
  | exception _ => [||]
  }
}

let saveTemplates = (templates: array<Template.t>): unit => {
  let payload = serialize(templates)
  Webapi.Dom.window->Webapi.Dom.Window.localStorage->Webapi.Dom.Storage.setItem(key, payload)
}

let loadTemplates = (): array<Template.t> => {
  switch Webapi.Dom.window->Webapi.Dom.Window.localStorage->Webapi.Dom.Storage.getItem(key) {
  | Some(raw) => parseTemplates(raw)
  | None => [||]
  }
}
