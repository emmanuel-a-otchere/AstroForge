# EGUI Migration — status, architecture, retirement plan

The AstroForge desktop front-end is being converted from **Tauri 2 + Svelte 5**
(`src/` + `src-tauri/`) to a **Rust-native egui + wgpu app**
(`crates/astroforge-ui` + `crates/astroforge-render`), executed as a
strangler-pattern rewrite: both front-ends share one data directory and one
core, and the Tauri app remains the shipping front-end until the native app
passes human QA (see "Retirement" below).

**Branching:** this follows Model B — `main` ships the web UI, the protected
`egui/main` branch is the egui parity main, changes flow one-directionally
(`main` → `egui/main`, never the reverse except the labeled `cutover` PR).
Enforced by the `ui-isolation` CI job, CODEOWNERS, and branch protection —
see [BRANCHING.md](BRANCHING.md).

## Current status

| Phase | Scope | State |
|---|---|---|
| P0 | eframe shell, design tokens (docs/DESIGN.md port), semantic components, `Services` store mirroring, `api::project` slice | done (PR #487) |
| P1 | CPU render pipeline: magic-byte decode dispatch, ToneMap over core stretching, ColorImage conversion, synthetic sample frame | done (PR #487) |
| P1b | wgpu path: rgba16float textures painted via `egui_wgpu::CallbackTrait`, Apple M1 verified; CPU fallback intact | done (PR #488) |
| P2 | chrome: confirm modals (single destructive-action gate), rfd dialogs, drag-drop intake, per-screen window titles, statusbar | done (PR #489) |
| P3 | api surface port (~130 direct-call fns in 13 modules); six workspaces: Overview, Import, Process (background runs), Enhance (linear node canvas), Compare (A/B + diff + metric deltas), Export | done (PR #490) |
| P4 | Gallery screen, Recipes library (import/export/apply/delete), AI models panel (catalog + background download), `version_dag.rs` layered timeline (4 tests) | done (PR #490) |
| P5 | parity audit (CI-gated), this document, retirement checklist | done except retirement (below) |

Design tokens: `crates/astroforge-ui/src/tokens.rs` is the single source of
truth for look-and-feel (ported verbatim from `docs/DESIGN.md`); screens never
contain color/size literals. `theme.rs` is the only tokens->egui mapping;
`semantics.rs` is the component vocabulary (status chips with LED halos,
section cards, kv tables, empty states, error banners, nav items).

## Running the native app

```sh
cargo run -p astroforge-ui
```

It opens the SAME data directory as the Tauri app
(`~/Library/Application Support/com.astroforge.app` on macOS; override with
`ASTROFORGE_DATA_DIR`). Projects, recipes, sessions, and the gallery are
shared during the migration window.

Keyboard: `Cmd/Ctrl+1..6` global screens; `Cmd/Ctrl+Shift+1..6` project
workspaces (when a project is open); `Esc` -> Home.

## Parity audit

`python3 scripts/parity_audit.py` compares every command the web front-end
invokes against the direct-call api surface, and gates CI
(`.github/workflows/ci.yml` rust job). Currently: **147/147 covered**
within a documented two-entry allow-list:

1. `pipeline_run_session` — legacy one-shot MVP runner; pending until the
   Process workspace grows it.
2. `recipe_feed_list` — a DEAD CALL in `src/lib/recipeFeed.ts`: no such
   command exists in `src-tauri` or `astroforge-core` (runtime would fail
   with "command not found"). The egui api cannot port a command with no
   implementation; fix or delete the call in the web app.

## Known core findings (from the migration audit)

- `astroforge_core::export::export_tiff_16bit` writes an invalid
  BitsPerSample IFD entry for RGB (inline value instead of an offset
  array), so the crate's own `decoders::from_tiff_bytes` rejects RGB
  exports. Mono exports roundtrip fine. Affects the Tauri app's
  ImageCanvas too. Render tests use the `tiff` crate encoder meanwhile.
- `recipe_feed_list` (above).
- `SPEC_CATALOG` and the pipeline handler registry were duplicated into
  the egui `Services`; after retirement, move them into
  `astroforge-core` so there is a single construction site.

## Retirement (gated — do not run until QA sign-off)

Retire the web stack ONLY after all of these pass with real data on macOS,
Windows, and Linux:

- [ ] Import a real FITS session end-to-end (scan -> analyse -> confirm).
- [ ] Run the pipeline to completion; verify stage executions, pause/resume,
      cancel, retry, recommendations apply/dismiss.
- [ ] Enhance: reorder/insert/remove stages; save as recipe; re-apply it.
- [ ] Compare two real versions (canvases, diff, metric deltas).
- [ ] Export every format and verify outputs open in external tools.
- [ ] Recipes import/export round-trip; AI model download; gallery.

Then, in one commit:

1. `git rm -r src src-tauri src/lib public index.html svelte.config.js
   vite.config.ts vitest.config.ts vitest.setup.ts tsconfig.json package.json
   package-lock.json node_modules`
2. Remove the `src-tauri` workspace member and `@tauri-apps/*` remnants.
3. Rename the bin: `crates/astroforge-ui` `[[bin]] name = "astroforge-ui"`
   -> `astroforge` (the `crates/astroforge-app` CLI already owns the
   headless `astroforge` name — coordinate the rename, e.g. ship the GUI
   as `astroforge` and move the CLI to `astroforge-cli`).
4. Update `release.yml` from `tauri-apps/tauri-action` to
   `cargo build --release -p astroforge-ui` per-platform packaging.
5. Update `README.md`, `WORKFLOW.md`, `CONTRIBUTING.md` (drop npm/vite
   instructions; keep `cargo run -p astroforge-ui`).
6. Resolve the two allow-listed parity gaps and drop the allow-list.
