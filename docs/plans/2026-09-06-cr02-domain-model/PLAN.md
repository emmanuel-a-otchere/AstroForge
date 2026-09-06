# CR-02 Implementation Plan — Project, Session & Artifact Architecture

**Date:** 2026-09-06
**Source CR:** [CR-02 — Project, Session & Artifact Architecture](../../CR-02-PROJECT-SESSION-ARTIFACT-ARCHITECTURE.md) (Status: Proposed, Priority: Critical / Foundational)
**Strategy:** CR-02 §41 — phased migration, no big-bang rewrite. Each phase is one PR-sized tranche.

## Current state (as of bd1c059)

What already exists in `crates/astroforge-core`:

| CR-02 concept | Existing | Gap |
|---|---|---|
| Project | `db.rs::Project` (id, name, target_type) | No target_id FK, no status state machine, no schema_version, no project_root, no source_mode |
| Target | — | Absent entirely |
| Session | `db.rs::Session` + `session.rs::SessionStore` | No capture metadata, no profiles, no classification persistence |
| SourceAsset | — | Absent (no immutability/hash model) |
| Content hashing | — | Absent |
| Artifact | `artifact.rs::ArtifactRecord/ArtifactStore` | No categories, no lineage (parent ids), no producer linkage |
| ImageVersion | — | Absent |
| Recipe | `recipe.rs::Recipe` + `recipe_store.rs` (versioned) | Largely aligned; keep, reference by id |
| PipelineRun | — | Absent (stage_runs hang off session, not run) |
| StageRun | `db.rs::StageRun` (session-keyed) | Needs run-keyed domain type |
| Checkpoint | `db.rs::Checkpoint` | Path-only; no restore semantics |
| AIOperation | — | Absent (recipe.rs has ModelUsage for recipes only) |
| Export | `export.rs` (FITS/TIFF writer) | No Export domain record |
| project_events | — | Absent |
| Schema versioning | per-store `CREATE TABLE IF NOT EXISTS` | No `schema_migrations` table, no versioned migrations |

## Phases (CR-02 §41 → PR tranches)

| Phase | Sub-CR | Deliverable | Risk | Depends on |
|---|---|---|---|---|
| P0 | — | This plan + CR-02 doc saved | LOW | — |
| P1 | CR-02.1 | Domain types module (`domain.rs`): Project, Target, Session, CaptureMetadata, SourceAsset, Artifact (canonical), ImageVersion, PipelineRun, StageRunRecord, AIOperation, Export, ProjectEvent + enums (ObjectType, SourceMode, ArtifactCategory, ProjectStatus state machine, PipelineRunStatus) + unit tests | LOW | P0 |
| P2 | CR-02.2 | SQLite persistence: `schema_migrations` table, versioned migration v1→v2 adding CR-02 tables, repositories for Project/Target/Session/SourceAsset | MEDIUM | P1 |
| P3 | CR-02.3 | Artifact store: SHA-256 content hashing, atomic write (temp → validate → hash → rename → commit row), duplicate detection, integrity validation | MEDIUM | P1, P2 |
| P4 | CR-02.4 | Project lifecycle: create/open/close/rename/archive/delete/recover + project state machine enforcement | MEDIUM | P2 |
| P5 | CR-02.5 | Pipeline persistence: DAG execution records PipelineRun/StageRun/Checkpoint/Artifact lineage; crash-recovery detection | HIGH | P2, P3 |
| P6 | CR-02.6 | UI integration: frontend consumes project state via Tauri commands; `sessionStore` stops being authoritative for durable state | HIGH | P4, P5 |

**Estimated effort:** P1 ~0.5 day; P2/P3 ~1 day each; P4/P5 1–2 days each; P6 2–3 days. Sequenced, not parallel — each phase's schema/API is the next phase's contract.

## Decisions log

| # | Decision | Rationale |
|---|---|---|
| D-1 | Domain types land as a new `domain.rs` module alongside (not replacing) `db.rs` row structs | `db.rs` structs map the live v1 schema used by shipping IPC commands; replacing them in-place would break `session.rs`/`gallery.rs`/`recipe_store.rs`. CR-02.2 migrates the schema and converges the mappers. |
| D-2 | Recipe is NOT redefined in the domain module | `recipe.rs::Recipe` + `recipe_store.rs` already implement the versioned recipe model CR-02 §14 describes. Domain types reference recipes by `recipe_id` only. |
| D-3 | Domain StageRun is named `StageRunRecord`, keyed by `run_id` | Avoids collision with `db.rs::StageRun` (session-keyed). CR-02.5 makes PipelineRun the parent of stage runs. |
| D-4 | `ProjectStatus` transitions are validated in code (`can_transition_to`), EXPORTED → REVIEW allowed | CR-02 §21: EXPORTED must not be terminal. Encoding the state machine in the type layer now prevents invalid transitions leaking into CR-02.4 lifecycle code. |
| D-5 | Content hashing is SHA-256, hex-encoded, computed at import | CR-02 §7. Implemented in CR-02.3 (artifact store), but `content_hash` fields exist on the types from P1 so the schema doesn't churn. |
| D-6 | Ids are opaque strings (`String`), UUIDs generated at creation | Matches existing `db.rs` convention (`id TEXT PRIMARY KEY`). No UUID crate decision needed in P1 — generation lands with the repositories in P2. |
| D-7 | Per-project directory layout (CR-02 §18) is deferred to CR-02.4 | Current stores are app-data-dir scoped. The self-contained `Projects/<name>/project.db` layout arrives with project lifecycle, when `project_root` becomes meaningful. |

## Acceptance mapping

Each phase closes the CR-02 §39 acceptance criteria it owns:

- P1: type-level foundation only (no user-facing criteria)
- P2: Project (create/persist/stable id/multi-session/rename), Session (import/persist/classification)
- P3: Source Assets (immutability/hash/duplicates/missing-detection), Artifacts (identity/lineage/no-overwrite/integrity)
- P4: Standalone operation (project works offline, restored after restart)
- P5: Pipeline (runs/stages persistent, restart-surviving, checkpoint restore, stage rerun)
- P6: Image Versions + AI provenance surfaced in UI; reproducibility contract end-to-end

## Out of scope (CR-02 §40, confirmed)

Processing algorithms, AI models, histogram UI, recipe marketplace, cloud sync, telescope control, acquisition control.

## Status

| Phase | Status | PR |
|---|---|---|
| P0 | done | #233 |
| P1 | done | #234 |
| P2 | in_progress | this PR |
| P3–P6 | pending | — |
