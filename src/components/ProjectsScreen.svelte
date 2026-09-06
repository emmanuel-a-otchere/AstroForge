<!--
  CR-03 P2 — Projects workspace (§7).

  Renders the project grid driven by projectsStore (which wraps the
  CR-02.6 IPC client). Each card exposes:
    - thumbnail (P2 placeholder; P5 will derive from latest image version)
    - target
    - target type (deep_sky | planetary | lunar)
    - session count (P2 placeholder; derived in P3 from real sessions)
    - processing state (P2 placeholder; derived in P3 from pipeline runs)
    - last modified
    - latest image version (P2 placeholder; P5 derives)

  §26 empty state: "Your astrophotography projects — Nothing here yet."
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { projectsStore, isProjectsEmpty } from "../state/projects";
  import type { ProjectSummary } from "../lib/astroforge-api";
  import type { ProjectAction } from "../state/projects";

  let { onAction }: { onAction?: (action: ProjectAction, project?: ProjectSummary) => void } = $props();

  onMount(() => {
    projectsStore.refresh();
  });

  function handleAction(action: ProjectAction, project?: ProjectSummary) {
    onAction?.(action, project);
  }

  function formatDate(iso: string): string {
    if (!iso) return "—";
    const d = new Date(iso);
    if (Number.isNaN(d.getTime())) return iso;
    return d.toLocaleDateString(undefined, {
      year: "numeric",
      month: "short",
      day: "numeric",
    });
  }
</script>

