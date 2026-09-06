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
  import { applicationNavTarget, studioViewport } from "./state/application";
  import { projectContext } from "./state/project-context";
  import { workspaceState } from "./state/workspace";
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
      // Opening a project enters Studio context.
      projectContext.open(project);
      studioViewport.openProject({
        project_id: project.project_id,
        name: project.name,
      });
      void workspaceState.load(project);
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
  {:else if $applicationNavTarget === "home"}
    <HomeScreen />
  {:else if $applicationNavTarget === "projects"}
    <ProjectsScreen onAction={handleProjectAction} />
  {:else}
    <section class="placeholder-screen font-body" aria-live="polite">
      <p>This screen is a placeholder in CR-03 P4b.</p>
      <p class="placeholder-hint">
        Available application screens in P4b: Home, Projects. Studio overlay
        is active when a project is open. Other screens (Recipes, AI
        Models, Settings, Help) ship in later phases.
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
</style>