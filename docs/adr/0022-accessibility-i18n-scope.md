# ADR-0022 — Accessibility & i18n Scope (WCAG, keyboard, screen reader, translations)

- Status: Accepted (2026-11-06)
- Gates: `RT-14 (W14)` / GitHub issue #432; records the decisions
  issues #130 / #131 / #132 carried (their close comments were
  malformed — `$2` — so, like ADR-0021 for #119, this ADR is the real
  record)
- Spec: §17 item 12 (Open Items)

## Context

Phase 4 M4.5 (Accessibility & i18n) deferred three decisions into the
recon tranche: WCAG AA scope (full in v1 vs. deferred), keyboard +
screen-reader support, and the i18n framework. Current state, from the
shipped UI (not aspiration): `aria-label` / `role` are used across the
surfaced screens (nav buttons with descriptions, `role="status"` /
`role="alert"` state lines, labelled canvas toolbar controls, a11y
`autofocus` warnings tripped in CI); focus management and full
keyboard paths are unaudited; all user-facing strings are English,
inline in components.

## Decision

**Progressive accessibility, deferred i18n.**

1. **WCAG 2.1 AA: targeted subset in v1.x, full audit post-v1.**
   In scope for v1.x (structural, cheap while the surface is small):
   complete keyboard operability (every action reachable, visible
   focus, no pointer-only gestures), name/role/value for all
   interactive elements (extend the existing aria usage to 100%
   coverage), and respectful `prefers-reduced-motion`. Out of scope
   until post-v1: formal WCAG audit/certification, screen-reader
   certification passes on NVDA/JAWS/VoiceOver (a manual matrix the
   project cannot honestly claim without testers on each), and
   contrast remediation tooling. #131 stays open against the v1.x
   subset with these gates.
2. **i18n: post-v1, English-only in v1.x.** Extracting strings now
   would freeze a message catalogue before the UI vocabulary has
   stopped moving (CR-10's editor surface is mid-flight); a premature
   `t()` indirection costs every subsequent slice and buys no user
   value yet. When CR-10 lands, adopt Svelte i18n (svelte-i18n or
   typesafe equivalent), extract to a neutral English catalogue, and
   invite community translations then — the README's translation
   contribution hook points at that moment. #132 stays open, blocked
   on CR-10.

### Why

- The honest version of WCAG AA is a tested claim; an untested
  "AA-compliant" label is worse than a documented subset.
- Keyboard + name/role/value are cheap now and expensive to retrofit;
  audits and translation catalogs are cheap later and churn now.
- RT-14 is tranche-P3 ("can slip past v0.1") by explicit roadmap
  design; this decision keeps v0.1 shippable without pretending at
  either certification or a localization that has no translators yet.

### Consequences

- v1.x release notes state the subset plainly (keyboard + ARIA +
  reduced-motion) and do not claim WCAG AA conformance.
- The full AA audit, SR certification, and i18n extraction become
  first-class post-v1 work items with #131/#132 as their homes.
