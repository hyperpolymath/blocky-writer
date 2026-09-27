<!-- berrywiki
id: 0195b100-0000-7000-8000-000000000004
parent: 0195b100-0000-7000-8000-000000000001
position: 2
kind: page
tags:
  - ci
  - governance
archived: false
-->

<!--
SPDX-License-Identifier: MPL-2.0
-->

# CI and Gates

Start here before you touch a workflow, a pin, or anything under `.github/`.

## The gate that actually matters

`.github/workflows/lock-sync-gate.yml` — **`actions.lock is in sync with the
workflow YAML`**. It carries no `uses:` of its own, so it is immune to the
failure it detects.

GitHub refuses to *start* any workflow whose step-level `uses:` refs are not
recorded, under that workflow's own path, in `.github/workflows/actions.lock`.
The refusal is silent: `startup_failure`, zero jobs, no log. A stale lock entry
is as fatal as a missing one.

**Rule: any change to a `uses:` line ships with the matching `actions.lock`
change in the same commit.** Verify with:

```bash
scripts/check-lock-sync.sh    # needs gawk
```

## The Rust gate

`.github/workflows/ci.yml`, job **`Rust core (tests, formatting, lint)`** — the
only job in that workflow:

| Step | Command |
| --- | --- |
| Check formatting | `cargo fmt --manifest-path rust/pdftool_core/Cargo.toml -- --check` |
| Run unit tests | `cargo test --manifest-path rust/pdftool_core/Cargo.toml --locked` |
| Run Clippy | `cargo clippy --manifest-path rust/pdftool_core/Cargo.toml --locked --all-targets -- -D warnings` |

## Other workflows

| Workflow | Purpose |
| --- | --- |
| `lock-sync-gate.yml` | The lockfile gate above. |
| `ci.yml` | The Rust core job. Deliberately contains **no** lockfile job — a lockfile checker inside a workflow that cannot start when the lock is broken can never report. |
| `codeql.yml` | CodeQL Advanced. |
| `governance.yml` | Governance checks. |
| `secret-scanner.yml` | Secret scanning. |
| `hypatia-scan.yml` | Neurosymbolic governance scan. |
| `mirror.yml` | Mirrors to GitLab, Bitbucket, Disroot, Gitea, Codeberg, SourceHut, Radicle. |
| `pages.yml` | Builds the Pages site with **Ddraig** (Idris 2). |
| `casket-pages.yml` | Builds the Pages site with **casket-ssg** (Haskell). |
| `boj-build.yml`, `instant-sync.yml`, `push-email-notify.yml`, `label-triage.yml`, `labels.yml` | Estate plumbing. |

## Known problems — read before you trust a green

* **Two Pages workflows.** `pages.yml` and `casket-pages.yml` both build and
  deploy to the `github-pages` environment under the same `pages` concurrency
  group, so they cancel each other. `pages.yml` also looks for `README.md`,
  which does not exist here (this repo uses `README.adoc`), so it would publish
  a bare `# hyperpolymath/blocky-writer` index. `casket-pages.yml` handles
  `README.adoc` correctly. **Which one is canonical is undecided.**
* **The bot cannot trigger Actions.** Every run whose actor is
  `arena-ai-coding-agent[bot]` is refused at startup with
  `Actor is not allowed to trigger Actions workflows`. A bot-authored PR shows
  *no* repository checks at all — not red, absent — because a startup failure
  creates no check run. This is an org-side policy, not something a file change
  can cure. Consequence: **the owner must trigger or merge**, and a bot merge
  leaves `main` looking red even when the files are correct.
* **`Branch-Floor` has no `required_status_checks` rule.** So "removed from the
  required set" is vacuously satisfied, and a red gate is a convention rather
  an enforcement. The permanent cure is owner-side.

## Where the determinations live

`docs/ci/CHECK-DETERMINATIONS.adoc` is the ledger. Every CI check that has ever
been red on `main` has exactly one determination — *fixed*, *retired* or
*exempt* — and none is ever closed by muting it (`continue-on-error`, demotion
to a warning, or silent removal). Read the standing rules at the bottom before
you diagnose anything.
