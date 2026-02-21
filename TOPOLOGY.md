<!-- SPDX-License-Identifier: PMPL-1.0-or-later -->
<!-- TOPOLOGY.md — Project architecture map and completion dashboard -->
<!-- Last updated: 2026-02-19 -->

# blocky-writer — Project Topology

## System Architecture

```
                        ┌─────────────────────────────────────────┐
                        │              USER / BROWSER             │
                        │        (Firefox / PDF Forms)            │
                        └───────────────────┬─────────────────────┘
                                            │
                                            ▼
                        ┌─────────────────────────────────────────┐
                        │           EXTENSION UI LAYER            │
                        │    (ReScript + React + Office.js)       │
                        │  ┌───────────┐  ┌───────────────────┐  │
                        │  │ Popup UI  │  │  Content Script   │  │
                        │  └─────┬─────┘  └────────┬──────────┘  │
                        └────────│─────────────────│──────────────┘
                                 │                 │
                                 ▼                 ▼
                        ┌─────────────────────────────────────────┐
                        │           BACKGROUND SERVICE            │
                        │      (ReScript, State Management)       │
                        └───────────────────┬─────────────────────┘
                                            │
                                            ▼
                        ┌─────────────────────────────────────────┐
                        │           CORE PROCESSING (WASM)        │
                        │    (Rust pdftool_core, AcroForms)       │
                        └───────────────────┬─────────────────────┘
                                            │
                                            ▼
                        ┌─────────────────────────────────────────┐
                        │             DATA LAYER                  │
                        │      (IndexedDB, Local Storage)         │
                        └─────────────────────────────────────────┘

                        ┌─────────────────────────────────────────┐
                        │          REPO INFRASTRUCTURE            │
                        │  Deno-first scripts  .machine_readable/ │
                        │  webpack.config.cjs  Containerfile      │
                        └─────────────────────────────────────────┘
```

## Completion Dashboard

```
COMPONENT                          STATUS              NOTES
─────────────────────────────────  ──────────────────  ─────────────────────────────────
EXTENSION LAYERS
  Popup UI (ReScript/React)         ██████████ 100%    Stateful forms stable
  Background Script                 ██████████ 100%    WASM bridge active
  Content Script                    ████████░░  80%    PDF block detection refining

CORE (RUST/WASM)
  pdftool_core (WASM)               ██████████ 100%    Core logic stable
  AcroForm Writeback                ██████████ 100%    Text/Select widgets verified
  Error Taxonomy                    ██████████ 100%    Structured error codes active

REPO INFRASTRUCTURE
  Deno Build Tasks                  ██████████ 100%    deno task build verified
  .machine_readable/                ██████████ 100%    STATE.a2ml tracking
  Containerfile                     ██████████ 100%    Reproducible dev env

─────────────────────────────────────────────────────────────────────────────
OVERALL:                            █████████░  ~90%   Production-ready extension
```

## Key Dependencies

```
Block Detection ───► AcroForm Parser ───► WASM Bridge ───► Extension UI
      (Rust)              (Rust)             (JS)           (ReScript)
```

## Update Protocol

This file is maintained by both humans and AI agents. When updating:

1. **After completing a component**: Change its bar and percentage
2. **After adding a component**: Add a new row in the appropriate section
3. **After architectural changes**: Update the ASCII diagram
4. **Date**: Update the `Last updated` comment at the top of this file

Progress bars use: `█` (filled) and `░` (empty), 10 characters wide.
Percentages: 0%, 10%, 20%, ... 100% (in 10% increments).
