# Spec Delta — RT-10: Community Recipe Gallery (ADR-0021)

**Date:** 2026-11-06 · **Carrier for:** RT-10 (W10) / #428, resolving #119
(§17 item 3 — Open Items) · **Status:** Accepted

## What changes

Spec §17 item 3 ("Recipe gallery hosting: GitHub-based repo vs. static
JSON index (low cost)?") is **resolved: static in-repo JSON index**
(`recipes/index.json`), per [ADR-0021](../adr/0021-recipe-gallery-hosting.md).
The repository is the host and the curation surface (PR review); the app
bundles the index at build time — no hosting infrastructure, no runtime
fetches in v0.x.

## Behavior

1. The Recipes screen renders the bundled index as a read-only
   **Community gallery** section: name, target, equipment hints, stage
   count, integrity flag, and a per-entry Import action.
2. Importing a gallery entry runs the **same strict sanitized-recipe
   import** as a file import (§11.1 SharedRecipe → §23 strict validation
   → §28 security gate → RecipeEvent log). No new trust surface: gallery
   entries are curated by PR review (ADR-0021), and ADR-0019's "recipes
   carry no arbitrary code" applies unchanged.
3. The index is gated by CI
   ([`crates/astroforge-ai/tests/recipe_gallery_index.rs`](../../crates/astroforge-ai/tests/recipe_gallery_index.rs)):
   parse + required fields, ingest→export frame, NodeCatalog stage
   vocabulary, unique names, hub-verified model pins, honest integrity
   flags.

## Non-changes

- The 5-tab Recipe Library, the file Import… path, and the §11.2
  stripped-before-sharing rules are unchanged.
- No network fetches in v0.x; a fetch-from-raw fallback is a recorded
  follow-up in ADR-0021, not a commitment.

## Versioning

This delta rides the current 1.4.0 active spec; the §17 open item moves
to "resolved — see ADR-0021" at the next spec cut.
