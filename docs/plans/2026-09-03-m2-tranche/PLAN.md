# AstroForge — Phase 2 Tranche Plan ("M2")

**Created:** 2026-09-03
**Scope:** Phases 7-9 of the AstroForge development programme (the tranche that opens the
post-M1.5 era, named "Phase 2" in the user-facing project plan to align with the existing
0/1/1.5/2/3/4 phase numbering in `docs/PROJECT_PLAN.md`).
**Replaces:** the implicit "next milestone after Phase 6 cleanup" expectation that the
unfinished M1.5 (50 open issues #138-#187) is the next thing to ship.

> **Truth call before any code.** Phases 1-6 of the Coder session delivered: Tauri 2
> shell, design-token system (32 vars), rusqlite GalleryStore + Tauri IPC bridge, and
> Supabase removal. **None of the M1.5 processing pipeline (P1.5-M1 through P1.5-M8)
> shipped** — all 50 of its issues (#138-#187) remain OPEN. Phase 1 also has 6
> unfinished tasks (P1-M5-T3/T4/T5/T6). Phase 2 onward is mostly spec'd but not started.
>
> This plan therefore reframes Phase 2 as: **finish what was promised, then ship one
> end-to-end vertical slice that proves the architecture works.** Spec-driven rule:
> every new task traces to a CR or the spec, no scope creep.

---

## How to read this plan

- **Phase 7** = close out Phase 1 MVP smoke tests (P1-M5-T3 → T6) — the MVP exit
  criterion from §16.
- **Phase 8** = ship the **M1.5.1 Foundation** slice (P1.5-M1-T1 → T8). The bare
  minimum to make the train real: types, store, commit/undo, mode switch, receipts,
  autosave. Without this, every M1.5 milestone is paint on empty walls.
- **Phase 9** = the **First Vertical Slice** — one end-to-end stage (Stretch, the
  highest-value single stage per CR §4.4) wired from the Svelte UI through the Rust
  orchestrator to the preview canvas. The goal is "I can load a FITS file, apply a
  stretch, and see the result" — even if everything else is still placeholder.

Phases 2-6 of the user's original roadmap (M2-M4 in PROJECT_PLAN §) remain
**deferred** until 7-9 close. No M2/M3/M4 work begins until this tranche ships.

---

## Phase 7 — Close MVP (Phase 1 exit criterion)

**Goal:** Honour the §16 MVP Definition of Done: FITS folder → TIFF on Windows,
macOS, and a 4 GB configuration, end-to-end, scripted.

**Scope (4 tasks):**

| ID | Task | GitHub issue | Status now |
|----|------|--------------|------------|
| P1-M5-T3 | Wire end-to-end pipeline: ingest → calibrate → register → stack → stretch → export | [#42](https://github.com/emmanuel-a-otchere/AstroForge/issues/42) | in_progress |
| P1-M5-T4 | Scripted smoke test: FITS folder → TIFF on Windows | [#43](https://github.com/emmanuel-a-otchere/AstroForge/issues/43) | pending |
| P1-M5-T5 | Scripted smoke test: FITS folder → TIFF on macOS | [#44](https://github.com/emmanuel-a-otchere/AstroForge/issues/44) | pending |
| P1-M5-T6 | Memory test: 30-frame stack on 4 GB without OOM | [#45](https://github.com/emmanuel-a-otchere/AstroForge/issues/45) | pending |

**Existing assets (no new modules needed):**
- `crates/astroforge-core/src/ingest.rs`, `calibration.rs`, `stacking.rs`,
  `stretching.rs`, `export.rs` — all present per the file inventory.
- `crates/astroforge-core/src/mvp_pipeline.rs` — likely the wiring target.
- `crates/astroforge-core/src/orchestrator.rs` — DAG runner.

**Risk:** if `mvp_pipeline.rs` exists but is wired against an older API, expect to fix
type drift against the actual `ingest::scan_folder` / `calibration::apply_*` /
`stacking::kappa_sigma_clip` signatures. Budget: 1 PR with rework if signatures diverge.

**Exit criterion:**
- `cargo test --workspace` passes with at least **3 new integration tests**:
  - smoke test (Windows runner)
  - smoke test (macOS runner)
  - memory test (Linux runner with `RUSTFLAGS="-C codegen-units=1"` + cgroup cap)
- A new `scripts/mvp_smoke.sh` in the repo root runs the smoke flow against a fixture
  folder (`tests/fixtures/sample-session/` with 5 synthetic FITS frames).
- `PROJECT_PLAN.md` P1-M5-T3 → done, T4/T5/T6 → done.

**Verification:**
- CI matrix: Windows + macOS + Linux runners all pass.
- Local dry-run: `bash scripts/mvp_smoke.sh tests/fixtures/sample-session` exits 0 and
  produces `output.tif` (~16-bit, valid TIFF header).

---

## Phase 8 — M1.5.1 Foundation Slice

**Goal:** Make the processing train a real piece of software, not a mock. Types, store,
commit/undo, mode switch, receipts, autosave — without these, no M1.5 stage can be
seriously built.

**Scope (8 tasks from P1.5-M1):**

| ID | Task | Issue | Notes |
|----|------|-------|-------|
| P1.5-M1-T1 | Define `ProcessingMode` type + session-level mode state | [#137](https://github.com/emmanuel-a-otchere/AstroForge/issues/137) | Types only. No behaviour yet. |
| P1.5-M1-T2 | Define `PipelineNode` / `PipelineGraph` TS types matching CR JSON model | [#138](https://github.com/emmanuel-a-otchere/AstroForge/issues/138) | Already done in `src/lib/pipeline-store.ts` (lines 21-39) — verify + close issue. |
| P1.5-M1-T3 | Implement session state store | [#139](https://github.com/emmanuel-a-otchere/AstroForge/issues/139) | Already done (`src/lib/pipeline-store.ts` writable + derived). Verify + close. |
| P1.5-M1-T4 | Next Button action logic | [#140](https://github.com/emmanuel-a-otchere/AstroForge/issues/140) | Already done (`commitStage`, `goToStep`, `nextStep`, `prevStep` in pipeline-store). Verify + close. |
| P1.5-M1-T5 | Undo/redo history stack with versioned snapshots | [#141](https://github.com/emmanuel-a-otchere/AstroForge/issues/141) | Already done (`undo`, `redo`). Verify + close. |
| P1.5-M1-T6 | Mode-switch logic with confirmation | [#142](https://github.com/emmanuel-a-otchere/AstroForge/issues/142) | Partial (`setMode(keepPixelState: boolean)`). Needs confirmation dialog. |
| P1.5-M1-T7 | Stage receipt/log system | [#143](https://github.com/emmanuel-a-otchere/AstroForge/issues/143) | `StageReceipt` type exists; emission needs wiring through Svelte components. |
| P1.5-M1-T8 | Crash-safe autosave to local rusqlite (was Supabase) | [#144](https://github.com/emmanuel-a-otchere/AstroForge/issues/144) | **NEW** — original task was Supabase; we re-routed to local rusqlite in PR #195. Spec already updated. |

**Strategy:**
1. **Audit pass (1 day):** open each issue, mark `done` if the code already covers it.
   Many are already shipped but issues weren't closed. This is the cheapest PR possible.
2. **Mode-switch confirmation dialog (1 PR):** `ModeSwitchConfirm.svelte` using
   existing tokens; calls `setMode(mode, true)` or `setMode(mode, false)` based on user
   choice. Closes P1.5-M1-T6.
3. **Autosave to rusqlite (1 PR):** mirror the `GalleryStore` pattern (Phase 4 #193).
   New `SessionStore` in `crates/astroforge-core/src/session_store.rs` + 3 Tauri
   commands + Svelte `loadAutosave`/`startAutosave` wiring. Closes P1.5-M1-T8.
4. **Receipt emission (1 PR):** thread `StageReceipt` through the components that
   already do commit. Closes P1.5-M1-T7.

**Exit criterion:**
- 8 P1.5-M1 issues closed (or marked deferred with explicit reason).
- `cargo test --workspace` passes; **at least 5 new tests** for SessionStore round-trip.
- `npm run check` 0 errors.

---

## Phase 9 — First Vertical Slice (Stretch)

**Goal:** Prove the architecture by shipping ONE end-to-end stage from UI to pixels.
The chosen stage is **Stretch (P1.5-M6-T7)** because:
- It has the highest user value (the "make the image look good" moment).
- It's mathematically contained (1D transfer functions).
- It maps directly to the existing `crates/astroforge-core/src/stretching.rs`.
- The MTF shader is the simplest meaningful WebGL effect.

**Slice (4 tasks):**

| ID | Task | Issue |
|----|------|-------|
| P1.5-M6-T1 | Stage 1 Ingest + Analyse (camera/filter/bit-depth/linear detection) | [#169](https://github.com/emmanuel-a-otchere/AstroForge/issues/169) |
| P1.5-M6-T7 | Stage 7 Stretch (Deep + multi-preview) | [#175](https://github.com/emmanuel-a-otchere/AstroForge/issues/175) |
| P1.5-M2-T2 | WebGL rendering pipeline (texture upload, full-screen quad, fragment shader) | [#146](https://github.com/emmanuel-a-otchere/AstroForge/issues/146) |
| P1.5-M2-T3 | MTF stretch shader in GLSL | [#147](https://github.com/emmanuel-a-otchere/AstroForge/issues/147) |

**Architecture:**
```
Wizard UI (Svelte)
  └─> Tauri command: load_fits(path) → F32Image
  └─> Tauri command: analyse(image) → DataType + ImageStats
  └─> Tauri command: stretch(image, params) → F32Image
  └─> Preview canvas (Svelte)
      └─> WebGL2 context
      └─> Texture upload from F32Image (CPU→GPU via Float32Array)
      └─> MTF fragment shader (3 channels, blackPoint/midtones/highlights)
      └─> Canvas present
```

**Stages to build (PRs):**

1. **PR-A: WebGL2 pipeline + neutral preview** — `crates/astroforge-core/src/ingest.rs`
   already loads FITS; add a Tauri command `load_fits(path) → {width, height, pixels}`.
   PreviewCanvas renders a flat grayscale texture. **Closes #146.**
2. **PR-B: Stretch stage** — `stretching.rs` MTF stretch on the Rust side, exposed as
   `apply_stretch` Tauri command. PreviewCanvas re-uploads the texture. **Closes
   partial #175.**
3. **PR-C: Multi-preview grid** — render 3 stretch variants side-by-side. **Closes
   #175 + partial #151 (Hold to Compare infrastructure).**
4. **PR-D: Ingest + DataType declaration** — analyse command produces
   `{cameraType, filterSet, bitDepth, isLinear, deviceModel}`. Svelte UI shows the
   declared data type as a header pill. **Closes #169.**

**Verification:**
- End-to-end manual test: load `tests/fixtures/sample-session/light_001.fits`, apply
  Deep stretch, see preview update.
- `cargo test --workspace` includes **2 new ingest tests** + **3 new stretch tests**.
- Headless screenshot of Mode C (Wizard) showing the ingest button + stretch preview.

---

## What's explicitly OUT of this tranche

| | |
|---|---|
| ❌ M1.5-M2 (other preview features — zoom/pan/compare/debounce) | not in tranche; deferred to Phase 10 |
| ❌ M1.5-M3 (Wizard mode bottom sheet — already exists in `WizardBottomSheet.svelte`, audit-only) | deferred to Phase 10 |
| ❌ M1.5-M4 (Forge mode node graph — `NodeSidebar`/`ParameterSidebar` exist, audit-only) | deferred to Phase 10 |
| ❌ M1.5-M5 (AI Service Layer) | defer until stretch slice ships; AI is optional |
| ❌ M1.5-M6 stages 2,3,4,5,6,8,9,10 | one stage per tranche; no parallel multi-stage |
| ❌ M1.5-M7 (history/versioning) | covered by Phase 8 (P1.5-M1-T5); M7 (versioned artefact store) deferred |
| ❌ M1.5-M8 (smart-telescope detection) | deferred; needs M6-T1 first |
| ❌ Phase 2-4 of original roadmap (AI Hub, narrowband, plate solving, planetary, plugin API) | deferred until M1.5 completes |

**Rationale:** This is a vertical-slice programme. Ship one path end-to-end; prove the
architecture; **then** parallelise across stages. Trying to ship all 10 stages + AI +
history + multi-preview at once is how we got 50 unfinished issues.

---

## Tranche timeline estimate (rough)

| Phase | Tasks | PRs | Wall-clock (solo dev, evenings) |
|-------|-------|-----|----------------------------------|
| 7 | 4 | 2-3 | 1-2 weeks |
| 8 | 8 | 4 (1 audit + 3 features) | 1 week |
| 9 | 4 | 4 | 2-3 weeks |
| **Total** | **16** | **10-11** | **4-6 weeks** |

---

## Decision points (to confirm with user before any code)

1. **Phase 9 stage pick.** Stretch is recommended (highest user value, contained maths).
   Alternatives: Ingest-only (lowest value), Crop (interactive but UI-heavy), Stretch
   (recommended).
2. **WebGL2 vs WebGPU.** WebGL2 is recommended (universal support, no extra deps).
   WebGPU gives better perf but smaller support matrix.
3. **Sample FITS fixture.** Need at least one real FITS file in
   `tests/fixtures/sample-session/` for the smoke test + stretch demo. AstroForge_Spec
   §5 mentions a sample; confirm or generate synthetically.
4. **Audit pass scope.** P1.5-M1 has 8 issues; many are already coded. Should the audit
   PR be one combined PR (closes 5+ issues) or one PR per already-coded task? Recommend
   combined: faster, clearer "we shipped M1.5.1" signal.

---

## Verification checklist (per phase)

For every PR:
- [ ] `cargo fmt --all -- --check` exit 0
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` exit 0
- [ ] `cargo test --workspace --no-fail-fast` all pass (new tests added)
- [ ] `npm run check` 0 errors
- [ ] `npm run build` exit 0
- [ ] No new cloud providers or external services introduced
- [ ] No new dependencies unless explicitly justified
- [ ] No `unwrap()`/`panic!()`/`console.log()` in shipped paths

---

## Machine-readable summary

```yaml
schema: astroforge/tranche-plan/v1
tranche:
  id: phase-2
  name: "M2 — Tranche after M1.5 cleanup"
  created: 2026-09-03
  status: pending-approval
  keeper: coder
phases:
  - id: 7
    name: "Close MVP exit criterion"
    goal: "Honour §16 MVP DoD: FITS folder → TIFF, scripted"
    tasks: 4
    issues: ["#42", "#43", "#44", "#45"]
    exit_criterion: "Smoke + memory tests green on Windows + macOS + Linux CI matrix"
    status: pending
  - id: 8
    name: "M1.5.1 Foundation Slice"
    goal: "Types, store, commit/undo, mode-switch, receipts, autosave"
    tasks: 8
    issues:
      ["#137", "#138", "#139", "#140", "#141", "#142", "#143", "#144"]
    exit_criterion: "All 8 P1.5-M1 issues closed; SessionStore has 5+ tests"
    status: pending
  - id: 9
    name: "First Vertical Slice (Stretch)"
    goal: "End-to-end FITS load → stretch → preview, one stage"
    tasks: 4
    issues: ["#169", "#175", "#146", "#147"]
    exit_criterion: "Manual demo: load fits, apply Deep stretch, see preview"
    status: pending
deferred:
  - id: M1.5-M2-stretch
    note: "Other PreviewCanvas features (zoom/pan/compare/debounce) deferred"
  - id: M1.5-M3
    note: "Wizard UI audit only; full mode wiring deferred"
  - id: M1.5-M4
    note: "Forge UI audit only; full mode wiring deferred"
  - id: M1.5-M5
    note: "AI Service deferred until stretch slice proves architecture"
  - id: M1.5-M6-other-stages
    note: "Stages 2,3,4,5,6,8,9,10 deferred (one stage per tranche)"
  - id: M1.5-M7
    note: "Versioned artefact store deferred; covered partially by Phase 8"
  - id: M1.5-M8
    note: "Smart-telescope detection deferred; needs M6-T1"
  - id: original-M2-M4
    note: "Phases 2-4 of original roadmap deferred until M1.5 completes"
repos:
  - name: AstroForge
    role: monorepo
    url: github.com/emmanuel-a-otchere/AstroForge
    head: ca3ff16
agents:
  - name: coder
    role: primary
    skills: [writing-plans, subagent-driven-development, test-driven-development]
```