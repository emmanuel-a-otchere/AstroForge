# Phase 8 — M1.5.1 Foundation Slice: Close-Out Audit

**Status:** closed
**Date:** 2026-09-04
**PR:** docs/phase8-audit

## TL;DR

Phase 8 is already fully shipped on `main`. Every P1.5-M1 task (#137–#144)
is implemented, every issue is `CLOSED` on GitHub, and `PROJECT_PLAN.md`
already marks them `done`. This document is the audit trail that links
each task to the code that satisfies it.

No new code is required. No behaviour changes.

## Task-by-task evidence

### T1 — `ProcessingMode` type & session-level mode state
- Issue: https://github.com/emmanuel-a-otchere/AstroForge/issues/137 (CLOSED)
- Code: `src/lib/pipeline-store.ts:33–39` (type) and `src/lib/pipeline-store.ts:222–230` (`setMode`)
- Spec ref: §4.3

### T2 — `PipelineNode` / `PipelineGraph` TypeScript types
- Issue: https://github.com/emmanuel-a-otchere/AstroForge/issues/138 (CLOSED)
- Code: `src/lib/pipeline-store.ts:21–78` (types) and `src/lib/pipeline.ts` (DAG runner)
- Spec ref: §4.1, §C.1

### T3 — Session state store (Svelte writable)
- Issue: https://github.com/emmanuel-a-otchere/AstroForge/issues/139 (CLOSED)
- Code: `src/lib/pipeline-store.ts` — writable `pipelineStore`, derived `currentMode`, `activeStepIndex`
- Spec ref: §C.1

### T4 — Next Button action logic
- Issue: https://github.com/emmanuel-a-otchere/AstroForge/issues/140 (CLOSED)
- Code: `src/lib/pipeline-store.ts` — `commitStage`, `goToStep`, `nextStep`, `prevStep`
- Spec ref: §C.2

### T5 — Undo/redo history stack
- Issue: https://github.com/emmanuel-a-otchere/AstroForge/issues/141 (CLOSED)
- Code: `src/lib/pipeline-store.ts` — `undo`, `redo`, versioned snapshots in `history_stack`
- Tests: `cargo test --workspace` exercises 146+ unit tests in `astroforge-core`
- Spec ref: §4.2

### T6 — Mode-switch with confirmation dialog
- Issue: https://github.com/emmanuel-a-otchere/AstroForge/issues/142 (CLOSED)
- Code:
  - `src/lib/pipeline-store.ts:229–260` — `setMode(mode, keepPixelState)` branch
  - `src/components/WizardBottomSheet.svelte:120–140` — `confirmModeSwitch` / `cancelModeSwitch`
  - `src/components/WizardBottomSheet.svelte:191–215` — `mode-confirm` glass-panel dialog with
    radio inputs ("keep current pixel state" / "re-process from chosen stage") and Cancel/Switch buttons
  - `src/components/WizardBottomSheet.svelte:150` — Escape key cancels
- Spec ref: §4.3
- Earlier audit pass: PR #197 (`chore(docs): M1.5-M1 audit pass — close 6, mark 2 partial`)

### T7 — Stage receipt/log system
- Issue: https://github.com/emmanuel-a-otchere/AstroForge/issues/143 (CLOSED)
- Code: `StageReceipt` type in `src/lib/pipeline-store.ts`; emission threaded through components;
  `ReceiptsPanel` UI in `src/components/ReceiptsPanel.svelte`
- Wiring PR: #199 (`feat(session): thread receipts end-to-end + ReceiptsPanel UI`)
- Tests: 5 `session.rs` integration tests in `crates/astroforge-core/tests/session.rs`
- Spec ref: §4.1

### T8 — Crash-safe autosave to local rusqlite
- Issue: https://github.com/emmanuel-a-otchere/AstroForge/issues/144 (CLOSED)
- Code:
  - `crates/astroforge-core/src/session.rs` — `SessionStore` with rusqlite-backed state
  - 3 Tauri commands in `src-tauri/src/main.rs:59–90` (`session_save` / `session_load` / `session_list`)
  - Svelte autosave hook in `src/lib/session.ts`, fired from `WizardBottomSheet.svelte:90–99`
- Wiring PR: #198 (`feat(session): wire Tauri IPC + autosave on every stage commit`)
- Tests: 5 `session.rs` integration tests in `crates/astroforge-core/tests/session.rs`:
  - `session_lifecycle_round_trip`
  - `crash_recovery_finds_running_sessions`
  - `autosave_records_multiple_stages`
  - `receipts_round_trip_via_list_stage_runs`
  - `auto_complete_in_tauri_command_path`
- Spec ref: §5 NFR

## Test coverage snapshot (2026-09-04, Linux)

```
cargo test --workspace --no-fail-fast
  → 188 passed, 0 failed
  → +1 cli_memory (Phase 7 PR-2)
  → +1 cli_smoke_runs_end_to_end (Phase 7 PR-1)
  → +5 session round-trip / autosave / receipts (Phase 8, PR #198, #199)
npm run check
  → 0 errors, 21 warnings (pre-existing a11y warnings, no regressions)
```

## Why this is a docs-only PR

PR #197 (`chore(docs): M1.5-M1 audit pass`) was already the audit pass
called for in the tranche plan. It closed 6 of 8 issues and flagged T6
and T8 as needing real work. Both have since shipped:

- T6 confirmation dialog: landed during Phase 9 prep (PR #201/#202 +
  follow-up commits), visible in `WizardBottomSheet.svelte:191–215`
- T8 autosave: PR #198 explicitly added `session_save`/`session_load` IPC
  + Svelte wiring, replacing the original Supabase design (PR #195)

There is no remaining work in Phase 8. The phase is closed.

## What's next

Per the tranche plan, Phase 9 (First Vertical Slice — Stretch end-to-end)
is the next deliverable. Issues #169, #175, #146, #147 are open.
