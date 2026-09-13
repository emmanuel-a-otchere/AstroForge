# ADR-07.7 — Decision Is User-Owned

**Status:** Accepted (2026-09-12, CR-07 audit + B1 slice)
**Source:** CR-07 §3 + §17 + §21 + §33, ADR-07.7

## Context

CR-07 §3 establishes the core principle that AstroForge measures image
characteristics; the user decides what constitutes the better image.
The §33 ADR is explicit:

> "Decision Is User-Owned — AstroForge can provide analysis and
> recommendations but does not silently determine artistic preference."

The §21 "No AI Winner by Default" rule further establishes that the
application must not auto-label an image as "AI Winner" unless the
user explicitly asks for automated ranking.

## Decision

`ImageDecision::promote()` is **explicit-only**. There is no
`auto_promote()`, no scheduler that watches `QualityAssessment` and
upgrades a version's state, and no implicit transition triggered by
pipeline completion. Every state transition records the user (or
their automation) as the agent in `DecisionHistoryEntry`.

`ImageDecision::state.next()` returns the canonical next state in the
promotion flow:

```
Working -> Candidate -> Preferred -> Final
```

Reject (`-> Rejected`) is allowed from any non-terminal state and is
a terminal state itself (no further transitions out of Rejected in
B1; restoring to Working requires a new `transition_to(Working)`
call, which is **not** in the canonical flow and is **not yet
implemented** to prevent accidental re-promotion of rejected work).

`Reference` is a terminal state for versions the user wants to keep
visible as reference material without promoting them through the
canonical flow.

The `QualityAssessment::verdict` is **advisory only**. It is not
written to `ImageDecision::state`. If the user wants to use a
verdict-driven automation, they must explicitly opt in (CR-07 §21
"Recommended based on selected quality criteria" framing).

## Consequences

- **Positive:** The user retains full authority over the decision.
- **Positive:** `ImageDecision::history` is an audit trail of who
  decided what, when, and why.
- **Positive:** Auto-promotion is impossible by construction —
  there is no code path that mutates `state` without an explicit
  method call.
- **Negative:** Batch workflows that want to auto-rank require
  explicit opt-in (the user must call a separate API).
- **Negative:** Un-rejecting a version requires either a new
  `transition_to(Working)` call (not in B1) or creating a new
  `ImageDecision` row. This is the safe default; accidental
  un-rejection is a worse failure mode than the friction of
  re-deciding.

## Alternatives considered

- **Auto-promote based on `QualityAssessment::verdict`.** Rejected:
  violates §3 core principle + §21 "No AI Winner by Default".
- **Scheduler-driven promotion.** Rejected: same.
- **Allow `Rejected -> Working` direct transition.** Deferred: the
  friction is intentional; users should consciously create a new
  decision rather than un-reject by accident.
