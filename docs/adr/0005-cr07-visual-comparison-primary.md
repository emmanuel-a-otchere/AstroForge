# ADR-07.2 — Visual Comparison Is Primary

**Status:** Accepted (2026-09-12, CR-07 audit + B1 slice)
**Source:** CR-07 §33, ADR-07.2

## Context

CR-07 §33 establishes that quantitative metrics support visual judgment;
they do not replace it. The existing `CompareWorkspace.svelte` (PR #308)
already places the image canvas as the dominant element in the layout.
The §3 core principle states explicitly:

> "AstroForge measures image characteristics; the user decides what
> constitutes the better image."

## Decision

The `ComparisonSession` data model treats the visual rendering as the
primary surface; metrics are attached to a `ComparisonMetric` row keyed
by `(session_id, region_id, version_id)`. The session does not store
a "winner" or "score" — that is the user's decision, recorded in
`ImageDecision`.

`QualityAssessment::verdict` is **advisory** (`StrongImprovement`,
`ImprovementWithTradeoffs`, `Neutral`, `Degradation`, `Inconclusive`)
and is never treated as authoritative. The user's `ImageDecision` is
the source of truth.

## Consequences

- **Positive:** The decision flow remains user-owned (ADR-07.7).
- **Positive:** The data model can represent "two versions have
  identical metrics but different visual character" — the metrics are
  not conflated with the verdict.
- **Negative:** Code that wants to find the "best" version in a set
  cannot query a session-level ranking; it must iterate decisions and
  metrics.
- **Negative:** Automated workflows (e.g. batch processing) cannot
  consume a session's verdict directly; they must wait for the user's
  decision or use a separate, explicit "auto-rank" mode (which
  CR-07 §21 requires to be opt-in and clearly labeled).

## Alternatives considered

- **Auto-ranked `ComparisonSet` with `ComparisonSet::winner_version_id`.**
  Rejected: this would conflict with ADR-07.7 (decision is user-owned).
  CR-07 §21 is explicit: AstroForge should not silently label an image
  "AI Winner".
