<!--
SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
SPDX-License-Identifier: MPL-2.0
-->

## What changed

<!-- One or two sentences. What does this PR do, and why? -->

## Type of change

- [ ] Bug fix (`fix:`)
- [ ] New feature (`feat:`)
- [ ] Documentation (`docs:`)
- [ ] CI / workflow / lockfile (`ci:`)
- [ ] Refactor, no behaviour change (`refactor:`)
- [ ] Chore (`chore:`)

## Local verification

<!-- Paste the actual output. If you could not run something — no Rust toolchain, no
     crates.io access, no gawk — say so explicitly rather than implying you did. -->

```bash
cargo fmt    --manifest-path rust/pdftool_core/Cargo.toml -- --check
cargo test   --manifest-path rust/pdftool_core/Cargo.toml --locked
cargo clippy --manifest-path rust/pdftool_core/Cargo.toml --locked --all-targets -- -D warnings
```

## Checklist

- [ ] I ran the three commands above (or explained why I could not).
- [ ] **If I changed a `uses:` line**, `actions.lock` is updated in this same commit and I ran `scripts/check-lock-sync.sh`.
- [ ] **If I removed or retired a CI check**, there is a matching row in `docs/ci/CHECK-DETERMINATIONS.adoc`.
- [ ] I have not added `continue-on-error`, `if: false`, or weakened any gate.
- [ ] If I changed a component's state, `TOPOLOGY.adoc` is updated.
- [ ] If I changed something the wiki asserts, the matching `wiki/` page is updated.
- [ ] Documentation that this PR makes stale has been updated in the same PR.

## Owner actions

<!-- Only needed when CI cannot run for this PR. Delete if CI is green. -->

- [ ] Merge as the repository owner, not via the bot. Runs triggered by
      `arena-ai-coding-agent[bot]` are refused at startup
      (`Actor is not allowed to trigger Actions workflows`), so a bot-authored PR
      shows **no** repository checks at all — not red, absent.

## Notes for the reviewer

<!-- Anything surprising, anything deliberately left alone, any follow-up you found
     but did not fix. -->
