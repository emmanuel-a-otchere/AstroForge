<!--
  App.svelte — application + studio orchestrator.

  CR-03 P4b: deprecates the wizard (AppShell + ModeA/B/C/D +
  WizardBottomSheet + CanvasBackdrop + ManifestReview +
  PreviewCanvas + layout-mode + preview-store). The Application
  shell is now the only shell. Opening a project enters Studio
  context; closing returns to Application context.

  Renders:
    - ApplicationShell wrapping the active view
      - Home (when no project open, nav=home)
      - Projects (when no project open, nav=projects)
      - StudioShell (when a project is open)
    - Dialog layer (create / rename / delete / error toast)

  Svelte 5 + SvelteKit-style snippet pattern. Application +
  Studio state lives in src/state/.
-->
<script lang="ts">
  import ApplicationShell from "./components/ApplicationShell.svelte";
  import HomeScreen from "./components/HomeScreen.svelte";
  import ProjectsScreen from "./components/ProjectsScreen.svelte";
  import ProjectDialog from "./components/ProjectDialog.svelte";
  import DeleteProjectDialog from "./components/DeleteProjectDialog.svelte";
  import StudioShell from "./components/StudioShell.svelte";
  import OverviewWorkspace from "./components/OverviewWorkspace.svelte";
  import ImportWorkspace from "./components/ImportWorkspace.svelte";
  import ProcessWorkspace from "./components/ProcessWorkspace.svelte";
  import EnhanceWorkspace from "./components/EnhanceWorkspace.svelte";
  import CompareWorkspace from "./components/CompareWorkspace.svelte";
  import ExportWorkspace from "./components/ExportWorkspace.svelte";
  // CR-05 R1 — application-level destinations that previously
  // rendered a generic placeholder route. Each ships with real
  // content in this slice (Recipes, AI Models, Settings, Help).
  import RecipesScreen from "./components/RecipesScreen.svelte";
  import AiModelsScreen from "./components/AiModelsScreen.svelte";
  import SettingsScreen from "./components/SettingsScreen.svelte";
  import HelpScreen from "./components/HelpScreen.svelte";
  import { applicationNavTarget, studioViewport } from "./state/application";
  import {
    closeProject as lifecycleCloseProject,
    openProject as lifecycleOpenProject,
  } from "./state/project-lifecycle";
  import { dialogOpen as dialogOpenStore, deleteDialogOpen as deleteDialogOpenStore } from "./state/dialog-state";
  import { useKeyboardShortcuts } from "./state/keyboard-shortcuts";
  import { projectsStore } from "./state/projects";
  import type { ProjectAction } from "./state/projects";
  import type { ProjectSummary } from "./lib/astroforge-api";
  import * as api from "./lib/astroforge-api";
  import { fly } from "svelte/transition";

  // Project dialog state for create + delete gates.
  let dialogOpen = $state(false);
  let dialogMode = $state<"create" | "rename">("create");
  let dialogProject: ProjectSummary | null = $state(null);
  let deleteDialogOpen = $state(false);
  let deleteTarget: ProjectSummary | null = $state(null);
  let dialogError = $state<string | null>(null);

  // Mirror the local dialog state into the cross-component stores
  // so the keyboard-shortcut handler can read whether a dialog
  // is currently open (and let the dialog handle its own Escape).
  $effect(() => {
    dialogOpenStore.set(dialogOpen);
  });
  $effect(() => {
    deleteDialogOpenStore.set(deleteDialogOpen);
  });

  // Wire global keyboard shortcuts (Cmd/Ctrl+N/O/S/1-6, Escape).
  useKeyboardShortcuts({
    onCreateProject: () => {
      dialogMode = "create";
      dialogProject = null;
      dialogOpen = true;
    },
  });

  function handleProjectAction(action: ProjectAction, project?: ProjectSummary) {
    dialogError = null;
    if (action === "create") {
      dialogMode = "create";
      dialogProject = null;
      dialogOpen = true;
      return;
    }
    if (action === "rename" && project) {
      dialogMode = "rename";
      dialogProject = project;
      dialogOpen = true;
      return;
    }
    if (action === "delete" && project) {
      deleteTarget = project;
      deleteDialogOpen = true;
      return;
    }
    if (action === "archive" && project) {
      void api.projectArchive(slugFromName(project.name))
        .then(() => projectsStore.refresh())
        .catch((e) => (dialogError = e instanceof Error ? e.message : String(e)));
      return;
    }
    if (action === "open" && project) {
      // R4: the lifecycle module is the single source of
      // truth for the open transition. It resets every
      // project-scoped store before loading the new
      // project's data so stale state can't leak between
      // projects.
      void lifecycleOpenProject(project);
      return;
    }
  }

  async function handleDialogSubmit(value: { name: string }) {
    try {
      if (dialogMode === "create") {
        await api.projectCreate({
          name: value.name,
          applicationVersion: "0.1.0",
        });
      } else if (dialogMode === "rename" && dialogProject) {
        await api.projectRename(slugFromName(dialogProject.name), value.name);
      }
      dialogOpen = false;
      await projectsStore.refresh();
    } catch (e) {
      dialogError = e instanceof Error ? e.message : String(e);
    }
  }

  async function handleDeleteConfirm() {
    if (!deleteTarget) return;
    const slug = slugFromName(deleteTarget.name);
    try {
      await api.projectDelete(slug);
      deleteDialogOpen = false;
      deleteTarget = null;
      await projectsStore.refresh();
    } catch (e) {
      dialogError = e instanceof Error ? e.message : String(e);
      deleteDialogOpen = false;
    }
  }

  function slugFromName(name: string): string {
    return name
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-|-$/g, "");
  }
