/* SPDX-License-Identifier: AGPL-3.0-or-later WITH Palimpsest */

@react.component
let make = (~blocks: array<PdfTool.block>, ~onFill: Js.Dict.t<string> => unit) => {
  let (fields, setFields) = React.useState(() => Js.Dict.empty())

  let handleChange = (label: string, value: string) => {
    setFields(prev => {
      let next = Js.Dict.copy(prev)
      Js.Dict.set(next, label, value)
      next
    })
  }

  <div>
    <h1 style={ReactDOM.Style.make(~fontSize="16px", ~margin="0 0 12px 0", ())}>
      {React.string("Block-Based Form Filler")}
    </h1>
    {
      blocks
      ->Belt.Array.map(block => {
        let current = Js.Dict.get(fields, block.label)->Belt.Option.getWithDefault("")
        <Block key={block.label} label={block.label} value={current} onChange={v => handleChange(block.label, v)} />
      })
      ->React.array
    }
    <button
      onClick={_ => onFill(fields)}
      style={
        ReactDOM.Style.make(
          ~width="100%",
          ~padding="10px",
          ~borderRadius="6px",
          ~border="1px solid #111827",
          ~backgroundColor="#111827",
          ~color="#fff",
          ~cursor="pointer",
          (),
        )
      }>
      {React.string("Fill Form")}
    </button>
  </div>
}
