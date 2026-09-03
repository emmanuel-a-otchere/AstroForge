# Plan Changelog — AstroForge Phase 2 Tranche

## [0.1.0] ; 2026-09-03 ; Initial M2 tranche plan

**What this version does**

- Acknowledges the truth: Phases 1-6 of the Coder session shipped UI chrome, design
  system, local gallery, and Supabase removal — but the M1.5 processing pipeline (50
  open issues) and the MVP smoke tests (4 unfinished) are still pending.
- Reframes "M2" as a 3-phase tranche (Phases 7-9) that:
  1. Closes the MVP exit criterion (Phase 7).
  2. Ships the M1.5.1 foundation slice (Phase 8) — the bare minimum to make the train
     real.
  3. Ships ONE end-to-end vertical slice (Phase 9 — Stretch) that proves the
     architecture.
- Defers everything else (parallel stages, AI, history/versioning, smart-telescope,
  Phases 2-4 of original roadmap) until 7-9 close.

**What changed vs prior plan**

- Prior expectation: "M2 = real backend wiring, AI service, missing screens" (the
  implicit assumption after M1.5).
- This plan: M2 = finish what we said we'd ship in M1.5 (MVP + M1.5.1 + Stretch slice).
  Then re-plan M3 from there.

**Open decisions (awaiting user verdict)**

1. Stage pick for vertical slice: **Stretch** recommended.
2. Rendering API: **WebGL2** recommended.
3. Sample FITS fixture: confirm or generate synthetically.
4. Audit pass shape: combined PR vs per-issue PR (combined recommended).

**Risks flagged**

- `mvp_pipeline.rs` may have type-drift against current ingest/calibration/stacking
  signatures — budgeted for 1 rework PR.
- Smart-telescope detection (M1.5-M8) deferred; users on Seestar/Dwarf lose one feature
  this tranche. Acceptable per "vertical slice" principle.
- AI Service (M1.5-M5) deferred. Stretch slice ships with no AI; user applies defaults.

**Next action**

User picks the open decisions. Then the audit pass begins (Phase 8 step 1) — quickest
possible PR to close already-shipped M1.5 issues.