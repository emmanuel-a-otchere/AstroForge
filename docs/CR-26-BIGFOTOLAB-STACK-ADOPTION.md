# CR-26 bigfotolab Stack Adoption (Tauri/web UI)

Status: Proposed
Target: AstroForge v1.6.0
Priority: High
Depends on: BRANCHING.md (Model B, two protected mains), docs/DESIGN.md
("AstroForge Precision" token set), CR-03 (Application shell + Studio shell)
Enables: frameless custom-chrome window UX, component-library-driven UI on top
of the existing design tokens, routing/state/i18n foundations, auto-update,
window-state persistence

Reference implementation: `emmanuel-a-otchere/bigfotolab` (Tauri 2 + Vue 3 +
Vite 8 + pnpm + Tailwind 4/daisyUI, `src-vite/` tree, frameless window,
7 Tauri plugins).

⸻

## 1. Intent

AstroForge's web UI today is Tauri 2 + Svelte 5 with every layer hand-rolled:
runes stores, state-driven navigation, a custom CSS token system
(`src/app.css`), and ~70 Svelte components (~27.7k LOC of `.svelte`, ~14k LOC
of `.ts`). bigfotolab demonstrates a proven, shipping organization for the
same class of app (local-first desktop tool over a Rust core): a frameless
custom-chrome Tauri window, Vue 3 + vue-router + Pinia, Tailwind 4 + daisyUI
themed from a design-token file, a separate `src-vite/` frontend tree, pnpm +
Vite 8, vue-i18n, and a curated set of feature libs and Tauri plugins.

Adopting that stack lets us realize the "AstroForge Precision" design ethos
(docs/DESIGN.md) through a real component library instead of hand-written CSS
per component, and gives us routing, persisted state, i18n, and auto-update
foundations we currently lack. The Rust command surface and the on-disk
project format do **not** change; this is a front-end + shell adoption.

## 2. Adoption map

| Layer | bigfotolab | Decision | Rationale |
|---|---|---|---|
| Window shell | `decorations: false`, `transparent: true`, `macOSPrivateApi`, custom titlebar | **Adopt** | Core of the "shell" ask; needs a `TitleBar` component with drag region + min/max/close |
| `withGlobalTauri: true` | global `window.__TAURI__` bridge | **Adopt with mitigation** | Faster bridge code, but exposes Tauri to any injected script. Mitigate: capabilities allowlist + CSP (currently `csp: null` — must be fixed as part of this) + tight `assetProtocol` scope |
| Framework | Vue 3.5 (SFCs) | **Adopt via strangler tree** | Port into a new `src-vite/` tree; Svelte app in `src/` keeps shipping until parity, then is deleted |
| Routing | vue-router 5 | **Adopt** (in `src-vite/`) | Replaces `applicationNavTarget` / `studioViewport` state-nav |
| State | Pinia + `pinia-plugin-persistedstate` | **Adopt** (in `src-vite/`) | Replaces runes stores in `src/state/`; persisted state replaces manual `localStorage` hydration |
| Styling | Tailwind 4 + PostCSS + daisyUI 5 + cally | **Adopt, themed** | Tailwind v4 `@theme` maps 1:1 onto the DESIGN.md token palette; `src/app.css` tokens remain the source of truth |
| Build / package manager | Vite 8 + pnpm, frontend in `src-vite/` with own lockfile | **Adopt** | Root stays npm while `src/` exists; switch root to pnpm at cutover |
| i18n | vue-i18n, 17 locales | **Adopt structure, en-only first** | Locale pipeline ready; only `en` shipped initially |
| Feature libs | leaflet, vue-lazyload, vue-draggable-plus, date-fns, cally, video.js | **Adopt selectively** | leaflet → plate-solve/target map view; vue-lazyload → Gallery; vue-draggable-plus → node palette ordering; date-fns + cally → import/settings dates. **Skip video.js** (no video domain). Keep AstroForge's WebGPU/WebGL render libs — bigfotolab has no equivalent and ours are domain-specific |
| Tauri plugins | dialog, fs, os, process, shell, updater, window-state | **Adopt additions** | Add `os`, `process`, `updater`, `window-state`; keep existing dialog/fs/shell |
| Backend bridge | global Tauri + thin command wrapper | **Adopt pattern** | Port `src/lib/astroforge-api.ts` to `src-vite/src/common/tauri.js` (+ types); same command names/args, Rust side unchanged |
| Rust core | single crate, `t_*.rs`, rusqlite, native C/C++ shims | **Do not adopt** | Violates BRANCHING.md isolation gates (`src-tauri` must depend only on `astroforge-core`/`astroforge-ai`; `egui/main` shares those crates). SQLite is a data-model change — defer to its own CR if ever needed |

## 3. Target layout

```
src-vite/                     # new Vue frontend tree (mirrors bigfotolab)
  index.html
  package.json                # vue, vue-router, pinia, vue-i18n, tailwind, ...
  pnpm-lock.yaml
  vite.config.js              # port 3580 while src/ runs on 1420
  tailwind.config.js / postcss.config.js
  src/
    main.js  App.vue
    router/  stores/  composables/  common/  locales/  assets/
    views/     # Home, Projects, Recipes, AIModels, Settings, Help, Studio/*
    components/# TitleBar, shells, workspaces, dialogs, node graph, canvas
  tests/
src/                          # legacy Svelte app — ships until cutover, then deleted
src-tauri/tauri.conf.json     # frontendDist/devUrl flipped at cutover
```

## 4. Phases

