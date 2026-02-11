/* SPDX-License-Identifier: AGPL-3.0-or-later WITH Palimpsest */

module Template = {
  type t = {
    id: string,
    name: string,
    blocks: array<PdfTool.block>,
  }
}

let key = "blocky-writer:templates"

@val
external stringify: 'a => string = "JSON.stringify"

@val
external parse: string => 'a = "JSON.parse"

@val
external setItem: (string, string) => unit = "localStorage.setItem"

@val
external getItem: string => Js.Nullable.t<string> = "localStorage.getItem"

let serialize = (templates: array<Template.t>): string => stringify(templates)

let parseTemplates = (raw: string): array<Template.t> => {
  switch parse(raw) {
  | templates => templates
  | exception _ => []
  }
}

let saveTemplates = (templates: array<Template.t>): unit => {
  let payload = serialize(templates)
  setItem(key, payload)
}

let loadTemplates = (): array<Template.t> => {
  switch getItem(key)->Js.Nullable.toOption {
  | Some(raw) => parseTemplates(raw)
  | None => []
  }
}
