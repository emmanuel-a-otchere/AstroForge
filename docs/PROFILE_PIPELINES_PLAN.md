# Pipeline Profiles — Strategy & Implementation Plan

## Feature

**Save telescope-specific pipeline editing profiles for image processing, with full linear version history.**

- Named, reusable profiles (e.g. "DwarfII Smart Telescope · OSC · v3 — Core-Preserved")
- Telescope class + sensor type drive the profile (Smart Telescope, OSC, mono, etc.)
- **Linear** version history (v1 → v2 → v3 …) — every save increments version
- Apply profile to a session → seeds `pipeline-store` with the profile's stage params
- CRUD UI: create, list, view versions, apply, edit, save-as-new-version
- Persistence: rusqlite + Tauri IPC (mirror the existing `GalleryStore` pattern)

## Resolved decisions (2026-09-05)

| # | Decision | Resolution |
|---|---|---|
| D-1 | Versioning model | **2 — Linear history** (single `version` integer per profile; parent_version tracked for traceability; no branching) |
| D-2 | Apply surface | **1 — InitialDialog dropdown + processing-view header menu** |
| D-3 | Schema migration | **1 — Soft migration on load** (`migrate_v1_to_v2()` fn, persist as `2.0`) |
| D-4 | `color_calibration` appears twice (G2V WB + SCNR pass) | **Split** into `color_wb` + `color_scnr` (clean stage types; one instance per profile) |
| D-5 | `creative_polish` packs 5 concerns | **Catch-all** (single stage, multiple params) |
| D-6 | `coreProtectMask` flag location | **Session-level toggle** (NOT in profile; lives in `sessionStore.sessionFlags`, activates profile params at apply time) |

## Discovery — what already exists

The core data shape is **already designed and tested** — `crates/astroforge-core/src/recipe.rs` (292 lines, 7 unit tests):

- `Recipe { schema_version: "1.0", name, description, target_type, stages: Vec<RecipeStage>, required_models: Vec<String>, integrity: IntegrityBadge }`
- `RecipeStage { stage_id, enabled, params: HashMap<String, serde_json::Value> }`
- `IntegrityBadge { perceptual_models_used, deterministic_models_used, seed_recorded, models: Vec<ModelUsage> }`
- Operations: `new / add_stage / add_model / set_seed_recorded / to_json / from_json / sanitize_recipe / validate_compatibility / apply_recipe / integrity_label`
- `target_type` field already accommodates telescope class (existing test uses `"deep_sky"`; DwarfII would use `"smart_telescope_osc"` or similar)
- `sanitize_recipe` already strips paths, GPS, machine IDs — handles export safety

**What's missing** — Recipe is an orphan module:
- `pub mod recipe;` in `crates/astroforge-core/src/lib.rs:31` exposes the types — but **no other file imports them**.
- No persistence layer (no DB table, no rusqlite rows).
- No Tauri IPC commands (`#[tauri::command]`).
- No frontend TypeScript types mirroring `Recipe`.
- No UI surface (the `BeforeAfterSlider.svelte:6` `onSavePreset` placeholder is the only seam).
- `apply_recipe` returns `Vec<(String, HashMap<String, serde_json::Value>)>` — shape-compatible with `pipeline-store`'s `PipelineNode.params` (`Record<string, unknown>`), but the bridge function doesn't exist yet.

**Existing precedent to mirror:** the `GalleryStore` implementation in `crates/astroforge-core/src/gallery.rs` + Tauri commands in `src-tauri/src/main.rs` + IPC bridge in `src/lib/ipc.ts` + `Gallery.svelte` consumer. Same four-layer pattern: Rust core → Tauri command → IPC bridge → Svelte component.

## Dwarf2 profile → AstroForge stage mapping

The user's DwarfII pipeline doesn't map 1:1 to AstroForge's 10-stage taxonomy. Mapping:

| DwarfII stage | AstroForge stage(s) | Key params (DwarfII → AstroForge key) |
|---|---|---|
| **Pre-Processing** → hot pixel map | `ingest` | `hotPixelSigma: 3.0` |
| Pre-Processing → polynomial background | `background_extraction` | `polyOrder: 3` |
| Pre-Processing → multiscale wavelet denoise | `denoise` | `method: "wavelet"`, `layers: 3`, `strength: 0.25` |
| Pre-Processing → adaptive smoothing | `denoise` | `edgeProtect: true` (combine with above) |
| **Calibration & Stretching** → G2V WB | `color_calibration` | `wbReference: "G2V"` |
| Calibration → background neutralization | `color_calibration` | `bgNeutralRGB: [25, 25, 25]` |
| Calibration → histogram stretch | `stretch` | `midtone: 0.40`, `blackPoint: 0.02`, `highlights: 0.98` |
| **Detail Enhancement** → deconv sharpening | `sharpen_deconvolution` | `psfRadius: 2.0`, `iterations: 15`, `coreProtect: true`, `coreProtectFalloff: 0.6` |
| Detail → CLAHE local contrast | `creative_polish` | `claheRadius: 45`, `claheClip: 0.012` |
| **Color & Finalization** → SCNR green | `color_calibration` | `scnrStrength: 0.6`, `scnrTarget: "green"` |
| Color → saturation boost | `creative_polish` | `saturationBoost: 0.10`, `haOnly: true` |
| Final → Lanczos resample 4K | `creative_polish` | `upscaleTarget: [4096, 3072]`, `resampleMethod: "lanczos"` |
| Final → unsharp mask | `creative_polish` | `unsharpRadius: 1.2`, `unsharpAmount: 0.25` |
| **Core Detail Recovery** (delta) | `sharpen_deconvolution` | `iterations: 15` (was 25 → reduce) |
| Core → midtone refinement | `stretch` | `midtone: 0.40` (was 0.35 → raise) |

**Core preservation principle:** the DwarfII profile carries a session-level `coreProtectMask: true` toggle that activates Gaussian falloff around bright regions during deconvolution. This is a profile-wide modifier, not a single-stage param — needs `Recipe.integrity` or a new `Recipe.flags: Vec<String>` field.

## Critical decisions (3 forks — must resolve before implementation)

### D-1: Versioning model — branches or linear?

Three options:

**A. Linear history** (Git-like, single line)
- Every save = new version (v1, v2, v3 …)
- `Recipe` already has `schema_version` but that's the format version, not the user-content version
- Add `version: u32`, `parent_version: Option<u32>`, `created_at: String` to `Recipe`
- Stored as separate DB rows; UI shows linear timeline
- **Pro:** simplest model; matches "git log" mental model; trivial undo
- **Con:** can't experiment without losing main; can't A/B two approaches

**B. Branching** (Git-like, with branches)
- A profile has many versions, versions can branch
- Need `branch: String` field plus `version + parent_version`
- UI shows branch graph
- **Pro:** supports "try a tweak without losing main" workflow that astrophotographers actually want
- **Con:** 2–3× implementation cost; needs merge UX; needs conflict resolution

**C. Snapshots with named variants**
- A profile = a *named head* (e.g. "DwarfII Core-Preserved") pointing to a snapshot
- Multiple named heads allowed (e.g. "DwarfII v3" + "DwarfII experiment — more stretch")
- Each save updates the active head to a new snapshot
- **Pro:** middle ground — supports multiple concurrent approaches without full graph; UI is "list of profiles + version history per profile"
- **Con:** still needs the snapshot store; not much cheaper than A

**Recommendation: 2 — Linear history.** Reason — astrophotographers iterate ("try the same profile with stretch pushed harder"), but rarely need parallel branches. Linear gives them full history without merge UX. A "save as new version" action increments `version`; "revert to v1" reads the v1 row. **RESOLVED 2026-09-05.**

### D-2: Where does the profile apply?

Three insertion points:

**A. In the wizard flow** — between `session-setup` and `review-frames`, add a "Pick profile" step
- **Pro:** discoverable; users start from a profile rather than blank defaults
- **Con:** adds a step; might feel like friction

**B. In the processing view header** — a "Profile" dropdown next to the mode switcher
- **Pro:** zero-flow disruption; power-user feature
- **Con:** invisible to new users

**C. As a session-level metadata** — initial dialog asks "telescope class", and the matching profile auto-loads
- **Pro:** zero-click default experience
- **Con:** hides the feature; users don't learn they can override

**Recommendation: 1 — InitialDialog dropdown + processing-view header menu.** Reason — discoverable (pick at session start) without forcing a wizard step. Mid-session swap is a power-user feature exposed via header menu. **RESOLVED 2026-09-05.**

### D-3: Schema migration policy

Recipe has `schema_version: "1.0"`. When we add `version + parent_version + branch` (D-1) and any new params, that's a breaking change for any saved JSON.

