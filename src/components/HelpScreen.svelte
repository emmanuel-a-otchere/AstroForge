<!--
  HelpScreen — application-level Help destination.

  CR-05 R1: replaces the placeholder route for the "Help" nav
  target. Surfaces a structured help index: keyboard shortcuts
  (mirrored from `useKeyboardShortcuts`), in-app screens
  (Studio tabs + application targets), and links to the
  project specs.

  No backend, no IPC — Help is informational.
-->
<script lang="ts">
  import {
    APPLICATION_NAV_ITEMS,
    type ApplicationNavTarget,
  } from "../state/application";
  import { STUDIO_NAV_ITEMS, type StudioView } from "../state/application";

  const shortcuts: { keys: string; description: string }[] = [
    { keys: "⌘N", description: "New project" },
    { keys: "⌘O", description: "Open the Projects list" },
    { keys: "⌘S", description: "Mark the active project as saved" },
    { keys: "⌘1", description: "Open Overview (Studio)" },
    { keys: "⌘2", description: "Open Import (Studio)" },
    { keys: "⌘3", description: "Open Process (Studio)" },
    { keys: "⌘4", description: "Open Enhance (Studio)" },
    { keys: "⌘5", description: "Open Compare (Studio)" },
    { keys: "⌘6", description: "Open Export (Studio)" },
    { keys: "Esc", description: "Close the project or current dialog" },
  ];

  function studioLabel(view: StudioView): string {
    return STUDIO_NAV_ITEMS.find((i) => i.id === view)?.label ?? view;
  }

  function appLabel(target: ApplicationNavTarget): string {
    return APPLICATION_NAV_ITEMS.find((i) => i.id === target)?.label ?? target;
  }
</script>

<section class="help-screen" aria-label="Help">
  <header class="screen-header">
    <h1 class="font-display">Help</h1>
    <p class="font-body">
      Keyboard shortcuts, screen index, and project documentation.
    </p>
  </header>

  <div class="grid">
    <article class="card" aria-labelledby="help-shortcuts">
      <h2 id="help-shortcuts" class="font-display">Keyboard shortcuts</h2>
      <ul class="shortcut-list">
        {#each shortcuts as s (s.keys)}
          <li>
            <kbd>{s.keys}</kbd>
            <span class="font-body">{s.description}</span>
          </li>
        {/each}
      </ul>
    </article>

    <article class="card" aria-labelledby="help-screens">
      <h2 id="help-screens" class="font-display">In-app screens</h2>
      <div class="screen-groups">
        <div>
          <h3 class="font-display">Application</h3>
          <ul>
            {#each APPLICATION_NAV_ITEMS as item (item.id)}
              <li>
                <span class="material-symbols-outlined" aria-hidden="true">
                  {item.icon}
                </span>
                <span class="font-body">
                  <strong>{appLabel(item.id)}</strong>
                  <span class="muted">— {item.hint}</span>
                </span>
              </li>
            {/each}
          </ul>
        </div>
        <div>
          <h3 class="font-display">Studio (open a project first)</h3>
          <ul>
            {#each STUDIO_NAV_ITEMS as item (item.id)}
              <li>
                <span class="material-symbols-outlined" aria-hidden="true">
                  {item.icon}
                </span>
                <span class="font-body">
                  <strong>{studioLabel(item.id)}</strong>
                  <span class="muted">— ⌘{["overview", "import", "process", "enhance", "compare", "export"].indexOf(item.id) + 1}</span>
                </span>
              </li>
            {/each}
          </ul>
        </div>
      </div>
    </article>

    <article class="card" aria-labelledby="help-docs">
      <h2 id="help-docs" class="font-display">Documentation</h2>
      <p class="font-body muted">
        Project specs and audit docs live in the repository under
        <code>docs/</code>. The most relevant reads for a new user are:
      </p>
      <ul class="doc-list">
        <li><code>docs/CR-05-INTELLIGENT-PROCESSING.md</code> — full CR-05 spec</li>
        <li>
          <code>docs/plans/2026-09-07-cr05-intelligent-processing/PLAN.md</code> —
          implementation plan
        </li>
        <li><code>docs/UI_WORKFLOW_AUDIT.md</code> — current screen integration audit</li>
        <li><code>docs/M8_AUDIT.md</code> — P7 audit and acceptance status</li>
      </ul>
    </article>
  </div>
</section>

<style>
  .help-screen {
    display: flex;
    flex-direction: column;
    gap: var(--sp-lg);
    padding: var(--sp-xl);
    overflow-y: auto;
    height: 100%;
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

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(360px, 1fr));
    gap: var(--sp-lg);
  }

  .card {
    background: var(--surface-container);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-lg);
    padding: var(--sp-lg);
    display: flex;
    flex-direction: column;
    gap: var(--sp-md);
  }

  .card h2 {
    margin: 0;
    font-size: 1.05rem;
  }

  .card h3 {
    margin: var(--sp-sm) 0 var(--sp-xs) 0;
    font-size: 0.95rem;
    color: var(--on-surface-variant);
  }

  .shortcut-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: var(--sp-xs);
  }

  .shortcut-list li {
    display: flex;
    gap: var(--sp-md);
    align-items: center;
  }

  .screen-groups {
    display: flex;
    flex-direction: column;
    gap: var(--sp-md);
  }

  .screen-groups ul {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: var(--sp-xs);
  }

  .screen-groups li {
    display: flex;
    gap: var(--sp-sm);
    align-items: flex-start;
  }

  .screen-groups .material-symbols-outlined {
    color: var(--primary);
  }

  .muted {
    color: var(--on-surface-variant);
  }

  .doc-list {
    list-style: disc;
    padding-left: var(--sp-lg);
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: var(--sp-xs);
  }

  .doc-list code {
    font-family: var(--font-data, monospace);
    font-size: 0.85rem;
  }

  kbd {
    display: inline-block;
    padding: 2px 8px;
    background: var(--surface-container-highest);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-sm);
    font-family: var(--font-data, monospace);
    font-size: 0.8rem;
    min-width: 56px;
    text-align: center;
  }
</style>
