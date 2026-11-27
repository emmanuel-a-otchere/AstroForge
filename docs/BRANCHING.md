# Branching model — Model B: two protected "mains" per UI strategy

AstroForge carries **two front-end strategies** that must never be merged
into one codebase:

| Branch | UI strategy | Role |
|---|---|---|
| `main` | Tauri 2 + Svelte 5 (`src/`, `src-tauri/`) | **Shipping** front-end; release source |
| `egui/main` | Rust-native egui + wgpu (`crates/astroforge-ui`, `crates/astroforge-render`) | **Parity** front-end; becomes shipping at cutover |

Both branches contain the full workspace (both UIs compile on both
branches — that is what lets us diff them), but the **trees stay
separated**: the web tree and the egui crates never import each other.
They meet only at:

- the shared crates `astroforge-core` / `astroforge-ai` (and the CLI
  `astroforge-app`), and
- the on-disk data directory (`ASTROFORGE_DATA_DIR`, default
  `~/Library/Application Support/com.astroforge.app` on macOS).

## The one rule: one-directional flow

```
        core/shared changes
                │
                ▼
   ┌────────────────────┐      merge-forward (main → egui/main)
   │       main         │ ───────────────────────────────────────► ┌─────────────┐
   │  (ships web UI)    │                                            │  egui/main  │
   │                    │ ◄── NEVER merge egui/main → main ───────── │ (egui UI)   │
   └────────────────────┘      except the labeled `cutover` PR      └─────────────┘
```

1. **All work lands on `main` first** (web fixes, core changes, egui
   work that only touches shared crates or docs).
2. **Merge-forward**: after every `main` change that touches
   `crates/astroforge-core`, `crates/astroforge-ai`, `Cargo.toml`, or
   CI, run:
   ```sh
   git fetch origin
   git checkout egui/main && git merge origin/main && git push origin egui/main
   ```
   At minimum, merge-forward **weekly** so the parity branch never drifts.
3. **`egui/main` → `main` is forbidden** except the final retirement
   `cutover` PR (see below). The `ui-isolation` CI job fails any such PR
   unless it carries the `cutover` label.

Local dual checkout:

```sh
git worktree add ../AstroForge-egui egui/main
```

## Enforcement (fail-safe layers)

1. **CI `ui-isolation` job** (`.github/workflows/ci.yml`, runs on `main`
   and `egui/main`) via `scripts/check_ui_isolation.py`:
   - static: egui crates contain no `src-tauri` reference; `src-tauri`
     depends only on core/ai; web `src/` imports nothing from `../crates`;
     each crate's `astroforge-*` deps stay within its allow-list;
   - PR: an `egui/*` → `main` PR requires the `cutover` label; a PR
     touching both UI trees requires the `allow-cross-tree` label.
2. **Branch protection** on both `main` and `egui/main`: required status
   check `CI` (strict), no force-push, no deletion. (If GitHub reports the
   workflow-level `CI` context as missing after a rename, list the
   individual `CI / <job>` contexts instead.)
3. **CODEOWNERS** (`.github/CODEOWNERS`): web tree, egui crates, and
   shared core each have an owner section, so cross-tree edits always
   surface a reviewer. Enable `require_code_owner_reviews` on the
   branches once a second contributor joins.
4. **Parity gate** (`scripts/parity_audit.py`): the egui direct-call API
   must keep covering every Tauri command the web UI invokes
   (147/147, documented allow-list), so `egui/main` can never silently
   fall behind `main`.

## Cutover (retirement of the web stack)

When the egui app passes the human-QA checklist in
`docs/EGUI_MIGRATION.md` on macOS, Windows, and Linux:

1. Land the retirement commit on `egui/main` (deletes `src/`,
   `src-tauri/`, and the npm/vite files — a dual-tree PR, so label it
   `allow-cross-tree`).
2. Merge-forward everything else, run full CI on `egui/main`.
3. Open the `egui/main` → `main` PR labeled **`cutover`** — the only
   sanctioned reverse merge. After it merges, `main` ships the egui app
   and the web stack is gone.
4. Update `release.yml`, `README.md`, `WORKFLOW.md`, `CONTRIBUTING.md`
   (per the retirement checklist in `EGUI_MIGRATION.md`).

## What "never merged together" does NOT mean

The two UIs are *not* forbidden from coexisting in one branch checkout —
both compile on both branches, which is deliberate (strangler pattern):
it lets `parity_audit.py` compare the command surfaces in CI and lets a
single core change be tested against both front-ends. What is forbidden
is **cross-imports** and **unlabeled cross-tree PRs / reverse merges** —
that is what keeps each UI strategy independently revertable, releasable,
and reviewable on its own "main".
