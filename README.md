# blocky-writer

`blocky-writer` is a Mozilla Firefox extension for block-based PDF form filling.

## Stack

- Extension UI: ReScript + React
- Background/content scripts: ReScript
- Core processing: Rust compiled to WebAssembly (WASM)
- Local storage: IndexedDB

## Repo layout

- `src/`: ReScript sources for popup UI, background script, content script, and core modules
- `public/`: static extension files (`manifest.json`, popup HTML, icons)
- `rust/pdftool_core/`: Rust WASM crate for block detection and PDF operations
- `scripts/`: build helpers for WASM and extension bundles

## Quick start

1. Install Node dependencies:

```bash
npm install
```

2. Build ReScript output and bundle extension assets:

```bash
npm run build
```

3. Build the Rust WASM package:

```bash
npm run build:wasm
```

4. Run the extension in Firefox via `web-ext`:

```bash
npm run dev
```

## Notes

- The initial Rust implementation is a compile-safe skeleton that returns deterministic placeholder blocks.
- OCR and robust PDF writeback are intentionally staged for follow-up milestones.
- Source files include SPDX headers targeting AGPL + Palimpsest exception.