Options:
- **A. Hard break** — bump to `"2.0"`, fail to load `"1.0"` files
- **B. Soft migration** — on load, if schema_version < current, run a migration function. Persist as `2.0`
- **C. Dual-read** — accept both schemas, normalize to current on load

**Recommendation: 1 — Soft migration.** Reason — existing 7 tests already use JSON roundtrip; once real users have saved profiles in the wild, breaking the format is hostile. The migration cost is small (1 fn, ~50 lines) and `validate_compatibility` already exists to gate the read. **RESOLVED 2026-09-05.**

## Scope proposal (3 milestones, in dependency order)

### Milestone A — "Profiles exist" (data + persistence)
- Extend `Recipe` with `version, parent_version, branch, created_at, flags`
- Add `crates/astroforge-core/src/recipe_store.rs` mirroring `gallery.rs`: `RecipeStore { add / list / get / list_versions / latest_for_branch }`, rusqlite-backed, `#[cfg(test)] mod tests`
- Wire into `crates/astroforge-app/src/main.rs` (or wherever the gallery commands live)
- Tauri command: `save_profile(profile: Recipe) -> Result<i64>`, `list_profiles() -> Vec<ProfileSummary>`, `get_profile(id: i64) -> Recipe`, `list_profile_versions(branch: String) -> Vec<RecipeSummary>`
- Frontend `src/lib/recipe-store.ts` mirroring `gallery.ts`
- IPC bridge in `src/lib/ipc.ts`
- Tests: Rust `cargo test` +12; frontend smoke test in `vite preview` of `RecipeSummary` shape

**Deliverable:** user can save a profile, list profiles, retrieve a version. **No UI yet — this is data-layer.**

### Milestone B — "Profiles apply" (integration)
- Bridge function `applyProfileToPipeline(profile: Recipe, pipeline: PipelineGraph) -> PipelineGraph` in `src/lib/pipeline-store.ts`
- InitialDialog gets "Telescope profile" dropdown (load profile list from IPC at mount)
- On `handleInitialConfirm`, if a profile is selected, `initSession(undefined, "pure_expert")` + apply profile to seed `nodes[].params`
- For each node, set `status: "active"` if profile.enabled, `"skipped"` if not
- Tests: store-level unit test for `applyProfileToPipeline` (DwarfII fixture mapping)

**Deliverable:** loading a DwarfII profile auto-fills the 10-stage pipeline graph with mapped params.

### Milestone C — "Profiles CRUD + versions" (UI)
- New `src/components/ProfileManager.svelte` (modal or panel)
- New `src/components/ProfileVersionHistory.svelte` (timeline list)
- Header dropdown in processing view (D-2 option B): "Profile: <name> ▾"
  - "Apply this profile"
  - "Save current params as new version"
  - "Fork from this version (save as new branch)"
  - "View version history…"
- Save-as-new-version flow: reads `pipeline-store` current params, builds a `Recipe`, calls `save_profile`
- Tests: frontend smoke test for ProfileManager modal open/close + save button

**Deliverable:** full CRUD UI; user can save, fork, and apply profiles with version history.

## Estimated effort

| Milestone | Dev time | Tests added | Risk |
|---|---|---|---|
| A — Data + persistence | 1.5 days | +12 Rust | Low (mirror GalleryStore) |
| B — Apply integration | 0.75 day | +5 TS | Medium (mapping correctness) |
| C — UI | 1.5 days | +5 frontend smoke | Medium (modal UX) |
| **Total** | **~4 days** | **+22** | |

## Dwarf2 v1 profile (canonical reference)

Once Milestone A lands, drop this `Recipe` into a seed script so the DwarfII profile ships out of the box:

