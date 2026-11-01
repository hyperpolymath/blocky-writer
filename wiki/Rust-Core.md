<!-- berrywiki
id: 0195b100-0000-7000-8000-000000000003
parent: 0195b100-0000-7000-8000-000000000001
position: 1
kind: page
tags:
  - rust
  - core
archived: false
-->

<!--
SPDX-License-Identifier: MPL-2.0
-->

# Rust Core

`rust/pdftool_core` — the only implemented, tested part of this repository.

## Crate facts

| | |
| --- | --- |
| Edition | 2021 |
| Crate types | `cdylib`, `rlib` |
| Licence | MPL-2.0 |
| `#![forbid(unsafe_code)]` | yes |
| Lines (`src/lib.rs`) | See current source |
| Unit tests | 22 (6 fill/error, 14 detector, 2 numeric-geometry) |
| Current crate version | 0.2.0 |
| PR Rust CI | Run `41326285595` at `9f9370e` passed fmt, tests, and Clippy |

## Dependencies

| Crate | Version | Why |
| --- | --- | --- |
| `lopdf` | 0.34 | PDF object model, parsing, serialisation |
| `serde` | 1 (derive) | `Block` serialisation across the WASM boundary |
| `serde-wasm-bindgen` | 0.6 | serde ↔ `JsValue` |
| `wasm-bindgen` | 0.2 | export surface |
| `js-sys` | 0.3 | `Uint8Array` return type |

Nothing else. `Cargo.lock` is committed and CI runs with `--locked`.

## Public surface

Two `#[wasm_bindgen]` exports, plus the `Block` struct they share:

```rust
pub struct Block { pub label: String, pub x: f64, pub y: f64,
                   pub width: f64, pub height: f64 }
```

Everything else in the crate is private. `resolve_object`, `object_to_number`,
`rect_from_object`, `object_to_text`, `dict_text`, the field-descriptor walkers
and `apply_field_value` are internal machinery.

## Geometry and error contract

`/Rect` supplies two diagonal corners in PDF user space; PDF permits either
corner order. Detection maps them to `{x, y, width, height}` using the PDF-defined
interpretation and does not apply screen, rotation, or page-box transforms.
Chained indirect `/Rect` arrays and numeric elements are resolved with
reference-cycle protection. The output uses `f64` to preserve exactly
representable PDF integer coordinates through
binary64. Missing, unresolved, malformed, non-finite, or unrepresentable widget
geometry fails detection with `BW_PDF_WIDGET_RECT_INVALID`. Null or dangling
optional `/Annots` entries are treated as absent; malformed non-null entries
fail with structured codes. The PR's three new error codes and numeric surface
are included in crate version `0.2.0`.

## What `fill_blocks` does, in order

1. Reject empty input (`BW_PDF_EMPTY`).
2. `Document::load_mem` (`BW_PDF_INVALID`).
3. Resolve `trailer.Root` (`BW_PDF_ROOT_MISSING` / `BW_PDF_ROOT_INVALID`).
4. Resolve or materialise `catalog.AcroForm` (`BW_FORM_*`).
5. Set `NeedAppearances = true` so viewers regenerate appearances.
6. Walk `AcroForm.Fields` recursively, cycle-guarded by a `seen` set.
7. Build a `FieldDescriptor` per field: partial name, fully-qualified dotted
   name, inherited field type, and the widget ids underneath it.
8. Look each name up in the caller's map — fully-qualified first, then partial.
9. Dispatch on field type: `Tx`/`Ch` → text writeback, `Btn` → appearance-state
   writeback, anything else → `BW_FILL_UNSUPPORTED_FIELD_TYPE`.
10. If nothing matched and something was supplied → `BW_FILL_NO_MATCHING_FIELDS`.
11. `doc.save_to` (`BW_FILL_SAVE_FAILED`).

## Known limitations

* Only `Tx`, `Ch` and `Btn` field types are written. Signatures (`Sig`) are not.
* `detect_blocks` reports widget rectangles only. It does not detect ruled
  lines, per-character cells or baselines — the "hand spacing" case the project
  is named for is **not implemented yet**.
* Recursion depth is capped at 48 to bound hostile or cyclic field trees.
* `field_full_name` uses `.` as the separator, per the PDF convention.

## Running the checks

```bash
cargo fmt    --manifest-path rust/pdftool_core/Cargo.toml -- --check
cargo test   --manifest-path rust/pdftool_core/Cargo.toml --locked
cargo clippy --manifest-path rust/pdftool_core/Cargo.toml --locked --all-targets -- -D warnings
```

There is no Rust toolchain in most sandboxes, so if you cannot run these, say so
in the PR rather than implying you did. CI is the only authority on green.
