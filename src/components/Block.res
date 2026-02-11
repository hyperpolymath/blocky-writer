/* SPDX-License-Identifier: AGPL-3.0-or-later WITH Palimpsest */

@react.component
let make = (~label: string, ~value: string, ~onChange: string => unit) => {
  <div style={ReactDOM.Style.make(~marginBottom="8px", ())}>
    <label style={ReactDOM.Style.make(~display="block", ~fontWeight="600", ~marginBottom="4px", ())}>
      {React.string(label)}
    </label>
    <input
      type_="text"
      value={value}
      onChange={ev => onChange(Js.Dict.unsafeGet(Obj.magic(ReactEvent.Form.target(ev)), "value"))}
      style={
        ReactDOM.Style.make(
          ~width="100%",
          ~boxSizing="border-box",
          ~border="1px solid #d0d5dd",
          ~borderRadius="6px",
          ~padding="8px",
          (),
        )
      }
    />
  </div>
}
