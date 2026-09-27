<!-- berrywiki
id: 0195b100-0000-7000-8000-000000000002
parent: 0195b100-0000-7000-8000-000000000001
position: 0
kind: page
tags:
  - architecture
archived: false
-->

<!--
SPDX-License-Identifier: MPL-2.0
-->

# Architecture

## Layers

```
┌──────────────────────────────────────────────┐
│ USER / BROWSER (Firefox, PDF forms)          │
└───────────────────────┬──────────────────────┘
                        ▼
┌──────────────────────────────────────────────┐
│ EXTENSION UI LAYER                            │
│   popup.affine  content.affine               │
│   components/Block.affine  components/FormFiller.affine
└───────────────────────┬──────────────────────┘
                        ▼
┌──────────────────────────────────────────────┐
│ BACKGROUND SERVICE                            │
│   background.affine  core/Storage.affine     │
│   core/ProvenMount.affine                    │
└───────────────────────┬──────────────────────┘
                        ▼
┌──────────────────────────────────────────────┐
│ CORE PROCESSING (Rust → WASM)                 │
│   rust/pdftool_core                           │
│   detect_blocks()   fill_blocks()            │
└───────────────────────┬──────────────────────┘
                        ▼
┌──────────────────────────────────────────────┐
│ DATA LAYER                                    │
│   IndexedDB / local storage                  │
└──────────────────────────────────────────────┘
```

## What actually exists

| Layer | Reality |
| --- | --- |
| Rust/WASM core | **Implemented and tested.** `rust/pdftool_core/src/lib.rs`, ~1030 lines, 6 unit tests. |
| AffineScript surfaces | **Prototype.** `src/*.affine` sources exist; no compiler config, no bundle pipeline. |
| Extension packaging | **Absent.** `public/manifest.json` and icons are present; nothing produces a loadable `.xpi`. |
| Storage | **Prototype.** `src/core/Storage.affine` only. |

## The seam that matters

Everything the extension does to a PDF goes through exactly two exported
functions. That boundary is the whole contract:

```rust
#[wasm_bindgen] pub fn detect_blocks(pdf_data: &[u8]) -> Result<JsValue, JsValue>
#[wasm_bindgen] pub fn fill_blocks(pdf_data: &[u8], blocks: JsValue, fields: JsValue)
                                     -> Result<js_sys::Uint8Array, JsValue>
```

* `detect_blocks` walks every page's `Annots`, keeps the ones that are widget
  annotations with a usable `Rect`, and returns a `Block { label, x, y, width,
  height }` per widget. Labels come from the field's `/T`, falling back to the
  parent field's `/T`, falling back to `field_<page>_<index>`.
* `fill_blocks` takes the original PDF bytes plus a `field name → value` map,
  writes the values into the AcroForm, and returns new PDF bytes. It is
  **name-driven, not coordinate-driven** — the `blocks` argument is parsed and
  validated but the writeback is keyed on field names.

That last point is worth internalising: blocky-writer's contribution to the
document suite is *placement into fixed-layout forms*, not free-form layout.
See [Ecosystem](Ecosystem).

## Error taxonomy

Failures never cross the WASM boundary as strings. They are structured payloads
with a stable machine code, a human message, and optional context:

```json
{ "code": "BW_FILL_NO_MATCHING_FIELDS", "message": "…", "context": "Choice" }
```

There are 20 `BW_*` codes. Codes are stable API — treat adding or renaming one
as a breaking change, not a tidy-up.
