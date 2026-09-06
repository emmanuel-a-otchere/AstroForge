<!--
  CR-03 P4 — Studio workspace content slots.

  P4 ships content for all 5 working tabs (Import, Process, Enhance,
  Compare, Export) plus the §8 Overview wiring. Each tab ships:
    - A purpose statement
    - The state-driven content body (placeholders; P5 fleshes them
      out, P4b wires real source-asset + recipe + export controls)
    - A consistent §26 empty state for first-time use

  Imports / recipes / AI / source-asset / export IPC all exist via
  the CR-02.6 scaffold; P4 wires reads, P4b wires writes.
-->
<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    title,
    description,
    icon,
    children,
  }: {
    title: string;
    description: string;
    icon: string;
    children: Snippet;
  } = $props();
</script>

<section class="workspace" aria-labelledby="workspace-title">
  <header class="workspace-header">
    <span class="material-symbols-outlined workspace-icon" aria-hidden="true">
      {icon}
    </span>
    <div>
      <h1 id="workspace-title" class="font-display">{title}</h1>
      <p class="workspace-description font-body">{description}</p>
    </div>
  </header>
  <div class="workspace-body">
    {@render children()}
  </div>
</section>

<style>
  .workspace {
    display: flex;
    flex-direction: column;
    gap: var(--sp-lg);
    padding: var(--sp-xl);
    max-width: 1100px;
    margin: 0 auto;
    width: 100%;
    box-sizing: border-box;
    color: var(--on-surface);
  }

  .workspace-header {
    display: flex;
    align-items: flex-start;
    gap: var(--sp-md);
  }

  .workspace-icon {
    font-size: 36px;
    color: var(--primary);
  }

  .workspace-header h1 {
    font-size: 1.75rem;
    margin: 0 0 var(--sp-xs) 0;
  }

  .workspace-description {
    margin: 0;
    color: var(--on-surface-variant);
    max-width: 70ch;
    font-size: 0.95rem;
  }

  .workspace-body {
    display: flex;
    flex-direction: column;
    gap: var(--sp-md);
  }
</style>