### Phase 0 — Shell & security on the current app (ships immediately, low risk)
1. `tauri.conf.json`: frameless window (`decorations: false`, `transparent: true`), `macOSPrivateApi`.
2. Fix `csp: null` → restrictive CSP; extend `capabilities/default.json` for fs/shell/dialog scopes.
3. Register new plugins in `src-tauri/Cargo.toml` + `main.rs`: `os`, `process`, `updater`, `window-state`.
4. Add `assetProtocol` scoped to the AstroForge data dir only (`ASTROFORGE_DATA_DIR`).
5. Svelte `TitleBar` component (drag region, window controls, macOS traffic lights vs Windows/Linux buttons).
6. Exit: `npm run tauri dev` shows frameless app with working custom chrome on macOS/Windows/Linux.

### Phase 1 — Scaffold `src-vite/` (parallel tree)
1. `src-vite/` with pnpm + Vite 8 + Vue 3.5 + vue-router 5 + Pinia (+persistedstate) + vue-i18n (en) + Tailwind 4/daisyUI/cally.
2. Tailwind v4 `@theme` generated from docs/DESIGN.md tokens (script in `scripts/` so tokens stay single-sourced).
3. `common/tauri.js` bridge over `withGlobalTauri` + typed port of `astroforge-api.ts` (same command names).
4. Port `ApplicationShell` + `TitleBar` + Home + Projects; router with the same destinations.
5. Update `scripts/check_ui_isolation.py` `WEB_DIRS` to include `src-vite/` (web tree = `src/`, `src-vite/`, `src-tauri/`).
6. Exit: `pnpm --dir src-vite dev` (port 3580) runs the Vue shell alongside Svelte on 1420; both compile in CI.

### Phase 2 — Port the Studio (bulk of the work)
1. Port `StudioShell` + the six workspaces (Overview, Import, Process, Enhance, Compare, Export) and all dialogs, component by component, largest-value first.
2. Port `src/state/*` runes stores to Pinia stores; map `localStorage` keys to persistedstate.
3. Port `src/lib/*` (WebGPU/WebGL render, pipeline, node-graph, comparison) as framework-agnostic modules imported by the Vue app.
4. Port vitest suites from `src/lib/__tests__` + `tests/`.
5. Exit: parity checklist (per-screen: renders, IPC calls, shortcuts, keyboard nav) green; screenshots in `screenshots/` regenerated.

### Phase 3 — Feature libs
- vue-lazyload in Gallery; vue-draggable-plus in node palette/stage list; leaflet map view for plate-solve/target; date-fns/cally in import + settings.

### Phase 4 — Plugin UX
- window-state persistence, updater flow (with signing keys — open decision), os-specific behavior, process relaunch after update.

### Phase 5 — Cutover
1. Flip `tauri.conf.json` `build.frontendDist`/`devUrl`/`before*Command` to `src-vite`.
2. Delete `src/`, move pnpm to repo root, update root `package.json` scripts.
3. Update CI (`ui-isolation`, web test job) to the new tree; merge-forward to `egui/main` per BRANCHING.md (only shared crates/docs touched there).
4. Update README/CONTRIBUTING/WORKFLOW docs.

## 5. Compatibility guarantees

- Rust command names, arguments, and return types unchanged — `src-tauri/src/commands_*.rs` untouched except plugin registration.
- On-disk project/recipe/version format unchanged; no SQLite introduced.
- Keyboard shortcuts (⌘N/O/S/1–6, Esc) preserved.
- `egui/main` parity branch unaffected (it shares only `astroforge-core`/`astroforge-ai`); web-tree changes still land on `main` first per BRANCHING.md.

## 6. Risks & mitigations

| Risk | Mitigation |
|---|---|
| ~42k LOC port effort; regression per screen | Strangler tree — Svelte ships until parity; per-screen parity checklist; CI runs both trees in Phase 1–2 |
| `withGlobalTauri` widens attack surface | CSP set (removes current `csp: null`), capabilities allowlist, assetProtocol scoped to data dir |
| Frameless window quirks on Windows/Linux (resize borders, rounded corners, snap layouts, Wayland transparency) | Platform-specific CSS + `tauri` window API; test matrix in CI on all three OSes |
| Transparent window perf cost | `transparent: true` only where needed; measure on low-end Linux |
| Two package managers during migration | `src-vite/` owns its pnpm lockfile (bigfotolab pattern); root npm until cutover |
| CI `ui-isolation` rejects `src-vite/` | Extend `WEB_DIRS` in Phase 1 |
| LocalStorage keys lost at cutover | Migration shim in persistedstate plugin reading old keys once |

## 7. Verification per phase

- Phase 0: `npm run tauri dev` on macOS/Windows/Linux; `cargo check -p astroforge-app`... (`src-tauri` crate); `python scripts/check_ui_isolation.py`.
- Phases 1–4: `pnpm --dir src-vite build`, `pnpm --dir src-vite test` (vitest), `npm run check` (legacy until cutover), `cargo test -p astroforge-core`, ui-isolation script.
- Phase 5: full `npm run tauri build` bundles on all targets; `screenshots/` regenerated; ROADMAP updated.

## 8. Open decisions (need owner sign-off)

1. `withGlobalTauri: true` — accept global Tauri exposure (bigfotolab does, with capabilities), or keep module-only imports with a thin wrapper? Recommended: adopt + CSP.
2. i18n scope at cutover: en-only, or seed the 17 bigfotolab locales?
3. Updater: adopt `tauri-plugin-updater` now but defer signing-key setup until first signed release.
4. SQLite (bigfotolab's library DB): explicitly deferred — AstroForge's on-disk artifact model stays; revisit only if project indexing needs it.
