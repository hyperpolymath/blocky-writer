<!--
SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
SPDX-License-Identifier: MPL-2.0
-->

# Contributing to blocky-writer

**Read `README.adoc` first.** The short version of where this project stands:
the Rust/WASM core (`rust/pdftool_core`) is implemented, unit-tested and green
in CI. The AffineScript frontend (`src/*.affine`) is a prototype with no build
pipeline in this checkout. Nothing here is release-ready as a Firefox extension
yet.

---

## Clone and set up

```bash
git clone https://github.com/hyperpolymath/blocky-writer.git
cd blocky-writer
```

You need **Rust stable** with the `rustfmt` and `clippy` components. Nothing else
is required for the only checks that exist:

```bash
rustup toolchain install stable --component rustfmt clippy
```

There is **no** Node, Deno, npm, `wasm-pack` or Guix requirement. Older documents
in this repository describe a different project shape — if a file tells you to
run `deno task`, `just check`, or `guix develop`, it is stale. Please report it
with the *Documentation* issue template.

### Optional tooling in the tree

| File | What it is |
| --- | --- |
| `Justfile`, `contractile.just` | `just` recipes: `just doctor`, `just tour`, `just help-me`, `just aspect`, `just crg-grade`. Run `just --list` for the current set. |
| `mise.toml`, `.tool-versions` | Tool version pinning. |
| `Containerfile`, `stapeln.toml`, `selur-compose/compose.toml` | Container and compose definitions. |
| `setup.sh` | Bootstrap script. |
| `scripts/build-wasm.sh` | Builds the WASM package. Needs `wasm-pack` + `wasm32-unknown-unknown`. Nothing in CI runs it and nothing consumes the output yet. |

Several of these were minted from `hyperpolymath/rsr-template-repo` and describe
a project shape this repository does not have. Treat them as scaffolding.

---

## Repository structure

```text
blocky-writer/
├── rust/pdftool_core/        # Rust → WASM core (the only tested code)
│   ├── src/lib.rs            #   detect_blocks, fill_blocks, BW_* taxonomy
│   ├── Cargo.toml
│   └── Cargo.lock            # committed; CI runs with --locked
├── src/                      # AffineScript prototype (not buildable here)
│   ├── popup.affine, content.affine, background.affine
│   ├── components/           #   Block.affine, FormFiller.affine
│   └── core/                 #   PdfTool, ProvenMount, Storage
├── public/                   # manifest.json, popup.html, icons
├── tests/                    # aspect + fuzz placeholder
├── scripts/                  # build-wasm.sh, check-lock-sync.sh
├── wiki/                     # BerryWiki-format wiki source (see wiki/README.adoc)
├── docs/
│   ├── ci/CHECK-DETERMINATIONS.adoc   # the CI ledger — read the standing rules
│   ├── ecosystem/ECOSYSTEM.adoc       # suite boundary and neighbours
│   └── reports/, tech-debt-*.adoc
├── .machine_readable/        # machine-readable state; load-bearing, do not restructure
├── www/.well-known/          # ai.txt, humans.txt, security.txt
├── README.adoc               # the README (AsciiDoc, not Markdown)
├── TOPOLOGY.adoc             # architecture map + completion dashboard
├── EXPLAINME.adoc
├── TEST-NEEDS.adoc           # CRG test grade
├── CHANGELOG.adoc
├── CODE_OF_CONDUCT.adoc
├── SECURITY.adoc
├── GOVERNANCE.adoc
├── Containerfile
├── Justfile
└── .github/
    ├── ISSUE_TEMPLATE/       # bug_report, feature_request, documentation, config
    ├── PULL_REQUEST_TEMPLATE.md
    ├── workflows/
    └── actions.lock
```

Note the `.adoc` extensions. This repository documents itself in AsciiDoc, not
Markdown.

---

## How to contribute

### Reporting bugs