</script>

<ApplicationShell projectLabel={$studioViewport.project?.name ?? "No project open"}>
  {#if $studioViewport.project}
  <StudioShell>
    {#snippet overview()}
      <OverviewWorkspace />
    {/snippet}
    {#snippet import_()}
      <ImportWorkspace />
    {/snippet}
    {#snippet process()}
      <ProcessWorkspace />
    {/snippet}
    {#snippet enhance()}
      <EnhanceWorkspace />
    {/snippet}
    {#snippet compare()}
      <CompareWorkspace />
    {/snippet}
    {#snippet export_()}
      <ExportWorkspace />
    {/snippet}
  </StudioShell>
  <div class="shortcut-strip" aria-label="Keyboard shortcuts">
    <span class="shortcut-hint font-label">
      <kbd>⌘N</kbd> New project · <kbd>⌘O</kbd> Open · <kbd>⌘S</kbd> Save ·
      <kbd>⌘1</kbd>–<kbd>⌘6</kbd> Switch tab · <kbd>Esc</kbd> Close project
    </span>
  </div>
  {:else if $applicationNavTarget === "home"}
    <HomeScreen />
  {:else if $applicationNavTarget === "projects"}
    <ProjectsScreen onAction={handleProjectAction} />
  {:else if $applicationNavTarget === "recipes"}
    <RecipesScreen />
  {:else if $applicationNavTarget === "ai-models"}
    <AiModelsScreen />
  {:else if $applicationNavTarget === "settings"}
    <SettingsScreen />
  {:else if $applicationNavTarget === "help"}
    <HelpScreen />
  {:else}
    <section class="placeholder-screen font-body" aria-live="polite">
      <p>This destination is not implemented.</p>
      <p class="placeholder-hint">
        Application screens in this build: Home, Projects, Recipes, AI
        Models, Settings, Help. The Studio overlay is active when a
        project is open.
      </p>
    </section>
  {/if}
</ApplicationShell>

{#if dialogOpen}
  <ProjectDialog
    mode={dialogMode}
    project={dialogProject ?? undefined}
    open={dialogOpen}
    onSubmit={handleDialogSubmit}
    onCancel={() => {
      dialogOpen = false;
      dialogError = null;
    }}
  />
{/if}

{#if deleteDialogOpen && deleteTarget}
  <DeleteProjectDialog
    project={deleteTarget}
    open={deleteDialogOpen}
    onConfirm={handleDeleteConfirm}
    onCancel={() => {
      deleteDialogOpen = false;
      deleteTarget = null;
    }}
  />
{/if}

{#if dialogError}
  <div
    class="action-error"
    role="alert"
    transition:fly={{ y: 20, duration: 200 }}
  >
    <span class="material-symbols-outlined" aria-hidden="true">error</span>
    <span>{dialogError}</span>
  </div>
{/if}

<style>
  .action-error {
    position: fixed;
    bottom: var(--sp-xl);
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: var(--sp-sm);
    padding: var(--sp-sm) var(--sp-lg);
    background: var(--surface-container-highest);
    border: 1px solid #e53935;
    border-radius: var(--radius-lg);
    color: var(--on-surface);
    z-index: 1000;
  }

  .action-error .material-symbols-outlined {
    color: #ff8a80;
  }

  .placeholder-screen {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    padding: var(--sp-xl);
    gap: var(--sp-md);
    color: var(--on-surface-variant);
  }

  .placeholder-hint {
    color: var(--on-surface-variant);
    font-size: 0.85rem;
    max-width: 60ch;
  }

  .shortcut-strip {
    position: fixed;
    bottom: var(--sp-sm);
    left: 50%;
    transform: translateX(-50%);
    padding: var(--sp-xs) var(--sp-md);
    background: var(--surface-container-highest);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-full);
    z-index: 50;
    pointer-events: none;
  }

  .shortcut-hint {
    font-size: 0.75rem;
    color: var(--on-surface-variant);
  }

  .shortcut-strip kbd {
    display: inline-block;
    padding: 1px 6px;
    background: var(--surface-container);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-sm);
    font-family: var(--font-data, monospace);
    font-size: 0.7rem;
    margin: 0 2px;
  }
</style>