# ADR-0012 — CR-08 ADR-08.1: Recipe Represents Intent

- Status: Accepted (2026-09-22)
- Gates: `CR-08 §30`, `CR-08 §1`, `CR-08 §2`, `CR-08 §3`
- Closes: CR-08 §30 architectural decision records (sub-record 1/9)
- Source spec: `docs/CR-08-RECIPES-REPRODUCIBILITY-PROVENANCE.md` §30

## Context

CR-08 introduces Recipes as the canonical unit of
processing knowledge in AstroForge. Without an
explicit decision record, downstream code + UI work
will conflate Recipes with UI actions, with
implementation details, or with the Pipeline that
actually executes a Recipe against a dataset. The
audit row §30 flags this risk: "Recipes represent
processing intent rather than UI actions or
implementation details."

Three options were on the table:

1. **Recipe captures intent only.** The Recipe
   carries the user's goals (target type, style,
   quality profile, AI posture, processing
   objectives, quality targets) but never the
   concrete pipeline. The Pipeline is a
   dataset-specific projection.
2. **Recipe captures intent + concrete pipeline.**
   The Recipe remembers the exact pipeline that
   was last run against it, so re-applying is
   byte-for-byte identical.
3. **Recipe is the pipeline.** Treat the
   Recipe-Pipeline boundary as cosmetic; one
   structure with two names.

## Decision

**Adopt option 1: Recipes capture intent only.**
The Recipe is the durable knowledge artifact; the
Pipeline is dataset-specific projection derived
from the Recipe at apply time. The §10 Recipe
Editor (Beginner / Guided / Expert) collects intent
fields; the apply round projects them onto a
Pipeline via the §11 Recipe Application Flow.

### Why option 1

- **Knowledge reusability.** Intent (preserve star
  colors, maximize detail, target SNR 40 dB) is
  dataset-agnostic. The same Recipe applies across
  different captures, sessions, lighting, and gear
  with adaptive parameter derivation. Option 2
  would freeze the Recipe to one capture.
- **Clean separation of concerns.** The §21 data
  model keeps `Recipe` (intent fields +
  `stages: Vec<RecipeStage>`) and `Pipeline`
  (concrete stage parameters + execution log)
  distinct. Option 3 collapses them, breaking the
  data model.
- **Forward-compatible with the §22 quality
  catalog.** Natural / Detail / Clean / Publication
  profiles are intent-level axes; the §22 catalog
  ships canonical Recipes at each profile without
  committing to specific pipeline stages.

### Why not option 2

- Locks the Recipe to the first dataset it was
  applied against. Re-applying to a different
  dataset would force the user to either accept
  the wrong pipeline or duplicate the Recipe per
  dataset, defeating the purpose of reusability.

### Why not option 3

- Breaks the §21 data model. The Recipe struct
  carries intent-only fields (`quality_profile`,
  `ai_enhancement_level`, `processing_objectives`,
  `quality_targets`); collapsing into Pipeline
  would either bloat the Recipe with execution
  state or strip intent from the Pipeline.

## Consequences

- The Recipe struct stays intent-only. The §20
  validation pipeline runs against intent fields
  + the per-stage param ranges, not against
  concrete execution data.
- The apply round must consult
  `Recipe::effective_ai_enhancement_for_stage` +
  the §11 adaptive parameter derivation to
  project intent onto a Pipeline. This is
  enforced by the existing `recipe_apply` IPC.
- The §10.2 Guided tier's `processing_objectives`
  + `quality_targets` are intent fields (per the
  decision above); the §20 pipeline enforces
  their ranges but never persists execution
  outcomes.

## Related

- ADR-0013 (Recipe and Pipeline Are Distinct)
- ADR-0014 (Recipe Adaptation Must Be Explicit)
- ADR-08.1 source: `docs/CR-08-RECIPES-REPRODUCIBILITY-PROVENANCE.md` §30
- §1-§4 audit rows: `docs/CR-08-AUDIT.md`