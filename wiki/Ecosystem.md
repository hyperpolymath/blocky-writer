<!-- berrywiki
id: 0195b100-0000-7000-8000-000000000005
parent: 0195b100-0000-7000-8000-000000000001
position: 3
kind: page
tags:
  - ecosystem
  - suite
archived: false
-->

<!--
SPDX-License-Identifier: MPL-2.0
-->

# Ecosystem

blocky-writer is one component of the hyperpolymath **precision document
suite**. Knowing the boundary stops you building something that already exists
somewhere else.

## The suite boundary, in one table

| Project | Owns | Does **not** own |
| --- | --- | --- |
| **blocky-writer** (this repo) | Fitting and placement into fixed-layout PDF and application-form boxes, baselines and per-character cells designed for hand spacing | Multi-format viewing, conversion, editing, OCR |
| [docmatrix](https://github.com/hyperpolymath/docmatrix) | Multi-format conversion and precision coordination for the suite | The tabbed viewer/editor; PDF geometry |
| [formatrix-docs](https://github.com/hyperpolymath/formatrix-docs) | Tabbed viewing/editing of one logical document across TXT, tabular, Markdown, AsciiDoc, Djot, DEED, Org, RST, Typst | Fixed-layout PDF placement |
| **ForthWall** (proposed, unbuilt) | Optional capability-bounded execution layer for critical operations | Anything mandatory; it is never an end-user product |

The authoritative statement of this boundary is
[docmatrix issue #71](https://github.com/hyperpolymath/docmatrix/issues/71),
which also carries the composition contract. If you are about to change what
blocky-writer claims to do, check that issue first — it is the versioned
contract, not this page.

## The composition contract, abbreviated

1. The user's native input stays authoritative unless they explicitly choose
   another representation.
2. Every conversion identifies its parser/renderer versions and declares what
   is preserved, approximated, unsupported or lost.
3. Intermediate representations are inspectable, content-addressed and linked
   to their source.
4. **No component silently normalises** punctuation, Unicode/whitespace,
   attribution, terminology, document structure or **page geometry**.
5. Ambiguous or lossy transformations are proposals requiring approval.
6. A composed operation carries exact input/output hashes, provenance, evidence
   spans or page coordinates, and a replayable operation record.
7. Each component verifies its own contract; an independent end-to-end verifier
   checks the composed result.
8. Audit/propose is the non-interactive default. Automatic mutation stays
   disabled for ambiguous or critical operations.

Clause 4 is the one blocky-writer is most likely to break. Page geometry is
this project's whole subject — normalising it "to be helpful" is exactly the
failure the contract forbids.

## Proof obligations do not transfer

* Conversion round-trip tests do not prove synchronised editor state.
* Editor tests do not prove fixed-layout PDF geometry.
* Exact page coordinates do not prove semantic correctness.
* Deterministic Forth execution does not prove the chosen operation or location
  was correct.

So: blocky-writer's green `cargo test` says nothing about docmatrix's
conversion correctness, and vice versa. Do not let a neighbour's green badge be
read as evidence about this repo.

## Adjacent projects worth knowing

| Project | Why you might care |
| --- | --- |
| [affinescript](https://github.com/hyperpolymath/affinescript) | The frontend language. `src/*.affine` targets it; blocky-writer is a real consumer. |
| [docudactyl](https://github.com/hyperpolymath/docudactyl) | HPC document extraction — OCR, NER, metadata. A plausible future source of field labels. |
| [dotmatrix-fileprinter](https://github.com/hyperpolymath/dotmatrix-fileprinter) | Sibling in the "matrix" naming line; filesystem-side manipulation. |
| [presswerk](https://github.com/hyperpolymath/presswerk) | High-assurance local print router. Downstream of anything that produces a filled PDF. |
| [recon-silly-ation](https://github.com/hyperpolymath/recon-silly-ation) | Cross-document consistency reconciler. Distinct from ForthWall. |
| [universal-language-server-plugin](https://github.com/hyperpolymath/universal-language-server-plugin) | "One server, all editors, universal document conversion." Overlaps docmatrix's remit; watch it. |
| [gv-clade-index](https://github.com/hyperpolymath/gv-clade-index) | Taxonomy registry. This repo's `CLADE.a2ml` is registered there. |
| [deed-ecosystem](https://github.com/hyperpolymath/deed-ecosystem) / [standards](https://github.com/hyperpolymath/standards) | Governance and the `.deed` format. |
| [ddraig-ssg](https://github.com/hyperpolymath/ddraig-ssg) / [casket-ssg](https://github.com/hyperpolymath/casket-ssg) | The two SSGs behind the Pages workflows. See the conflict noted in [CI-and-Gates](CI-and-Gates). |
| [berrywiki](https://github.com/metadatastician/berrywiki) | This wiki's format and tooling. |

## Cross-repo communication norm

There is no standing integration programme. The working pattern is: each repo
states its own boundary in its README, links the others, and files a short
issue on a neighbour when its own status changes in a way that unblocks or
affects them. Keep it to that — do not attempt a big-bang integration.
