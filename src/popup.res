/* SPDX-License-Identifier: AGPL-3.0-or-later WITH Palimpsest */

let demoBlocks: array<PdfTool.block> = [
  {
    label: "Full Name",
    x: 100.,
    y: 140.,
    width: 160.,
    height: 20.,
  },
  {
    label: "Date of Birth",
    x: 100.,
    y: 170.,
    width: 160.,
    height: 20.,
  },
]

let mount = () => {
  switch Webapi.Dom.document->Webapi.Dom.Document.querySelector("#root") {
  | Some(root) =>
    ReactDOM.Client.createRoot(root)->ReactDOM.Client.Root.render(
      <FormFiller
        blocks={demoBlocks}
        onFill={fields => {
          Js.log2("Fill requested", fields)
        }}
      />,
    )
  | None => Js.log("popup root node missing")
  }
}

mount()
