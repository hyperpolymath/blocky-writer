<!-- berrywiki
id: 0195b100-0000-7000-8000-000000000001
parent: null
position: 0
kind: page
tags:
  - index
archived: false
-->

<!--
SPDX-License-Identifier: MPL-2.0
-->

# Home

**blocky-writer** is a Mozilla Firefox extension for filling **fixed-layout PDF
and application forms** — the kind laid out for boxes, baselines and
per-character cells that were designed for hand spacing rather than reliable
computer entry.

This wiki is the long-form companion to the repository. It is maintained in
BerryWiki format and mirrored to the GitHub wiki; the source of truth lives in
[`wiki/`](https://github.com/hyperpolymath/blocky-writer/tree/main/wiki) so it
is versioned, diffable and reviewable like any other file. See
[Contributing](Contributing) for how to edit it.

## Read this first

| If you want to… | Go to |
| --- | --- |
| Understand the shape of the thing | [Architecture](Architecture) |
| Work on the Rust/WASM core | [Rust-Core](Rust-Core) |
| Understand why CI is red or green | [CI-and-Gates](CI-and-Gates) |
| Know what blocky-writer is *not*, and who does that instead | [Ecosystem](Ecosystem) |
| Get a working dev environment | [Contributing--Dev-Setup](Contributing--Dev-Setup) |
| Look up a term | [Glossary](Glossary) |

## Current status in one paragraph

The Rust core (`rust/pdftool_core`) is implemented, unit-tested and green on
`cargo fmt`, `cargo test --locked` and `cargo clippy -D warnings`. The
browser-extension frontend is a **prototype**: this checkout has no
AffineScript compiler configuration, no JavaScript package manifest and no
reproducible extension bundle pipeline, so there is nothing to build or release
yet. Do not read the completion percentages in `TOPOLOGY.adoc` as a statement
about the frontend — see [CI-and-Gates](CI-and-Gates) for the current honest
picture.

## Quick commands

```bash
cargo fmt    --manifest-path rust/pdftool_core/Cargo.toml -- --check
cargo test   --manifest-path rust/pdftool_core/Cargo.toml --locked
cargo clippy --manifest-path rust/pdftool_core/Cargo.toml --locked --all-targets -- -D warnings
```

Those three lines are exactly what CI runs. If they pass locally, the
`Rust core (tests, formatting, lint)` check passes.
