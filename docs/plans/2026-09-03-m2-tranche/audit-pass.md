# M1.5-M1 Audit Pass — Verdict per task

This audit verifies which P1.5-M1 issues are already covered by code on `main`,
and splits them into COVERED (close now) vs PARTIAL (needs follow-on PR).

**Source:** `git log --oneline -5` from main + file inventory 2026-09-03.

## Summary

- **COVERED** (6 issues): #137, #138, #139, #140, #141, #142
- **PARTIAL** (2 issues): #143, #144
- **NOT_SHIPPED** (0 issues)

The 6 COVERED issues shipped across Phase 1 (initial scaffolding + UI work) but the
closes were never posted. This PR documents the evidence and posts the closes.

The 2 PARTIAL issues each get a follow-on PR tracked separately:
- **#143** (Stage receipt/log system): receipt type exists, emission path exists,
  but `WizardBottomSheet.svelte:55` is the only caller and passes no receipt; no UI
  display. → Follow-on PR: emit + display receipts.
- **#144** (Crash-safe autosave): `SessionStore` in
  `crates/astroforge-core/src/session.rs` is fully implemented with 2 unit tests;
  not wired to Tauri. → Follow-on PR: Tauri commands + Svelte autosave loop.

---

## Per-issue verdict

### #137 — Define `ProcessingMode` type and session-level mode state

**Status:** COVERED

**Evidence:** `src/lib/pipeline-store.ts:5`
```ts
export type ProcessingMode = "automagic" | "automagic_expert" | "pure_expert";
```

**Used by:** `setMode` (pipeline-store.ts:229), `ModeA`/`B`/`C`/`D` components, layout-mode store.

---

### #138 — Define `PipelineNode` and `PipelineGraph` TypeScript types

**Status:** COVERED

**Evidence:** `src/lib/pipeline-store.ts:21-50`
- `PipelineNode` (id, type, label, params, status, version, receipt?)
- `PipelineEdge` (from, to)
- `PipelineGraph` (nodes, edges)

Used by `sessionStore` initial graph (`pipeline-store.ts:172-188`).

---

### #139 — Implement session state store (Svelte writable store)

**Status:** COVERED

**Evidence:** `src/lib/pipeline-store.ts:207`
```ts
export const sessionStore = writable<SessionState>(createInitialSession());
```

Plus 6 derived stores (`currentMode`, `activeStepIndex`, `pipelineGraph`, `history`,
`canUndo`, `canRedo`, `activeNode`) at pipeline-store.ts:209-217.

`SessionState` type at pipeline-store.ts:83-94 covers every spec field.

---

### #140 — Implement Next Button action logic

**Status:** COVERED

**Evidence:** `src/lib/pipeline-store.ts`
- `commitStage(params, receipt?)` at :258 — commits params + receipt, advances step
- `goToStep(index)` at :316
- `nextStep()` at :334
- `prevStep()` at :340

**Caller:** `src/components/WizardBottomSheet.svelte:54-57`
```ts
function handleNext() {
  commitStage({ ...node.params, strength: sliderValue });
  nextStep();
  sliderValue = 0.5;
}
```

---

### #141 — Implement undo/redo history stack with versioned snapshots

**Status:** COVERED

**Evidence:** `src/lib/pipeline-store.ts`
- `HistoryEntry` type at :52-60 (versioned snapshot of params + receipt)
- `commitStage` pushes entry at :292-300
- `undo()` at :346 — restores prior params + bumps historyPointer
- `redo()` at :382
- `canUndo`/`canRedo` derived stores at :213-214

---

### #142 — Implement mode-switch logic with confirmation

**Status:** COVERED

**Evidence:** `src/lib/pipeline-store.ts:229` `setMode(mode, keepPixelState)`
- Confirmation flow at `src/components/WizardBottomSheet.svelte:87-92`
- Records `mode_switch` action in history at pipeline-store.ts:231-238
- `keepPixelState` flag controls whether pixel state is preserved

---

### #143 — Implement stage receipt/log system

**Status:** PARTIAL

**Evidence (positive):** `src/lib/pipeline-store.ts`
- `StageReceipt` type at :41-50 (stageId, timestamp, durationMs, parameters, warnings,
  metrics, engine?, success)
- `commitStage(params, receipt?)` accepts optional receipt; thread into state at :267-279
- `HistoryEntry.receipt` field stores receipt at :297

**Evidence (gap):**
- **Only 1 caller**: `src/components/WizardBottomSheet.svelte:55` calls
  `commitStage({ ...params, strength })` — passes no receipt
- **No UI display**: nothing in the components shows the receipt history

**Follow-on PR:** thread `durationMs`, `warnings`, `metrics` into commitStage call,
add a `ReceiptsPanel.svelte` to ModeC showing recent entries.

---

### #144 — Implement crash-safe autosave to local rusqlite

**Status:** PARTIAL

**Evidence (positive):**
- `crates/astroforge-core/src/db.rs:1-46` — full schema (projects, sessions,
  stage_runs, checkpoints +3 indexes)
- `crates/astroforge-core/src/session.rs:1-247` — `SessionStore` with full CRUD:
  - `create_project`, `create_session`, `record_stage_run`, `complete_stage_run`
  - `save_checkpoint`, `get_latest_checkpoint`
  - `get_session_status`, `set_session_status`
  - `find_interrupted_sessions` (crash recovery)
- 2 unit tests at session.rs:188-247
- Schema registered in `astroforge-core/src/lib.rs:32`

**Evidence (gap):**
- **No Tauri commands**: `src-tauri/src/main.rs` does not import `session::SessionStore`
- **No Svelte autosave loop**: nothing calls into the backend to persist on commit

**Follow-on PR:** mirror the GalleryStore pattern (PR #193):
- Register SessionStore in `main.rs` via `app.manage(SessionStore::new(...)?)`
- Add 3 Tauri commands: `session_create_project`, `session_record_stage`,
  `session_find_interrupted`
- Add Svelte `startAutosave(sessionId)` that calls `session_record_stage` on every
  `commitStage`
- Add 3 integration tests in `crates/astroforge-core/tests/session_persistence.rs`