<section class="projects-screen" aria-labelledby="projects-title">
  <header class="projects-header">
    <div>
      <h1 id="projects-title" class="font-display">Projects</h1>
      <p class="projects-subtitle font-body">
        Your persistent astrophotography library.
      </p>
    </div>
    <button
      type="button"
      class="primary-cta font-display"
      onclick={() => handleAction("create")}
    >
      <span class="material-symbols-outlined cta-icon" aria-hidden="true">add</span>
      New Project
    </button>
  </header>

  {#if $projectsStore.loading}
    <p class="status-line font-body" role="status">Loading projects…</p>
  {:else if $projectsStore.error}
    <div class="error-state" role="alert">
      <span class="material-symbols-outlined" aria-hidden="true">error</span>
      <p class="font-body">Failed to load projects: {$projectsStore.error}</p>
      <button
        type="button"
        class="ghost-link font-body"
        onclick={() => projectsStore.refresh()}
      >
        Retry
      </button>
    </div>
  {:else if isProjectsEmpty($projectsStore)}
    <!-- §26 No Projects empty state -->
    <div class="empty-state">
      <span class="material-symbols-outlined empty-icon" aria-hidden="true">
        collections
      </span>
      <p class="empty-title font-display">Your astrophotography projects</p>
      <p class="empty-body font-body">
        Nothing here yet. Create your first project from a telescope capture.
      </p>
      <div class="empty-actions">
        <button
          type="button"
          class="primary-cta font-display"
          onclick={() => handleAction("create")}
        >
          Create Project
        </button>
      </div>
    </div>
  {:else}
    <ul class="project-grid" aria-label="Project list">
      {#each $projectsStore.items as project (project.project_id)}
        <li class="project-card">
          <button
            type="button"
            class="card-button"
            onclick={() => handleAction("open", project)}
            aria-label="Open project {project.name}"
          >
            <div class="thumb" aria-hidden="true">
              <span class="material-symbols-outlined thumb-icon">
                image
              </span>
            </div>
            <div class="card-body">
              <div class="card-title-row">
                <h2 class="font-display card-title">{project.name}</h2>
                <span class="status-pill" data-status={project.status}>
                  {project.status}
                </span>
              </div>
              <p class="card-meta font-body">
                {project.application_version} · Updated {formatDate(project.updated_at)}
              </p>
              <p class="card-meta font-body">
                Target: {project.active_session_id ? "active session linked" : "no session yet"}
              </p>
            </div>
          </button>
          <div class="card-actions">
            <button
              type="button"
              class="ghost-link font-body"
              onclick={() => handleAction("rename", project)}
            >
              Rename
            </button>
            <button
              type="button"
              class="ghost-link font-body"
              onclick={() => handleAction("archive", project)}
            >
              Archive
            </button>
            <button
              type="button"
              class="ghost-link danger font-body"
              onclick={() => handleAction("delete", project)}
            >
              Delete
            </button>
          </div>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .projects-screen {
    display: flex;
    flex-direction: column;
    gap: var(--sp-lg);
    padding: var(--sp-xl);
    max-width: 1200px;
    margin: 0 auto;
    width: 100%;
    box-sizing: border-box;
    overflow-y: auto;
    color: var(--on-surface);
  }

  .projects-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--sp-md);
  }

  .projects-header h1 {
    font-size: 2rem;
    margin: 0 0 var(--sp-xs) 0;
  }

  .projects-subtitle {
    margin: 0;
    color: var(--on-surface-variant);
    font-size: 0.95rem;
  }

  .primary-cta {
    display: inline-flex;
    align-items: center;
    gap: var(--sp-xs);
    background: var(--primary);
    color: var(--on-primary);
    border: none;
    padding: var(--sp-sm) var(--sp-lg);
    border-radius: var(--radius-md);
    cursor: pointer;
    font-size: 0.95rem;
    font-weight: 600;
  }

  .primary-cta:hover {
    background: var(--primary-container);
    color: var(--on-primary-container);
  }

  .cta-icon {
    font-size: 20px;
  }

  .status-line {
    color: var(--on-surface-variant);
    font-size: 0.9rem;
  }

  .error-state {
    display: flex;
    align-items: center;
    gap: var(--sp-sm);
    padding: var(--sp-md);
    background: var(--surface-container-low);
    border: 1px solid #e53935;
    border-radius: var(--radius-md);
    color: #ff8a80;
  }

  .ghost-link {
    background: none;
    border: none;
    color: var(--primary);
    cursor: pointer;
    font-size: 0.85rem;
    padding: var(--sp-xs) var(--sp-sm);
    margin-left: auto;
  }

  .ghost-link:hover {
    text-decoration: underline;
  }

  .ghost-link.danger {
    color: #ff8a80;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    padding: var(--sp-xl);
    background: var(--surface-container-low);
    border: 1px dashed var(--outline-variant);
    border-radius: var(--radius-lg);
    gap: var(--sp-md);
  }

  .empty-icon {
    font-size: 48px;
    color: var(--on-surface-variant);
  }

  .empty-title {
    font-size: 1.25rem;
    margin: 0;
    color: var(--on-surface);
  }

  .empty-body {
    font-size: 0.95rem;
    margin: 0;
    color: var(--on-surface-variant);
    max-width: 50ch;
  }

  .empty-actions {
    display: flex;
    gap: var(--sp-sm);
  }

  .project-grid {
    list-style: none;
    padding: 0;
    margin: 0;
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
    gap: var(--sp-md);
  }

  .project-card {
    display: flex;
    flex-direction: column;
    background: var(--surface-container-low);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-lg);
    overflow: hidden;
    transition: border-color 0.15s ease;
  }

  .project-card:hover {
    border-color: var(--outline);
  }

  .card-button {
    display: flex;
    flex-direction: column;
    gap: var(--sp-sm);
    background: transparent;
    border: none;
    color: var(--on-surface);
    cursor: pointer;
    text-align: left;
    padding: 0;
  }

  .thumb {
    aspect-ratio: 4 / 3;
    background: var(--surface-container);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .thumb-icon {
    font-size: 48px;
    color: var(--on-surface-variant);
    opacity: 0.6;
  }

  .card-body {
    padding: var(--sp-md);
    display: flex;
    flex-direction: column;
    gap: var(--sp-xs);
  }

  .card-title-row {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: var(--sp-sm);
  }

  .card-title {
    font-size: 1.1rem;
    font-weight: 600;
    margin: 0;
    line-height: 1.2;
  }

  .status-pill {
    font-size: 0.7rem;
    padding: 2px var(--sp-sm);
    border-radius: var(--radius-full);
    background: var(--surface-container-high);
    color: var(--on-surface-variant);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .status-pill[data-status="Active"] {
    background: rgba(111, 191, 115, 0.2);
    color: #6fbf73;
  }

  .status-pill[data-status="Archived"] {
    background: rgba(255, 179, 0, 0.2);
    color: #ffb300;
  }

  .status-pill[data-status="Exported"] {
    background: rgba(33, 150, 243, 0.2);
    color: #64b5f6;
  }

  .card-meta {
    font-size: 0.85rem;
    color: var(--on-surface-variant);
    margin: 0;
  }

  .card-actions {
    display: flex;
    gap: var(--sp-xs);
    padding: var(--sp-sm) var(--sp-md);
    border-top: 1px solid var(--outline-variant);
    flex-wrap: wrap;
  }
</style>