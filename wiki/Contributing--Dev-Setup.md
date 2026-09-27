<!-- berrywiki
id: 0195b100-0000-7000-8000-000000000007
parent: 0195b100-0000-7000-8000-000000000006
position: 0
kind: page
tags:
  - contributing
  - setup
archived: false
-->

<!--
SPDX-License-Identifier: MPL-2.0
-->

# Dev Setup

## What you actually need

For the Rust core — the only thing with tests — you need:

* Rust stable with `rustfmt` and `clippy` components
* Network access to crates.io (or a vendored registry)

```bash
rustup toolchain install stable --component rustfmt clippy
```

That is it. There is no Node, Deno, npm or wasm-pack requirement for the core
checks, despite what older documents in this repo may say.

## What you do **not** need, and cannot currently use

* **Deno** — `deno.json` was removed in #43 (2026-08-24) along with every
  `deno task`. The `deno.lock` file still in the tree is a leftover.
* **wasm-pack** — only needed for `scripts/build-wasm.sh`, which produces a WASM
  package nothing in this repo currently consumes.
* **An AffineScript toolchain** — `src/*.affine` cannot be compiled here. There
  is no compiler configuration in this checkout.

If a document tells you to run `just check`, `mix compile`, or `guix develop`,
it is describing a different repository. See
[Contributing](Contributing) for what is real.

## First run

```bash
git clone https://github.com/hyperpolymath/blocky-writer.git
cd blocky-writer

cargo fmt    --manifest-path rust/pdftool_core/Cargo.toml -- --check
cargo test   --manifest-path rust/pdftool_core/Cargo.toml --locked
cargo clippy --manifest-path rust/pdftool_core/Cargo.toml --locked --all-targets -- -D warnings
```

`cargo test` runs 6 tests. If you get a different number, something changed and
the wiki is stale — fix the wiki.

## Building the WASM package (optional)

```bash
scripts/build-wasm.sh
```

Requires `wasm-pack` and the `wasm32-unknown-unknown` target. Nothing in CI does
this, and nothing consumes the output yet.

## Tooling files in the tree, and what they are for

| File | Status |
| --- | --- |
| `Justfile`, `contractile.just` | Task runner recipes. |
| `mise.toml`, `.tool-versions` | Tool version pinning. |
| `stapeln.toml` | Layer-based container definition. |
| `Containerfile` | Container build. |
| `selur-compose/compose.toml` | Compose definition. |
| `.editorconfig`, `.gitattributes` | Editor and git hygiene. |
| `setup.sh` | Bootstrap script. |

Several of these were minted from `rsr-template-repo` and describe a project
shape this repo does not have. Treat them as scaffolding, not as instructions.
