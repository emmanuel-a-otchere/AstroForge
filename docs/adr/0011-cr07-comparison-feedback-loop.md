# ADR-07.8 — Comparison Is a Feedback Loop

**Status:** Accepted (2026-09-12, CR-07 audit + B1 slice)
**Source:** CR-07 §19 + §20 + §33, ADR-07.8

## Context

CR-07 §33 establishes that review can generate recommendations for
additional processing or enhancement. The §20 Intelligent
Recommendation After Comparison example is explicit:

> Version B has lower noise and better star separation, but increased
> highlight clipping. Recommended next step: Reduce stretch highlights
> before finalizing Version B.

This creates the §20 feedback loop:

```
Process -> Measure -> Compare -> Assess -> Recommend -> Improve
```

## Decision

`QualityAssessment` is the data carrier for the comparison-driven
recommendation. It carries:

- `summary: String` — the §11 natural-language assessment
  ("Strong improvement with minor trade-offs.")
- `findings: Vec<AssessmentFinding>` — per-finding severity +
  message
- `integrity_checks: Vec<IntegrityCheck>` — §12 astronomical
  integrity checks
- `verdict: Option<QualityVerdict>` — the §11 verdict
  (StrongImprovement / ImprovementWithTradeoffs / Neutral /
  Degradation / Inconclusive)

The recommendation engine can read `QualityAssessment` to produce
next-step suggestions. The existing `RecommendationEngine` +
`RecommendationRule` infrastructure (`recommendation.rs`, 557 LOC)
consumes the assessment and emits per-stage recommendations. B5 (CR-08
Provenance + AI) will wire this loop end-to-end.

The CR-07 §19 "Compare -> Continue Workflow" buttons (Continue
Enhancing / Create Branch / Mark Preferred / Export) live in the UI
(not in B1) but reference the data structures established here:
- "Continue Enhancing" -> `ComparisonItem::version_id` + the existing
  processing workspace
- "Create Branch" -> creates a new `ImageVersion` from the selected
  one (existing CR-02 domain model)
- "Mark Preferred" -> `ImageDecision::transition_to(Preferred, reason)`
- "Export" -> existing export pipeline

## Consequences

- **Positive:** The comparison data model is the bridge between
  review and re-processing; the recommendation engine consumes it
  directly.
- **Positive:** Users can iterate without leaving the workspace.
- **Negative:** The recommendation engine currently consumes
  pipeline-stage metrics (noise, sharpness, etc.) but does not yet
  consume `QualityAssessment` directly. Wiring this loop is B5's
  scope, not B1.
- **Negative:** Until the §19 UI buttons are implemented (B4), the
  data is correct but not exposed to the user.

## Alternatives considered

- **Tight coupling between `QualityAssessment` and `Recommendation`.**
  Rejected: would require moving recommendation logic into
  `comparison.rs` and break the existing module boundary. B5 will
  wire the two via the `recommendation.rs` consumption pattern.
- **Mandatory next-step recommendation.** Rejected: violates ADR-07.7
  (decision is user-owned); the user may choose to finalize without
  a recommendation.
