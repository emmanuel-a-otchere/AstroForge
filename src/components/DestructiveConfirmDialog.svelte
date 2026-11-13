<!--
  CR-10 P1.6.3.2 — destructive-stage confirm gate.

  Fires when a graph mutation targets a DESTRUCTIVE stage (per the
  audit decision Q-3 convention: destructive ops warn before pixels
  move). Click-confirm (no typed gate — graph mutations are
  recoverable from the plan history, unlike project deletion).
  Matches DeleteProjectDialog's backdrop/Escape/a11y structure so
  destructive UX stays consistent across the app.
-->
<script lang="ts">
  let {
    open,
    title,
    body,
    confirmLabel = "Confirm",
    onConfirm,
    onCancel,
  }: {
    open: boolean;
    title: string;
    body: string;
    confirmLabel?: string;
    onConfirm: () => void;
    onCancel: () => void;
  } = $props();

  let submitting = $state(false);

  $effect(() => {
    if (open) {
      submitting = false;
    }
  });

  function handleConfirm() {
    submitting = true;
    onConfirm();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") onCancel();
  }
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
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="destructive-title"
      aria-describedby="destructive-desc"
      onclick={(e) => e.stopPropagation()}
      onkeydown={handleKeydown}
      tabindex="-1"
    >
      <header class="dialog-header">
        <h2 id="destructive-title" class="font-display">{title}</h2>
      </header>
      <div class="dialog-body">
        <p id="destructive-desc" class="font-body">{body}</p>
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
          disabled={submitting}
        >
          {confirmLabel}
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
    border: 1px solid var(--error-container);
    border-radius: var(--radius-lg);
    min-width: 380px;
    max-width: 520px;
    width: 90%;
    box-shadow: var(--elevation-3);
    padding: var(--sp-lg);
  }

  .dialog-header h2 {
    margin: 0 0 var(--sp-sm) 0;
    font-size: 1.25rem;
    color: var(--on-surface);
  }

  .dialog-body p {
    margin: 0 0 var(--sp-sm) 0;
    color: var(--on-surface-variant);
    line-height: var(--lh-body);
  }

  .dialog-footer {
    display: flex;
    justify-content: flex-end;
    gap: var(--sp-sm);
    margin-top: var(--sp-md);
  }

  .secondary-cta,
  .danger-cta {
    padding: 0.5rem 1rem;
    border-radius: var(--radius-default);
    cursor: pointer;
    font-size: 0.9rem;
    border: 1px solid transparent;
  }

  .secondary-cta {
    background: var(--surface-container-high);
    color: var(--on-surface);
    border-color: var(--outline-variant);
  }

  .secondary-cta:hover {
    background: var(--surface-container-highest);
  }

  .danger-cta {
    background: var(--error-container);
    color: var(--on-error-container);
  }

  .danger-cta:hover {
    background: var(--error);
    color: var(--on-error);
  }

  .secondary-cta:disabled,
  .danger-cta:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
</style>
