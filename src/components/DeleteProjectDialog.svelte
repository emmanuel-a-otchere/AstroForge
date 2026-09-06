<!--
  CR-03 P2 — Project delete dialog (§26 hard-delete confirm gate).

  Requires the user to type the project name verbatim before the
  confirm button enables. Matches the existing DestructiveConfirmDialog
  pattern (also gate-by-typed-name) so users get one consistent
  destructive-action UX across the app.
-->
<script lang="ts">
  import type { ProjectSummary } from "../lib/astroforge-api";

  let {
    project,
    open,
    onConfirm,
    onCancel,
  }: {
    project: ProjectSummary | null;
    open: boolean;
    onConfirm: () => void;
    onCancel: () => void;
  } = $props();

  let typed = $state("");
  let submitting = $state(false);

  $effect(() => {
    if (open) {
      typed = "";
      submitting = false;
    }
  });

  function handleConfirm() {
    if (!project) return;
    if (typed.trim() !== project.name.trim()) return;
    submitting = true;
    onConfirm();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") onCancel();
  }

  const confirmEnabled = $derived(
    !!project && typed.trim() === project.name.trim() && !submitting,
  );
</script>

{#if open && project}
  <div
    class="dialog-backdrop"
    onclick={onCancel}
    onkeydown={handleKeydown}
    role="presentation"
  >
    <div
      class="dialog"
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="delete-title"
      aria-describedby="delete-desc"
      onclick={(e) => e.stopPropagation()}
      onkeydown={handleKeydown}
      tabindex="-1"
    >
      <header class="dialog-header">
        <h2 id="delete-title" class="font-display">Delete project</h2>
      </header>
      <div class="dialog-body">
        <p id="delete-desc" class="font-body">
          This permanently removes the project, including its sessions, source
          assets, pipeline runs, image versions, and any exports. Source data
          on disk under the project directory is also deleted.
        </p>
        <p class="font-body">
          Type <span class="project-name font-label">{project.name}</span>
          to confirm.
        </p>
        <input
          type="text"
          class="confirm-input"
          bind:value={typed}
          placeholder={project.name}
          disabled={submitting}
          autocomplete="off"
          autofocus
          aria-label="Type the project name to confirm"
        />
      </div>
      <footer class="dialog-footer">
        <button
          type="button"
          class="secondary-cta font-display"
          onclick={onCancel}
          disabled={submitting}
        >
          Cancel
        </button>
        <button
          type="button"
          class="danger-cta font-display"
          onclick={handleConfirm}
          disabled={!confirmEnabled}
        >
          Delete project
        </button>
      </footer>
    </div>
  </div>
{/if}

<style>
  .dialog-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }

  .dialog {
    background: var(--surface-container);
    border: 1px solid #e53935;
    border-radius: var(--radius-lg);
    min-width: 420px;
    max-width: 560px;
    width: 90%;
    box-shadow: 0 16px 48px rgba(0, 0, 0, 0.5);
    color: var(--on-surface);
  }

  .dialog-header {
    padding: var(--sp-lg) var(--sp-lg) var(--sp-sm);
    border-bottom: 1px solid var(--outline-variant);
  }

  .dialog-header h2 {
    margin: 0;
    font-size: 1.25rem;
    color: #ff8a80;
  }

  .dialog-body {
    padding: var(--sp-lg);
    display: flex;
    flex-direction: column;
    gap: var(--sp-md);
  }

  .project-name {
    color: var(--on-surface);
    font-weight: 600;
  }

  .confirm-input {
    padding: var(--sp-sm) var(--sp-md);
    background: var(--surface-container-lowest);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-md);
    color: var(--on-surface);
    font-size: 1rem;
    font-family: inherit;
  }

  .confirm-input:focus {
    outline: none;
    border-color: var(--primary);
  }

  .dialog-footer {
    display: flex;
    justify-content: flex-end;
    gap: var(--sp-sm);
    padding: var(--sp-md) var(--sp-lg);
    border-top: 1px solid var(--outline-variant);
  }

  .secondary-cta {
    background: transparent;
    color: var(--on-surface-variant);
    border: 1px solid var(--outline-variant);
    padding: var(--sp-sm) var(--sp-lg);
    border-radius: var(--radius-md);
    cursor: pointer;
    font-size: 0.9rem;
  }

  .danger-cta {
    background: #e53935;
    color: #fff;
    border: none;
    padding: var(--sp-sm) var(--sp-lg);
    border-radius: var(--radius-md);
    cursor: pointer;
    font-size: 0.9rem;
    font-weight: 600;
  }

  .danger-cta:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
</style>