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

1. Install dependencies (Deno-first):

```bash
deno install
```

2. Build ReScript output and bundle extension assets:

```bash
deno task build
```

3. Build the Rust WASM package:

```bash
deno task build:wasm
```

4. Run the extension in Firefox via `web-ext`:

```bash
deno task dev
```

5. Run core fill fixture tests (error taxonomy + AcroForm writeback):

```bash
deno task test:core-fill
```

## Notes

- Rust `fill_blocks` now performs AcroForm-aware writeback for text/select and button widgets, and emits structured taxonomy errors (`code`, `message`, `context`).
- Popup/background/content surfaces preserve taxonomy codes from the Rust WASM boundary.
- Source files include SPDX headers targeting AGPL + Palimpsest exception.

## Firefox troubleshooting

- If `deno task dev` fails with `ECONNREFUSED 127.0.0.1:<port>`, verify Firefox is installed and runnable.
- Close stale Firefox instances launched by prior `web-ext` sessions, then retry `deno task dev`.
- You can run `web-ext` directly with an explicit binary when needed:

```bash
deno run -A npm:web-ext run --source-dir dist --firefox /usr/bin/firefox
```
