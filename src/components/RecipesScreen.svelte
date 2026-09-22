<!--
  RecipesScreen — application-level Recipes destination.

  CR-05 R1: replaces the placeholder route for the "Recipes" nav
  target. Renders the saved pipeline profiles (CR-02 `RecipeStore`)
  as a list of cards with name, target type, and version. Selecting a
  card opens the existing `ProfileManager` modal so users get the
  full CRUD + version history UX from inside the application
  shell rather than only via a hidden menu entry.

  The component is read-only in this slice — it lists profiles and
  opens the editor; the editor's save/create behaviour is unchanged
  from Phase 1.5 PR-C (`src/components/ProfileManager.svelte`).
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { save as saveDialog, open as openDialog } from "@tauri-apps/plugin-dialog";
  import {
    loadProfiles,
    exportProfile,
    importProfile,
    profileStore,
    type RecipeSummary,
  } from "../lib/profile-store";
  import ProfileManager from "./ProfileManager.svelte";
  import RecipeLibrary from "./RecipeLibrary.svelte";
  import RecipeEditor, {
    type BeginnerTierPayload,
    type EditorPayload,
  } from "./RecipeEditor.svelte";
  import type {
    ProcessingObjectiveFromRust,
    QualityTargetsFromRust,
  } from "../lib/astroforge-api";
  import type {
    AiEnhancementLevelFromRust,
  } from "../lib/astroforge-api";

  let loading = $state(true);
  let error: string | null = $state(null);
  let managerOpen = $state(false);
  let beginnerEditorOpen = $state(false);
  let hasLoaded = $state(false);
  // CR-08 §10: live Beginner-tier payload. The
  // component emits every edit; the parent's
  // `saveBeginnerDraft` handler persists via
  // `recipe_save` when the user confirms.
  let beginnerDraft: BeginnerTierPayload = $state({
    name: "",
    targetType: "",
    qualityProfile: "natural",
    aiEnhancementLevel: "recommended",
  });
  // CR-08 §10.2 Guided tier: extended payload
  // surface. The Beginner tier's 4 fields remain
  // on `beginnerDraft`; the Guided tier fields
  // live alongside (defaulted empty / null).
  let guidedDraft = $state({
    processingObjectives: [] as ProcessingObjectiveFromRust[],
    qualityTargets: {
      target_snr_db: null,
      target_sharpness: null,
      target_background_smoothness: null,
    } as QualityTargetsFromRust,
    optionalOperations: [] as string[],
    stageOverrides: {} as Record<
      string,
      AiEnhancementLevelFromRust | null
    >,
    stageEnabled: {} as Record<string, boolean>,
  });
  // The set of stage IDs the editor's per-stage
  // inclusion + override sections render. Empty
  // by default; the Expert tier (ProfileManager)
  // is the source of truth for the stage list.
  let editorStageIds: string[] = $state([]);
  // CR-08 §19 file-dialog UI: status surface for
  // import / export operations. The dialog is a
  // Tauri-only feature; the action callbacks fall
  // back to a browser-mode prompt() when not running
  // inside the Tauri runtime so the UI flow still
  // exercises end-to-end.
  let importExportStatus: string | null = $state(null);
  let importExportError: string | null = $state(null);
  let importExportBusy = $state(false);

  function reportStatus(msg: string): void {
    importExportStatus = msg;
    importExportError = null;
  }

  function reportError(msg: string): void {
    importExportStatus = null;
    importExportError = msg;
  }

  function clearStatus(): void {
    importExportStatus = null;
    importExportError = null;
  }

  function slugify(s: string): string {
    return s
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "")
      .slice(0, 48);
  }

  async function onExportRecipe(
    summary: RecipeSummary,
  ): Promise<void> {
    clearStatus();
    importExportBusy = true;
    try {
      const payload = await exportProfile(summary.profileId);
      const defaultName = `${slugify(summary.name) || "recipe"}-v${summary.version}.afrecipe.json`;
      let chosen: string | null = null;
      if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
        const result = await saveDialog({
          title: `Export ${summary.name} v${summary.version}`,
          defaultPath: defaultName,
          filters: [
            { name: "AstroForge Recipe", extensions: ["json", "afrecipe"] },
            { name: "JSON", extensions: ["json"] },
          ],
        });
        chosen = typeof result === "string" ? result : null;
      } else {
        // Browser-mode fallback: prompt for a path. The
        // caller can paste a path the test harness can
        // observe (or hit Cancel).
        const input = window.prompt(
          `Export to path (browser-mode placeholder):`,
          defaultName,
        );
        chosen = input && input.trim() ? input.trim() : null;
      }
      if (!chosen) {
        reportStatus(`Export of ${summary.name} cancelled.`);
        return;
      }
      await writeFile(chosen, payload);
      reportStatus(
        `Exported ${summary.name} v${summary.version} to ${chosen}.`,
      );
    } catch (err) {
      reportError(
        err instanceof Error ? err.message : String(err),
      );
    } finally {
      importExportBusy = false;
    }
  }

  async function onImportRecipe(): Promise<void> {
    clearStatus();
    importExportBusy = true;
    try {
      let chosen: string | string[] | null = null;
      if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
        chosen = await openDialog({
          title: "Import AstroForge Recipe",
          multiple: false,
          directory: false,
          filters: [
            { name: "AstroForge Recipe", extensions: ["json", "afrecipe"] },
            { name: "JSON", extensions: ["json"] },
          ],
        });
      } else {
        const input = window.prompt(
          "Path to .afrecipe or .json file (browser-mode placeholder):",
        );
        chosen = input && input.trim() ? input.trim() : null;
      }
      if (!chosen || Array.isArray(chosen)) {
        reportStatus("Import cancelled.");
        return;
      }
      const json = await readFile(chosen);
      const summary = await importProfile(json);
      reportStatus(
        `Imported ${summary.name} v${summary.version}.`,
      );
    } catch (err) {
      reportError(
        err instanceof Error ? err.message : String(err),
      );
    } finally {
      importExportBusy = false;
    }
  }

  // Browser-mode shim: read a file via FileReader when
  // not running inside the Tauri runtime. Tauri mode
  // uses the file-system plugin (loaded by ProfileManager
  // already) so we don't need a Tauri-side helper here.
  async function readFile(path: string): Promise<string> {
    if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
      // Delegate to the @tauri-apps/plugin-fs readTextFile
      // via dynamic import (keeps the package.json light
      // for callers that don't exercise import).
      const fs = await import("@tauri-apps/plugin-fs");
      return fs.readTextFile(path);
    }
    // Browser-mode: best-effort fetch (works for the
    // sample .afrecipe fixtures when running a static
    // test page). Caller can override via the prompt.
    const response = await fetch(path).catch(() => null);
    if (!response || !response.ok) {
      throw new Error(`Could not read ${path} (browser-mode fetch failed).`);
    }
    return response.text();
  }

  // Write a string to a path. Tauri uses the file-system
  // plugin; browser-mode is a no-op + status report so the
  // user knows the bytes are in memory only.
  async function writeFile(path: string, contents: string): Promise<void> {
    if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
      const fs = await import("@tauri-apps/plugin-fs");
      await fs.writeTextFile(path, contents);
      return;
    }
    // Browser-mode: stash the payload in localStorage so
    // the test harness can observe it.
    try {
      localStorage.setItem(
        `astroforge.export.${path}`,
        contents,
      );
    } catch {
      // localStorage quota may be exceeded; swallow.
    }
  }

  onMount(async () => {
    try {
      await loadProfiles();
      hasLoaded = true;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  });

  // profileStore is the writable source; refresh after
  // ProfileManager saves a new version. The §15
  // RecipeLibrary.svelte subscribes to $profileStore
  // directly, so no local mirror is needed.
  $effect(() => {
    void loadProfiles();
  });
</script>

<section class="recipes-screen" aria-label="Recipes">
  <header class="screen-header">
    <div>
      <h1 class="font-display">Recipes</h1>
      <p class="font-body">
        Reusable processing workflows. Open a recipe to view its stages and
        version history; the existing Profile Manager handles edits and
        new-version saves.
      </p>
    </div>
    <div class="header-actions">
      <button
        type="button"
        class="secondary-cta font-label"
        onclick={onImportRecipe}
        disabled={importExportBusy}
        data-testid="import-recipe-btn"
        aria-label="Import recipe from .afrecipe file"
      >
        <span class="material-symbols-outlined" aria-hidden="true">
          file_download
        </span>
        Import…
      </button>
      <button
        type="button"
        class="manage-cta font-display"
        onclick={() => (beginnerEditorOpen = true)}
        aria-label="Create a new recipe using the Beginner tier editor"
        data-testid="beginner-new-recipe-btn"
      >
        <span class="material-symbols-outlined" aria-hidden="true">
          add_circle
        </span>
        New Recipe (Beginner)
      </button>
      <button
        type="button"
        class="secondary-cta font-label"
        onclick={() => (managerOpen = true)}
      >
        <span class="material-symbols-outlined" aria-hidden="true">
          bookmark_manager
        </span>
        Manage recipes
      </button>
    </div>
  </header>

  {#if importExportStatus}
    <p class="state-line status font-body" aria-live="polite" data-testid="import-export-status">
      {importExportStatus}
    </p>
  {/if}
  {#if importExportError}
    <p class="state-line error font-body" role="alert" data-testid="import-export-error">
      {importExportError}
    </p>
  {/if}

  {#if loading}
    <p class="state-line font-body" aria-live="polite">Loading recipes…</p>
  {:else if error}
    <p class="state-line error font-body" role="alert">{error}</p>
  {:else if !hasLoaded}
    <p class="state-line font-body" aria-live="polite">
      Recipe store unavailable in this build.
    </p>
  {:else if $profileStore.length === 0}
    <div class="empty-card font-body" aria-live="polite">
      <span class="material-symbols-outlined" aria-hidden="true">bookmark</span>
      <h2 class="font-display">No recipes yet</h2>
      <p>
        Recipes are saved from a session's pipeline. Process a project once
        and use the Process controls to save the current pipeline as a
        reusable recipe.
      </p>
    </div>
  {:else}
    <!--
      CR-08 §15: Recipe Library 5-tab layout (System / My Recipes /
      Project Recipes / Imported / Recently Used). The library is a
      pure consumer of `profileStore`: no separate IPC round-trip.
      Per-tab classification happens client-side against the
      `isSystem / isImported / lastUsedAt` flags folded onto the
      head row by §15's IPC contract change.
    -->
    <RecipeLibrary />
  {/if}

  {#if managerOpen}
    <ProfileManager onClose={() => (managerOpen = false)} />
  {/if}

  <!--
    CR-08 §10 Beginner tier editor modal. The
    Beginner tier's four fields (name + target_type +
    quality_profile + ai_enhancement_level) collect
    via `onChange`; the persisted save round-trip
    through `recipe_save` lands in a follow-on slice
    that wires the modal's "Save" button to the
    IPC. For this slice the modal renders the
    Beginner surface as a preview so the user can
    see the §10 progressive-disclosure shell shape.
  -->
  {#if beginnerEditorOpen}
    <div
      class="beginner-modal"
      role="dialog"
      aria-modal="true"
      aria-label="Beginner tier recipe editor"
      data-testid="beginner-editor-modal"
    >
      <button
        type="button"
        class="beginner-modal-backdrop"
        aria-label="Close Beginner editor"
        onclick={() => (beginnerEditorOpen = false)}
      ></button>
      <div class="beginner-modal-panel" role="document">
        <header class="modal-header">
          <h2 class="font-display">New Recipe: Beginner tier</h2>
          <button
            type="button"
            class="modal-close font-label"
            onclick={() => (beginnerEditorOpen = false)}
            aria-label="Close Beginner editor"
          >
            Close
          </button>
        </header>
        <RecipeEditor
          initial={{
            ...beginnerDraft,
            ...guidedDraft,
          } as EditorPayload}
          stageIds={editorStageIds}
          onChange={(next) => {
            beginnerDraft = {
              name: next.name,
              targetType: next.targetType,
              qualityProfile: next.qualityProfile,
              aiEnhancementLevel: next.aiEnhancementLevel,
            };
            guidedDraft = {
              processingObjectives: next.processingObjectives,
              qualityTargets: next.qualityTargets,
              optionalOperations: next.optionalOperations,
              stageOverrides: next.stageOverrides,
              stageEnabled: next.stageEnabled,
            };
          }}
        />
        <footer class="modal-footer">
          <p class="font-body modal-footer-note">
            Beginner captures the 4 essentials. Guided adds
            objectives + stage inclusion + AI overrides +
            quality targets + optional operations. Expert is
            the Profile Manager.
          </p>
          <button
            type="button"
            class="modal-save font-label"
            disabled
            title="Save round-trip wires to recipe_save IPC in a follow-on slice"
            data-testid="beginner-save-btn"
          >
            Save Recipe (preview)
          </button>
        </footer>
      </div>
    </div>
  {/if}
</section>

<style>
  .recipes-screen {
    display: flex;
    flex-direction: column;
    gap: var(--sp-lg);
    padding: var(--sp-xl);
    overflow-y: auto;
    height: 100%;
  }

  .screen-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: var(--sp-lg);
  }

  .screen-header h1 {
    margin: 0 0 var(--sp-xs) 0;
    font-size: 1.5rem;
  }

  .screen-header p {
    margin: 0;
    color: var(--on-surface-variant);
    max-width: 60ch;
  }

  .manage-cta {
    display: inline-flex;
    align-items: center;
    gap: var(--sp-xs);
    background: var(--primary);
    color: var(--on-primary);
    border: none;
    padding: var(--sp-sm) var(--sp-md);
    border-radius: var(--radius-md);
    cursor: pointer;
    font-size: 0.9rem;
    white-space: nowrap;
  }

  .manage-cta:hover {
    filter: brightness(0.92);
  }

  .state-line {
    color: var(--on-surface-variant);
  }

  .state-line.error {
    color: #e53935;
  }

  .empty-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--sp-md);
    padding: var(--sp-xl);
    background: var(--surface-container-low);
    border: 1px dashed var(--outline-variant);
    border-radius: var(--radius-lg);
    text-align: center;
    color: var(--on-surface-variant);
  }

  .empty-card h2 {
    margin: 0;
    color: var(--on-surface);
    font-size: 1.1rem;
  }

  .empty-card p {
    margin: 0;
    max-width: 50ch;
  }

  .empty-card .material-symbols-outlined {
    font-size: 48px;
    color: var(--primary);
  }

  /*
   * CR-08 §15: the inline recipe-card markup + its
   * .recipe-grid shell moved to RecipeLibrary.svelte
   * (5-tab layout). The .recipe-card-* and .card-*
   * styles below are dead and have been removed.
   */

  .header-actions {
    display: flex;
    gap: var(--sp-sm);
    align-items: center;
  }

  .secondary-cta {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    background: var(--surface-container-high);
    color: var(--on-surface);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-md);
    padding: 8px 14px;
    cursor: pointer;
    font-size: 0.85rem;
  }

  .secondary-cta:hover:not(:disabled) {
    background: var(--primary-container);
    border-color: var(--primary);
  }

  .secondary-cta:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .state-line.status {
    background: var(--primary-container);
    border: 1px solid var(--primary);
    border-radius: var(--radius-sm);
    padding: 8px 12px;
    color: var(--on-primary-container);
  }

  /*
   * CR-08 §10 Beginner tier modal styles. Mirrors the
   * modal-backdrop + modal-panel pattern from the
   * ProfileManager modal so the user gets a consistent
   * modal UX across tiers.
   */
  .beginner-modal {
    position: fixed;
    inset: 0;
    z-index: 1000;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--sp-md);
  }

  .beginner-modal-backdrop {
    position: absolute;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    border: 0;
    padding: 0;
    cursor: pointer;
  }

  .beginner-modal-backdrop:focus {
    outline: 2px solid var(--primary);
    outline-offset: -2px;
  }

  .beginner-modal-panel {
    position: relative;
    background: var(--surface-container);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-lg);
    padding: var(--sp-lg);
    width: min(640px, 100%);
    max-height: 90vh;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: var(--sp-md);
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sp-sm);
  }

  .modal-header h2 {
    margin: 0;
    font-size: 1.15rem;
  }

  .modal-close {
    background: transparent;
    color: var(--on-surface-variant);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-md);
    padding: var(--sp-xs) var(--sp-sm);
    cursor: pointer;
    font-size: 0.85rem;
  }

  .modal-close:hover {
    color: var(--on-surface);
    border-color: var(--primary);
  }

  .modal-footer {
    display: flex;
    flex-direction: column;
    gap: var(--sp-sm);
    padding-top: var(--sp-sm);
    border-top: 1px solid var(--outline-variant);
  }

  .modal-footer-note {
    margin: 0;
    color: var(--on-surface-variant);
    font-size: 0.85rem;
  }

  .modal-save {
    align-self: flex-end;
    background: var(--primary);
    color: var(--on-primary);
    border: none;
    border-radius: var(--radius-md);
    padding: var(--sp-sm) var(--sp-md);
    cursor: pointer;
    font-size: 0.9rem;
  }

  .modal-save:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
