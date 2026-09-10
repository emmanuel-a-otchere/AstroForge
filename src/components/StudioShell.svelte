<!--
  CR-03 P3 — Studio shell (§10).

  Layered overlay that renders on top of the ApplicationShell when a
  project is open. Composed of:
    - StudioHeader: project identity + save indicator + close-project
    - StudioTabRail: nav tabs for the 6 studio workspaces
    - StudioWorkspace: the currently-selected workspace screen
      (placeholder in P3; P4 fills in the real screens)
-->
<script lang="ts">
  import {
    studioViewport,
    STUDIO_NAV_ITEMS,
    type StudioView,
  } from "../state/application";
  import { closeProject as lifecycleCloseProject } from "../state/project-lifecycle";
  import SaveIndicator from "./SaveIndicator.svelte";
  import type { Snippet } from "svelte";

  let {
    overview,
    import_,
    process: processSnippet,
    enhance,
    compare,
    export_,
  }: {
    overview?: Snippet;
    import_?: Snippet;
    process?: Snippet;
    enhance?: Snippet;
    compare?: Snippet;
    export_?: Snippet;
  } = $props();

  function snippetFor(view: StudioView): Snippet | undefined {
    switch (view) {
      case "overview": return overview;
      case "import": return import_;
      case "process": return processSnippet;
      case "enhance": return enhance;
      case "compare": return compare;
      case "export": return export_;
    }
  }

  function setView(view: StudioView) {
    studioViewport.setView(view);
  }

  // R4: the lifecycle module is the single source of truth for
  // the close transition. It resets every project-scoped
  // store (workspace, versions, plan store) so reopening
  // starts from a clean slate.
  function closeProject() {
    lifecycleCloseProject();
  }
</script>

{#if $studioViewport.project}
  <div class="studio" data-studio-root="true">
    <header class="studio-header">
      <div class="header-left">
        <span class="material-symbols-outlined header-icon" aria-hidden="true">
          collections
        </span>
        <div class="project-identity">
          <span class="project-name font-display">{$studioViewport.project.name}</span>
          <span class="project-context font-body">Studio workspace</span>
        </div>
      </div>
      <div class="header-actions">
        <SaveIndicator />
        <button
          type="button"
          class="close-cta font-display"
          onclick={closeProject}
          aria-label="Close project"
        >
          <span class="material-symbols-outlined" aria-hidden="true">close</span>
          Close project
        </button>
      </div>
    </header>

    <nav class="tab-rail" aria-label="Studio workspaces">
      <ul role="tablist">
        {#each STUDIO_NAV_ITEMS as item (item.id)}
          <li role="presentation">
            <button
              type="button"
              role="tab"
              class="tab"
              aria-selected={$studioViewport.view === item.id}
              data-active={$studioViewport.view === item.id}
              onclick={() => setView(item.id)}
            >
              <span class="material-symbols-outlined tab-icon" aria-hidden="true">
                {item.icon}
              </span>
              <span class="tab-label font-body">{item.label}</span>
            </button>
          </li>
        {/each}
      </ul>
    </nav>

    <div class="studio-workspace" role="tabpanel" aria-label="{$studioViewport.view} workspace">
      {#if snippetFor($studioViewport.view)}
        {@render snippetFor($studioViewport.view)!()}
      {:else}
        <section class="placeholder-screen font-body" aria-live="polite">
          <p>The {$studioViewport.view} workspace ships in a later phase.</p>
          <p class="placeholder-hint">
            P3 ships the Studio shell + Overview + persistent project context.
            Import / Process / Enhance / Compare / Export land in P4.
          </p>
        </section>
      {/if}
    </div>
  </div>
{/if}

<style>
  .studio {
    display: grid;
    grid-template-rows: auto auto 1fr;
    height: 100%;
    width: 100%;
    background: var(--surface);
    color: var(--on-surface);
  }

  .studio-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--sp-md) var(--sp-lg);
    border-bottom: 1px solid var(--outline-variant);
    background: var(--surface-container-low);
    gap: var(--sp-md);
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: var(--sp-sm);
    min-width: 0;
  }

  .header-icon {
    font-size: 28px;
    color: var(--primary);
  }

  .project-identity {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .project-name {
    font-size: 1.1rem;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .project-context {
    font-size: 0.8rem;
    color: var(--on-surface-variant);
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: var(--sp-md);
  }

  /* SaveIndicator handles its own styling (see SaveIndicator.svelte). */

  .close-cta {
    display: inline-flex;
    align-items: center;
    gap: var(--sp-xs);
    background: transparent;
    border: 1px solid var(--outline-variant);
    color: var(--on-surface);
    padding: var(--sp-xs) var(--sp-md);
    border-radius: var(--radius-md);
    cursor: pointer;
    font-size: 0.85rem;
  }

  .close-cta:hover {
    background: var(--surface-container);
  }

  .close-cta .material-symbols-outlined {
    font-size: 18px;
  }

  .tab-rail {
    border-bottom: 1px solid var(--outline-variant);
    background: var(--surface-container-lowest);
  }

  .tab-rail ul {
    list-style: none;
    display: flex;
    padding: 0;
    margin: 0;
    gap: var(--sp-xs);
    padding: var(--sp-xs) var(--sp-md);
  }

  .tab {
    display: inline-flex;
    align-items: center;
    gap: var(--sp-xs);
    background: transparent;
    border: none;
    color: var(--on-surface-variant);
    padding: var(--sp-sm) var(--sp-md);
    border-radius: var(--radius-md);
    cursor: pointer;
    font-size: 0.9rem;
    position: relative;
  }

  .tab:hover {
    color: var(--on-surface);
    background: var(--surface-container-low);
  }

  .tab[data-active="true"] {
    color: var(--primary);
    background: var(--surface-container);
  }

  .tab[data-active="true"]::after {
    content: "";
    position: absolute;
    left: var(--sp-md);
    right: var(--sp-md);
    bottom: -1px;
    height: 2px;
    background: var(--primary);
    border-radius: 2px 2px 0 0;
  }

  .tab-icon {
    font-size: 18px;
  }

  .studio-workspace {
    overflow: auto;
    background: var(--surface);
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

  @media (max-width: 1024px) {
    .tab-rail ul {
      overflow-x: auto;
      flex-wrap: nowrap;
    }
    .tab {
      flex-shrink: 0;
    }
    .project-name {
      max-width: 30vw;
    }
  }
</style>