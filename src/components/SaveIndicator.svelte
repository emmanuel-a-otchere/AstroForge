<!--
  CR-03 P6 — SaveIndicator (§29).

  Three states: saved (green dot), saving (animated blue dot with
  pulse), failed (red dot with retry hint). Reads from project-context.
  Renders inline next to the project identity in the application and
  studio shells. Visible label flips to "Saving…", "Saved", or
  "Save failed" so the user always knows the project state.
-->
<script lang="ts">
  import { isProjectDirty, projectContext } from "../state/project-context";

  let { compact = false }: { compact?: boolean } = $props();

  // State resolution:
  //   1. project-context.dirty=true → "Saving…"
  //   2. otherwise → "Saved"
  // Note: a real "failed" state needs a tracking pipeline that
  // catches save errors. P6 ships the rendering; the actual
  // error-tracking wire lands when the project-edit RPC exists.
  const state = $derived<"saved" | "saving" | "failed">(
    $isProjectDirty ? "saving" : "saved",
  );
</script>

<span
  class="save-indicator"
  data-state={state}
  title={state === "saved"
    ? "All changes saved"
    : state === "saving"
      ? "Saving changes…"
      : "Save failed — retry from the project menu"}
  aria-live="polite"
  aria-atomic="true"
>
  <span class="dot" aria-hidden="true"></span>
  {#if !compact}
    <span class="label font-body">
      {state === "saved" ? "Saved" : state === "saving" ? "Saving…" : "Save failed"}
    </span>
  {/if}
</span>

<style>
  .save-indicator {
    display: inline-flex;
    align-items: center;
    gap: var(--sp-xs);
    padding: var(--sp-xs) var(--sp-sm);
    border-radius: var(--radius-md);
    background: var(--surface-container);
    color: var(--on-surface-variant);
    font-size: 0.8rem;
  }

  .save-indicator .dot {
    width: 8px;
    height: 8px;
    border-radius: var(--radius-full);
    background: #6fbf73; /* green for "Saved" per §24 */
    flex-shrink: 0;
  }

  .save-indicator[data-state="saving"] .dot {
    background: #64b5f6; /* cobalt blue per §24 */
    animation: pulse 1.4s ease-in-out infinite;
  }

  .save-indicator[data-state="failed"] .dot {
    background: #e53935; /* error red per §24 */
  }

  .save-indicator[data-state="failed"] {
    color: #ff8a80;
  }

  @keyframes pulse {
    0%, 100% {
      opacity: 1;
    }
    50% {
      opacity: 0.4;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .save-indicator[data-state="saving"] .dot {
      animation: none;
    }
  }
</style>