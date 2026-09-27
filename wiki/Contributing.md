<!-- berrywiki
id: 0195b100-0000-7000-8000-000000000006
parent: 0195b100-0000-7000-8000-000000000001
position: 4
kind: page
tags:
  - contributing
archived: false
-->

<!--
SPDX-License-Identifier: MPL-2.0
-->

# Contributing

## Before you open a PR

1. Run the three Rust commands from [Home](Home). All three must pass.
2. If you touched a `uses:` line, regenerate `actions.lock` in the same commit
   and run `scripts/check-lock-sync.sh`.
3. If you removed or retired a CI check, add or update a row in
   `docs/ci/CHECK-DETERMINATIONS.adoc`. Never silence a gate with
   `continue-on-error`, `if: false`, or a quiet deletion.
4. Update `TOPOLOGY.adoc` if you changed a component's completion state.

## House style

* AsciiDoc (`.adoc`) for repository documentation. SPDX header on every file.
* `MPL-2.0` licence. Palimpsest philosophy.
* Conventional commits: `fix(rust): …`, `docs(ci): …`, `ci: …`.
* No new TypeScript, Python or Go. No npm/bun/yarn/pnpm dependencies.
* `.machine_readable/` is load-bearing — do not restructure it casually.

## The `REQUIRES_INITIALISATION` gate

`0-AI-MANIFEST.a2ml` points at `REQUIRES_INITIALISATION.adoc`, which lists
substitution tokens the repo template could not fill because they need a human
decision. Resolve what you legitimately can from evidence; leave the rest. Do
not delete the file to make a gate go green.

## Editing this wiki

The wiki is BerryWiki-format. Two ways to work on it:

**Directly in the GitHub wiki** — fine for prose tweaks. The pages stay plain
Markdown and GitHub renders them natively.

**From the repo** — preferred for anything reviewable:

```bash
# wiki/ in this repo is the source of truth
berrywiki check wiki          # tree + diagnostics; exit 1 on any error
berrywiki sidebar wiki --write # regenerate _Sidebar.md
```

Then push `wiki/` to the wiki remote:

```bash
git clone https://github.com/hyperpolymath/blocky-writer.wiki.git
cp wiki/*.md blocky-writer.wiki/
cd blocky-writer.wiki && git add -A && git commit -m "wiki: sync from wiki/" && git push
```

The metadata block at the top of each page is what BerryWiki reads. It must be
the **first non-blank content** of the file. Hierarchy comes from the `parent`
id chain, never from the filename; the `--` in filenames is a human-friendly
slug only. `berrywiki check` catches broken links, missing parents, cycles and
duplicate ids.

Never serve `<script>` from this wiki. That is a BerryWiki invariant and it is
asserted in BerryWiki's own test suite.

See [Contributing--Dev-Setup](Contributing--Dev-Setup) for getting a working
environment first.
