# pdftool_core

Rust/WASM core for `blocky-writer`.

## Current scope

- Exposes `detect_blocks(pdf_data)` for block metadata extraction (placeholder output for now).
- Exposes `fill_blocks(pdf_data, blocks, fields)` for writeback (currently passthrough).

## Build

```bash
wasm-pack build --target web --out-dir pkg
```

## Next implementation milestones

1. Parse PDF page geometry and form widgets.
2. Integrate OCR-backed fallback for scanned documents.
3. Write field values back into PDF objects with deterministic positioning.
