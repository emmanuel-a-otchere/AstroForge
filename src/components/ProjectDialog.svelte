<!--
  CR-03 P2 — Project create / rename dialog.

  Mode-driven: emits 'submit' with { name } on confirm, 'cancel' on
  dismiss. Validation: non-empty name (whitespace trimmed). Submit is
  disabled while invalid. The §26 hard-delete gate is NOT here; it lives
  in DeleteProjectDialog.
-->
<script lang="ts">
  import type { ProjectSummary } from "../lib/astroforge-api";

  let {
    mode,
    project,
    open,
    onSubmit,
    onCancel,
  }: {
    mode: "create" | "rename";
    project?: ProjectSummary;
    open: boolean;
    onSubmit: (value: { name: string }) => void;
    onCancel: () => void;
  } = $props();

  let name = $state("");
  let error = $state<string | null>(null);
  let submitting = $state(false);

  // Reset state when the dialog opens. The `mode` and `project` props
  // are captured here so the dialog is consistent for the duration of
  // the open state; closing + reopening re-applies them.
  $effect(() => {
    if (open) {
      name = mode === "rename" && project ? project.name : "";
      error = null;
      submitting = false;
    }
  });

  function handleSubmit() {
    const trimmed = name.trim();
    if (!trimmed) {
      error = "Name cannot be empty.";
      return;
    }
    submitting = true;
    onSubmit({ name: trimmed });
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") onCancel();
    if (e.key === "Enter" && !submitting) handleSubmit();
  }

  const title = $derived(mode === "create" ? "Create project" : "Rename project");
  const submitLabel = $derived(mode === "create" ? "Create" : "Save");
</script>

{#if open}
  <div
    class="dialog-backdrop"
    onclick={onCancel}
    onkeydown={handleKeydown}
    role="presentation"
  >
    <div
      class="dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="project-dialog-title"
      onclick={(e) => e.stopPropagation()}
      onkeydown={handleKeydown}
      tabindex="-1"
    >
      <header class="dialog-header">
        <h2 id="project-dialog-title" class="font-display">{title}</h2>
      </header>
      <div class="dialog-body">
        <label class="field font-body" for="project-name-input">
          Project name
        </label>
        <input
          id="project-name-input"
          type="text"
          class="name-input"
          bind:value={name}
          placeholder={mode === "create" ? "e.g. M42 — Orion Nebula" : ""}
          disabled={submitting}
          autocomplete="off"
          autofocus
        />
        {#if error}
          <p class="field-error font-body" role="alert">{error}</p>
        {/if}
        <p class="hint font-body">
          The name is shown in the project header and in your library. You can
          rename the project at any time.
        </p>
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
          class="primary-cta font-display"
          onclick={handleSubmit}
          disabled={submitting || !name.trim()}
        >
          {submitLabel}
        </button>
      </footer>
    </div>
  </div>
{/if}

<style>
  .dialog-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }

  .dialog {
    background: var(--surface-container);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-lg);
    min-width: 380px;
    max-width: 520px;
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
  }

  .dialog-body {
    padding: var(--sp-lg);
    display: flex;
    flex-direction: column;
    gap: var(--sp-sm);
  }

  .field {
    font-size: 0.85rem;
    color: var(--on-surface-variant);
  }

  .name-input {
    padding: var(--sp-sm) var(--sp-md);
    background: var(--surface-container-lowest);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-md);
    color: var(--on-surface);
    font-size: 1rem;
    font-family: inherit;
  }

  .name-input:focus {
    outline: none;
    border-color: var(--primary);
  }

  .field-error {
    margin: 0;
    font-size: 0.85rem;
    color: #ff8a80;
  }

  .hint {
    margin: 0;
    font-size: 0.8rem;
    color: var(--on-surface-variant);
  }

  .dialog-footer {
    display: flex;
    justify-content: flex-end;
    gap: var(--sp-sm);
    padding: var(--sp-md) var(--sp-lg);
    border-top: 1px solid var(--outline-variant);
  }

  .primary-cta {
    background: var(--primary);
    color: var(--on-primary);
    border: none;
    padding: var(--sp-sm) var(--sp-lg);
    border-radius: var(--radius-md);
    cursor: pointer;
    font-size: 0.9rem;
    font-weight: 600;
  }

  .primary-cta:disabled {
    opacity: 0.4;
    cursor: not-allowed;
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

  .secondary-cta:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
</style>