Use the [bug report template](.github/ISSUE_TEMPLATE/bug_report.yml). Include:

* Which area — Rust core, frontend prototype, CI, docs or packaging.
* Steps to reproduce. For core bugs, a minimal PDF and the field map you passed.
* The **whole** error payload if you got one. Core failures carry a stable
  machine code plus message and context; the code alone is not enough.

Before reporting, search existing issues and check whether the bug is in the
core (actionable) or the frontend prototype (may not be).

### Suggesting features

Use the [feature request template](.github/ISSUE_TEMPLATE/feature_request.yml).
Check `docs/ecosystem/ECOSYSTEM.adoc` first — blocky-writer owns fixed-layout PDF
and application-form placement *only*. Viewing, editing, conversion, OCR and
print routing belong to other projects in the suite.

### Reporting documentation problems

Use the [documentation template](.github/ISSUE_TEMPLATE/documentation.yml).
Documentation drift is a defect here, not a nit.

### Security

Do **not** open a public issue. Use
[GitHub Security Advisories](https://github.com/hyperpolymath/blocky-writer/security/advisories/new).
See `SECURITY.adoc`.

### Opening a pull request

1. Fork and branch from `main`.
2. Make your change.
3. Run the three core checks — **all three must pass**:

   ```bash
   cargo fmt    --manifest-path rust/pdftool_core/Cargo.toml -- --check
   cargo test   --manifest-path rust/pdftool_core/Cargo.toml --locked
   cargo clippy --manifest-path rust/pdftool_core/Cargo.toml --locked --all-targets -- -D warnings
   ```

   `cargo test` runs 6 tests. If you get a different number, something changed
   and the wiki is stale — fix the wiki too.

   If you cannot run these — no Rust toolchain, no crates.io access — **say so
   explicitly in the PR** rather than implying you did. CI is the only authority
   on green.

4. **If you changed a `uses:` line in any workflow**, regenerate
   `.github/workflows/actions.lock` in the *same commit* and run
   `scripts/check-lock-sync.sh` (needs `gawk`). A stale lock entry is as fatal as
   a missing one, and the failure is silent — `startup_failure`, zero jobs, no
   log.
5. **If you removed or retired a CI check**, add or update a row in
   `docs/ci/CHECK-DETERMINATIONS.adoc`.
6. Fill in `.github/PULL_REQUEST_TEMPLATE.md`. Do not delete the checklist.

### House rules

* AsciiDoc (`.adoc`) for repository documentation. SPDX header on every file.
* MPL-2.0 licence, Palimpsest philosophy.
* Conventional commits: `fix(rust): …`, `docs(ci): …`, `ci: …`.
* No new TypeScript, Python or Go. No npm/bun/yarn/pnpm dependencies.
* No `unsafe` blocks in Rust without a safety comment. The core is
  `#![forbid(unsafe_code)]` — keep it that way.
* `.machine_readable/` is load-bearing. Do not restructure it casually.
* `BW_*` error codes are stable API. Adding or renaming one is a breaking change.
* **Never** silence a gate. No `continue-on-error`, no `if: false`, no quiet
  deletion of a job.

### CI you cannot trigger

Every Actions run whose actor is `arena-ai-coding-agent[bot]` is refused at
startup (`Actor is not allowed to trigger Actions workflows`). A bot-authored PR
shows **no** repository checks at all — not red, absent — because a startup
failure creates no check run. This is an org-side policy; no file change cures
it. Bot PRs must be merged by the repository owner so the merge push carries an
allowed actor. See the `<<actor>>` section of `docs/ci/CHECK-DETERMINATIONS.adoc`.

---

## The wiki

The project wiki lives at
<https://github.com/hyperpolymath/blocky-writer/wiki> in BerryWiki format. Its
source of truth is `wiki/` in this repository. See `wiki/README.adoc` for how to
edit and publish it.

## Questions

Open an issue, or see `GOVERNANCE.adoc` for how decisions are made.
