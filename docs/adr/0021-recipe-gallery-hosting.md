# ADR-0021 — Recipe Gallery Hosting

- Status: Accepted (2026-11-06)
- Gates: `RT-10 (W10)` / GitHub issue #428; resolves #119 (the recorded
  decision there was lost to a malformed comment — this ADR is the
  real record)
- Spec: §17 item 3 (Open Items) + §11.1 (sanitized SharedRecipe
  format); delta note in
  [../specs/DELTA_RT10_RECIPE_GALLERY.md](../specs/DELTA_RT10_RECIPE_GALLERY.md)

## Context

Spec §17 leaves recipe-gallery hosting as an open item: GitHub-based
repo vs. static JSON index (low cost), with CDN considerations for
global access. The app already has the two halves a gallery needs —
the sanitized [`SharedRecipe`](../../crates/astroforge-core/src/recipe_feed.rs)
format (§11.1) that travels between users, and a strict import path
(CR-08 §23 + §28 strict validation) that any gallery entry must
survive.

Options:

1. **Hosted service / CDN** — a real gallery backend or object store
   + CDN. Global, dynamic, but: infrastructure to run and pay for,
   moderation obligations, an upload path to build, and a trust
   surface (arbitrary JSON from strangers) the app is not yet
   equipped to police.
2. **GitHub-based repo as the host (static in-repo JSON index)** —
   `recipes/index.json` in this repository; the repo IS the host
   (raw.githubusercontent doubles as the CDN). Curation = PR review;
   validation = the CI gate; distribution = the index is bundled into
   the app at build time (`include_str!`) with zero runtime fetches
   in v0.x.
3. **Defer** — no gallery until Phase 4 proper.

## Decision

**Option 2 — static in-repo JSON index, curated by PR review.**
`recipes/index.json` is the gallery: an `index_version` envelope plus
SharedRecipe entries, gated by
[`crates/astroforge-ai/tests/recipe_gallery_index.rs`](../../crates/astroforge-ai/tests/recipe_gallery_index.rs)
(parse, required fields, ingest→export frame, NodeCatalog stage
vocabulary, unique names, hub-verified model pins, honest integrity
flags). The Recipes screen renders the bundled index as a read-only
Community section whose Import action feeds the existing strict
`recipe_import` path.

### Why

- **Zero infrastructure** — no service, no cost, no moderation queue
  outside the PR process the project already runs.
- **Curation is the trust model** — at v0.x scale, hand-vetted
  entries in a PR-reviewed file beat an open upload endpoint; the
  fail-closed import validation stays the second gate.
- **Offline-first** — the bundled index works with no network;
  a future fetch-from-raw fallback is additive.
- **The strict import path does the security work** — ADR-0019
  (recipes carry no arbitrary code) + §28 strict validation apply to
  gallery entries exactly as to file imports.

### Consequences

- Adding a gallery entry requires a PR (by design).
- The index ships with the app build; refreshing between releases
  means fetching raw `recipes/index.json` from the repo (follow-up —
  the bundled copy is authoritative in v0.x).
- Scale ceiling: when community volume makes PR-curation the
  bottleneck, revisit Option 1 with the moderation tooling that
  decision implies (recorded as a Phase 4 reconsideration, not a
  commitment).
