<!--
  CR-03 P1 — Home workspace (§6).

  Three sections:
    1. Primary actions (New Project / Open Project / Import Session / Resume Processing)
    2. Recent projects (empty in P1; populated by P2 from projectList())
    3. Welcome / orientation hint when no projects exist (§26 Empty States)

  P1 deliberately ships only the visual + behavior shell. The buttons
  route to placeholder modals; P2 wires the real create/open flow.
-->
<script lang="ts">
  import { applicationNavItem } from "../state/application";
  import type { ApplicationNavTarget } from "../state/application";

  type PrimaryAction = {
    id: "new-project" | "open-project" | "import-session" | "resume-processing";
    label: string;
    description: string;
    icon: string;
    primary: boolean;
  };

  const primaryActions: PrimaryAction[] = [
    {
      id: "new-project",
      label: "New Project",
      description: "Start a fresh astrophotography library from a target name.",
      icon: "add_circle",
      primary: true,
    },
    {
      id: "open-project",
      label: "Open Project",
      description: "Resume work in an existing project.",
      icon: "folder_open",
      primary: false,
    },
    {
      id: "import-session",
      label: "Import Session",
      description: "Bring raw telescope captures into a project.",
      icon: "file_download",
      primary: false,
    },
    {
      id: "resume-processing",
      label: "Resume Processing",
      description: "Pick up a pipeline where you left off.",
      icon: "play_arrow",
      primary: false,
    },
  ];

  function handle(action: PrimaryAction["id"]) {
    // P2 wires projectList() + projectCreate()/projectOpen() here.
    // P1 emits a CustomEvent so App.svelte can route the action through
    // its existing dialog layer without coupling the Home screen to a
    // concrete dialog component yet.
    const event = new CustomEvent("astroforge:home-action", {
      detail: { action },
      bubbles: true,
    });
    dispatchEvent(event);
  }

  function navTo(target: ApplicationNavTarget) {
    applicationNavItem(target); // future: actually navigate; P1 routes via store
  // Cast to unknown to satisfy strict unused-param handling until P3 lands
  void target;
  }
</script>

<section class="home-screen" aria-labelledby="home-title">
  <header class="home-header">
    <h1 id="home-title" class="font-display">Home</h1>
    <p class="home-subtitle font-body">
      What do you want to do? Pick a project to continue, or start a new one.
    </p>
  </header>

  <section class="primary-actions" aria-label="Primary actions">
    {#each primaryActions as action (action.id)}
      <button
        type="button"
        class="primary-action"
        class:primary={action.primary}
        onclick={() => handle(action.id)}
      >
        <span class="material-symbols-outlined icon" aria-hidden="true">
          {action.icon}
        </span>
        <span class="action-body">
          <span class="action-label font-display">{action.label}</span>
          <span class="action-description font-body">
            {action.description}
          </span>
        </span>
      </button>
    {/each}
  </section>

  <section class="recent-section" aria-label="Recent projects">
    <header class="recent-header">
      <h2 class="font-display">Recent Projects</h2>
      <button
        type="button"
        class="ghost-link font-body"
        onclick={() => navTo("projects")}
      >
        See all
      </button>
    </header>

    <!-- P1 ships the empty state per §26; P2 will replace this block with
         the populated grid driven by projectList(). -->
    <div class="empty-state">
      <span class="material-symbols-outlined empty-icon" aria-hidden="true">
        collections
      </span>
      <p class="empty-title font-display">Your astrophotography projects</p>
      <p class="empty-body font-body">
        Nothing here yet. Create your first project from a telescope capture
        or import an existing one.
      </p>
      <div class="empty-actions">
        <button
          type="button"
          class="primary-cta font-display"
          onclick={() => handle("new-project")}
        >
          Create Project
        </button>
        <button
          type="button"
          class="secondary-cta font-display"
          onclick={() => handle("open-project")}
        >
          Import Existing Project
        </button>
      </div>
    </div>
  </section>
</section>

<style>
  .home-screen {
    display: grid;
    grid-template-rows: auto 1fr auto;
    gap: var(--sp-xl);
    padding: var(--sp-xl);
    max-width: 1200px;
    margin: 0 auto;
    width: 100%;
    box-sizing: border-box;
    overflow-y: auto;
    color: var(--on-surface);
  }

  .home-header h1 {
    font-size: 2.5rem;
    margin: 0 0 var(--sp-sm) 0;
    color: var(--on-surface);
  }

  .home-subtitle {
    font-size: 1rem;
    margin: 0;
    color: var(--on-surface-variant);
    max-width: 60ch;
  }

  .primary-actions {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
    gap: var(--sp-md);
  }

  .primary-action {
    display: flex;
    align-items: center;
    gap: var(--sp-md);
    padding: var(--sp-md) var(--sp-lg);
    background: var(--surface-container-low);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-lg);
    color: var(--on-surface);
    cursor: pointer;
    text-align: left;
    transition: border-color 0.15s ease, background 0.15s ease;
  }

  .primary-action:hover {
    border-color: var(--outline);
    background: var(--surface-container);
  }

  .primary-action.primary {
    border-color: var(--primary);
    background: var(--surface-container);
  }

  .icon {
    font-size: 32px;
    color: var(--primary);
    flex: 0 0 auto;
  }

  .action-body {
    display: flex;
    flex-direction: column;
    gap: var(--sp-xs);
    min-width: 0;
  }

  .action-label {
    font-size: 1.1rem;
    font-weight: 600;
  }

  .action-description {
    font-size: 0.9rem;
    color: var(--on-surface-variant);
    line-height: 1.35;
  }

  .recent-section {
    display: flex;
    flex-direction: column;
    gap: var(--sp-md);
  }

  .recent-header {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
  }

  .recent-header h2 {
    font-size: 1.5rem;
    margin: 0;
    color: var(--on-surface);
  }

  .ghost-link {
    background: none;
    border: none;
    color: var(--primary);
    cursor: pointer;
    font-size: 0.95rem;
    padding: var(--sp-xs) var(--sp-sm);
  }

  .ghost-link:hover {
    text-decoration: underline;
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
    line-height: 1.4;
  }

  .empty-actions {
    display: flex;
    gap: var(--sp-sm);
    margin-top: var(--sp-sm);
  }

  .primary-cta {
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

  .secondary-cta {
    background: transparent;
    color: var(--primary);
    border: 1px solid var(--outline);
    padding: var(--sp-sm) var(--sp-lg);
    border-radius: var(--radius-md);
    cursor: pointer;
    font-size: 0.95rem;
  }

  .secondary-cta:hover {
    background: var(--surface-container);
  }

  @media (max-width: 1024px) {
    .home-screen {
      padding: var(--sp-lg);
    }
    .home-header h1 {
      font-size: 2rem;
    }
  }
</style>