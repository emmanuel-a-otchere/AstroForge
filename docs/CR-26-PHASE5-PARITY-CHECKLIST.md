# CR-26 Phase 5 — Parity checklist (Svelte `src/` vs Vue `src-vite/`)

Status: In progress
Scope: gate document for the Phase 5 cutover (flip `tauri.conf.json` to
`src-vite/`, retire `src/`). Every row must be ✅ before cutover, or have an
explicit waiver recorded here with a follow-up issue.

Legend: ✅ full parity · ◐ functional core only · ❌ missing

## 1. Screen-level parity

| Route / screen | Svelte (`src/`) | Vue (`src-vite/`) | Status | Notes |
|---|---|---|---|---|
| Home | `HomeScreen.svelte` | `views/HomeView.vue` | ✅ | actions, recent projects, empty state |
| Projects | `ProjectsScreen.svelte` | `views/ProjectsView.vue` | ✅ | grid, rename/archive/delete, create |
| Recipes | `RecipesScreen.svelte` + Library/Detail/Editor/ProfileManager/Gallery | `views/RecipesView.vue` | ◐ | search/filter list only; detail/editor/profiles missing |
| AI Models | `AiModelsScreen.svelte` | `views/AiModelsView.vue` | ✅ | list + download |
| Settings | `SettingsScreen.svelte` | `views/SettingsView.vue` | ◐ | Vue adds Updates + System panels (Phase 4); Svelte-specific prefs not yet ported |
| Help | `HelpScreen.svelte` | `views/HelpView.vue` | ✅ | shortcuts + workflow |
| Studio shell | `StudioShell.svelte` | `components/StudioShell.vue` | ✅ | tab rail, close project, save indicator |
| Studio · Overview | `OverviewWorkspace.svelte` + `VersionTimeline.svelte` | `views/studio/OverviewWorkspace.vue` + `VersionTimeline.vue` | ✅ | checklist from project overview + timeline |
| Studio · Import | `ImportWorkspace.svelte` + 4 wizard steps | `views/studio/ImportWorkspace.vue` | ◐ | compact 4-step wizard over the same IPC commands + date filter (cally) + sky map (leaflet); per-frame classification overrides / provenance UI missing |
| Studio · Process | `ProcessWorkspace.svelte` + controls/panels | `views/studio/ProcessWorkspace.vue` | ◐ | sessions, auto-plan, stage list (drag-reorder), start/pause/cancel; node-graph canvas mode, recovery banner, save-as-recipe missing |
| Studio · Enhance | `EnhanceWorkspace.svelte` + `RecommendationCard.svelte` + `EnhancementStudio.svelte` | `views/studio/EnhanceWorkspace.vue` + `RecommendationCard.vue` | ◐ | recommendation accept/reject ✅; enhancement operation stacks (EnhancementStudio) missing |
| Studio · Compare | `CompareWorkspace.svelte` + tools/panels/canvas | `views/studio/CompareWorkspace.vue` | ◐ | A/B pick, metrics table, decision, lazy previews; interactive canvas (zoom/pan/sync), compare tools, comparison sets, region picker, expert panels, DAG, provenance missing |
| Studio · Export | `ExportWorkspace.svelte` | `views/studio/ExportWorkspace.vue` | ✅ | version + format pickers; export IPC pending in both |
| Dialogs | `ProjectDialog`, `DeleteProjectDialog` | `components/ProjectDialog.vue`, `DeleteProjectDialog.vue` | ✅ | create/rename + typed-name delete gate |
| Chrome | `ApplicationShell` + `TitleBar` | `components/ApplicationShell.vue` + `TitleBar.vue` | ✅ | frameless titlebar, nav, shortcuts (⌘N/O/S/1–6, Esc) |
| Update banner | — (new in CR-26) | `components/UpdateBanner.vue` | ✅ | Vue-only addition (Phase 4) |

## 2. Data-layer parity

| Layer | Svelte | Vue | Status | Notes |
|---|---|---|---|---|
| IPC surface | `lib/astroforge-api.ts` (all commands) | `src-vite/src/lib/astroforge-api.ts` (verbatim copy, invoke via `common/tauri.js`) | ✅ | same command names/args |
| Core stores | runes stores in `src/state/` | Pinia (`stores/`) for app/projects/context/dialog + `lib/store-compat.ts` shim for the rest | ✅ | logic preserved verbatim |
| Lifecycle | `state/project-lifecycle.ts` | `stores/lifecycle.js` | ✅ | open/close resets all project-scoped stores |
| Render libs | `webgpu-*`, `image-canvas-webgl`, `shaders` | copied verbatim into `src-vite/src/lib/` | ◐ | libs present; only the WebGL preview path is wired into a view (Compare previews); full interactive ImageCanvas not yet mounted |
| Tests | vitest (133) | node:test (9) in `src-vite/tests/` | ◐ | store/util coverage; component-level tests to be ported before cutover |

## 3. Shell / plugins

| Feature | Status | Notes |
|---|---|---|
| Frameless transparent window + custom titlebar | ✅ | Phase 0 (both trees share `tauri.conf.json`) |
| CSP + capabilities + asset protocol | ✅ | Phase 0 |
| window-state persistence | ✅ | Phase 4 (auto via plugin registration) |
| updater banner + relaunch | ✅ | Phase 4 (pubkey placeholder until signing) |
| os info | ✅ | Phase 4 |
| i18n structure | ◐ | vue-i18n with `en` only; 17-locale seeding is a post-cutover decision |

## 4. Verification commands (run per screen before marking ✅)

```sh
pnpm --dir src-vite dev          # Vue tree on :3580
npm run dev                      # Svelte tree on :1420 — compare side by side
pnpm --dir src-vite build && pnpm --dir src-vite test
npm run check && npm test        # legacy tree stays green until deletion
python3 scripts/check_ui_isolation.py
```

Manual pass per screen: load data → exercise primary action → keyboard
shortcuts → dialog open/cancel → close project → reopen.

## 5. Cutover gate

Cutover PR (Phase 5) may proceed when:
1. Every ◐ row above is either ✅ or has a waiver + follow-up issue.
2. Both dev servers render each screen without console errors.
3. CI green on the cutover branch: `frontend-vue`, `frontend` (until `src/`
   is deleted), `rust*`, `smoke*`, `ui-isolation`.
4. `tauri.conf.json` flipped to `src-vite` (frontendDist `../src-vite/dist`,
   devUrl `:3580`, before*Commands pnpm), root package manager moved to pnpm,
   `scripts/check_ui_isolation.py` WEB_DIRS updated (`src/` removed),
   merge-forward to `egui/main` per docs/BRANCHING.md.
