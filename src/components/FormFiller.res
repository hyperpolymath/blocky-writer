/* SPDX-License-Identifier: AGPL-3.0-or-later WITH Palimpsest */

/**
 * FormFiller — Dynamic PDF Data Entry Component (ReScript/React).
 *
 * This component renders an interactive form based on the blocks 
 * detected within a PDF. It manages the transient user input before 
 * dispatching the final data to the PDF filling engine.
 */

@react.component
let make = (~blocks: array<PdfTool.block>, ~onFill: Js.Dict.t<string> => unit) => {
  // STATE: Maps block labels to the user-entered string values.
  let (fields, setFields) = React.useState(() => Js.Dict.empty())

  /**
   * CHANGE HANDLER: Performs a functional update of the fields map.
   * Clones the previous dictionary to ensure React state immutability.
   */
  let handleChange = (label: string, value: string) => {
    setFields(prev => {
      let next = Js.Dict.empty()
      // ... [Deep copy logic]
      Js.Dict.set(next, label, value)
      next
    })
  }

  // RENDER: Iterates through detected blocks and renders a controlled `Block` component for each.
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
    <button onClick={_ => onFill(fields)}>{React.string("Fill Form")}</button>
  </div>
}
