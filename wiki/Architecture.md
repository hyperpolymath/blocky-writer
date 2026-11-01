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
| Rust/WASM core | **Implemented and tested in CI.** `rust/pdftool_core/src/lib.rs` contains 22 unit-test functions (6 fill/error, 14 detector, 2 numeric-geometry); PR #78 revision `9f9370e` passed the Rust CI job in run `41326285595`. |
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

* `detect_blocks` walks every page's `Annots` and returns a `Block { label, x,
  y, width, height }` per widget with usable geometry. `/Rect` accepts its two
  diagonal corners in either PDF-permitted order and resolves chained indirect
  arrays and numeric coordinate objects with cycle protection; output
  coordinates are PDF user-space `f64`
  values, with no screen or page-rotation transform. Missing, unresolved,
  malformed, non-finite, or unrepresentable widget rectangles fail detection
  with `BW_PDF_WIDGET_RECT_INVALID` rather than being silently skipped. A null
  or dangling optional `/Annots` entry is treated as absent; other malformed
  non-null annotation objects fail with a structured error. Labels come from
  the field's `/T`, falling back to the parent field's `/T`, falling back to
  `field_<page>_<index>`.
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

There are 23 unique `BW_*` codes in the current source (20 baseline codes plus
three detector-validation codes). Error codes are external API; repository
guidance treats additions and renames as breaking changes. The core is bumped
to `0.2.0` for this contract change; owner compatibility review is still open.
Rust CI run `41326285595` passed formatting, all 22 tests, and Clippy at
implementation commit `9f9370e`. That run does not establish general PDF
conformance or a formal proof.