```rust
Recipe {
    schema_version: "2.0".into(),
    name: "DwarfII Smart Telescope · OSC".into(),
    description: "Core-preserved detail enhancement for DwarfII OSC captures".into(),
    target_type: "smart_telescope_osc".into(),
    version: 1,
    parent_version: None,
    branch: "main".into(),
    created_at: "2026-09-05T00:00:00Z".into(),
    flags: vec!["coreProtectMask".into()],
    stages: vec![
        RecipeStage { stage_id: "ingest".into(), enabled: true, params: hashmap!{ "hotPixelSigma" => json!(3.0) } },
        RecipeStage { stage_id: "background_extraction".into(), enabled: true, params: hashmap!{ "polyOrder" => json!(3) } },
        RecipeStage { stage_id: "denoise".into(), enabled: true, params: hashmap!{ "method" => json!("wavelet"), "layers" => json!(3), "strength" => json!(0.25), "edgeProtect" => json!(true) } },
        RecipeStage { stage_id: "color_calibration".into(), enabled: true, params: hashmap!{ "wbReference" => json!("G2V"), "bgNeutralRGB" => json!([25, 25, 25]) } },
        RecipeStage { stage_id: "stretch".into(), enabled: true, params: hashmap!{ "midtone" => json!(0.40), "blackPoint" => json!(0.02), "highlights" => json!(0.98) } },
        RecipeStage { stage_id: "sharpen_deconvolution".into(), enabled: true, params: hashmap!{ "psfRadius" => json!(2.0), "iterations" => json!(15), "coreProtect" => json!(true), "coreProtectFalloff" => json!(0.6) } },
        RecipeStage { stage_id: "creative_polish".into(), enabled: true, params: hashmap!{ "claheRadius" => json!(45), "claheClip" => json!(0.012), "saturationBoost" => json!(0.10), "haOnly" => json!(true), "upscaleTarget" => json!([4096, 3072]), "resampleMethod" => json!("lanczos"), "unsharpRadius" => json!(1.2), "unsharpAmount" => json!(0.25) } },
        RecipeStage { stage_id: "color_calibration".into(), enabled: true, params: hashmap!{ "scnrStrength" => json!(0.6), "scnrTarget" => json!("green") } }, // applied as a second pass
        // stages not relevant to DwarfII: crop_rotate, star_handling, export
    ],
    required_models: vec![], // none currently — all deterministic
    integrity: IntegrityBadge { perceptual_models_used: false, deterministic_models_used: false, seed_recorded: false, models: vec![] },
}
```

(Conceptually; final wiring uses split `color_wb` + `color_scnr` per D-4.)

## Resolved during PR-A planning (2026-09-05)

- **D-4:** **Split** into `color_wb` + `color_scnr` (clean stage types; one instance per profile). Implementation impact:
  - Add `color_wb` and `color_scnr` to `PipelineStageType` enum in `src/lib/pipeline-store.ts`
  - Reorder `PIPELINE_STAGES` to place them after `color_calibration`
  - DwarfII seed maps: G2V WB → `color_wb`, SCNR green → `color_scnr`
  - **Final taxonomy:** `color_calibration` (background neutralization) + `color_wb` (white balance) + `color_scnr` (green/noise removal)
- **D-5:** **Catch-all** — `creative_polish` remains a single stage. Splitting it would balloon the enum (CLAHE/saturation/upscale/resample/unsharp = 5 new stage types) without clear benefit. The param set within is rich enough to express all 5 concerns.
- **D-6:** **Session-level toggle.** Implementation:
  - Add `SessionState.sessionFlags: Record<string, boolean>` field
  - `coreProtectMask: true` is set on session start (default `false`)
  - Profile `sharpen_deconvolution` stage has a `coreProtectRequired: true` param marker
  - At apply time, if `sessionFlags.coreProtectMask === false`, the param is omitted
  - Result: profile stays pure; user toggles the mask in the session UI; profile "just works" when the flag is on
- **D-7:** **Local-only for v1.** Sharing (export/import JSON) is a downstream feature; Recipe's JSON roundtrip is already there.
- **D-8:** **Hard-coded in Rust core** as `const DWARF2_V1: Recipe` + auto-seeded on first run (no `assets/seed/*.json` file). Keeps the demo experience tight — open the app, profile is already there.

## Sequencing recommendation

Decisions locked. Proceed in this order:

1. **PR-A** Data + persistence (~1.5 days): extend `Recipe` with `version + parent_version + created_at + branch (always "main")`, add `RecipeStore` (rusqlite) mirroring `gallery.rs`, Tauri commands + IPC bridge + TS types, `migrate_v1_to_v2()` fn, seed DwarfII v1. Also: add `color_wb` + `color_scnr` to `PipelineStageType` enum (D-4). No UI changes.
2. **PR-B** Apply integration (~0.75 day): `applyProfileToPipeline()` bridge, `sessionFlags` on `SessionState`, InitialDialog "Telescope profile" dropdown. E2E test: pick DwarfII → see 7 stages pre-populated with DwarfII params.
3. **PR-C** UI (~1.5 days): `ProfileManager.svelte` modal, version history timeline, header dropdown with "Save as new version". E2E test: open dropdown → save new version → see it in history → revert to v1.

Total: ~4 days, three PRs, +22 tests, zero new architectural debt.
