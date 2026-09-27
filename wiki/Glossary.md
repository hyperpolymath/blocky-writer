<!-- berrywiki
id: 0195b100-0000-7000-8000-000000000008
parent: 0195b100-0000-7000-8000-000000000001
position: 5
kind: page
tags:
  - reference
archived: false
-->

<!--
SPDX-License-Identifier: MPL-2.0
-->

# Glossary

**AcroForm**
: The PDF interactive-form dictionary, reached from `catalog.AcroForm`.
  `fill_blocks` creates one if absent and sets `NeedAppearances`.

**A2ML**
: The `.a2ml` markup format used by `.machine_readable/`. Succeeded the `.scm`
  (Scheme) files in a 6SCM → 6A2 migration.

**AffineScript**
: The resource-safe, effect-typed language this project's frontend is written
  in. Compiles to WebAssembly. See
  [affinescript](https://github.com/hyperpolymath/affinescript).

**Appearance state (`AS`)**
: The `/AS` entry on a widget annotation naming which appearance stream in
  `/AP /N` is currently shown. Checkboxes and radio groups are written by
  setting this, not by writing a value.

**BerryWiki**
: A companion authoring and navigation layer for GitHub wikis: tree,
  backlinks, tags and a zero-JavaScript editor over plain Markdown, driven by a
  hidden HTML-comment metadata block. This wiki uses its format. See
  [berrywiki](https://github.com/metadatastician/berrywiki).

**Block**
: The struct blocky-writer exports: `{ label, x, y, width, height }`. One per
  detected widget annotation. Coordinates are PDF user-space, `f32`.

**CLADE**
: A taxonomy declaration in `.machine_readable/CLADE.a2ml`, registered with
  [gv-clade-index](https://github.com/hyperpolymath/gv-clade-index). This repo
  is clade `ap`, phase `active`.

**Contractile**
: A machine-readable constraint document. The family here is `MUST`, `TRUST`,
  `DUST`, `INTENT`, `ADJUST`, held under `.machine_readable/`.

**DEED**
: The `.deed` manifest format coordinated by
  [deed-ecosystem](https://github.com/hyperpolymath/deed-ecosystem).

**Determination**
: The ruling recorded in `docs/ci/CHECK-DETERMINATIONS.adoc` for a CI check
  that has been red on `main`. Exactly one of *fixed*, *retired* or *exempt*.

**ForthWall**
: A **proposed**, not implemented, capability-bounded Forth execution layer for
  critical precision operations. Do not treat any documentation, FFI surface or
  placeholder test as evidence that it exists.

**Palimpsest**
: The licence philosophy of this estate. MPL-2.0 text with additional
  emotional-lineage and AI-training provisions.

**RSR**
: Rhodium Standard Repository — the template and standard set this repo was
  minted from (`rsr-template-repo``).

**startup_failure**
: GitHub's silent refusal to start a workflow: zero jobs, no log. Three causes
  in this repo's history — lockfile drift, unlisted workflow, refused actor.
  Only the run's web-page annotation distinguishes them.

**Widget annotation**
: An annotation with `/Subtype /Widget`. `detect_blocks` keeps only these, and
  only when they carry a usable `/Rect`.
