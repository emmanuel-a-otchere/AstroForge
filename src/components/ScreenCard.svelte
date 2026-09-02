<!--
  ScreenCard — shared "screen page" container used across all four modes.

  Provides:
  - Glassmorphism panel (bg + 1px border + backdrop-filter blur)
  - Standardised header with title + label-caps kicker
  - Scrollable body (children snippet)
  - Optional footer snippet for persistent controls

  Uses Svelte 5 snippets for content injection.
-->
<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    title = null,
    kicker = null,
    width = "100%",
    maxWidth = null,
    variant = "default",
    children,
    footer,
  }: {
    title?: string | null;
    kicker?: string | null;
    width?: string;
    maxWidth?: string | null;
    variant?: "default" | "overlay";
    children?: Snippet;
    footer?: Snippet;
  } = $props();
</script>

<div
  class="screen-card"
  class:screen-card-overlay={variant === "overlay"}
  style="width: {width}; {maxWidth ? `max-width: ${maxWidth};` : ''}"
>
  {#if title || kicker}
    <header class="screen-card-header">
      <div class="screen-card-header-text">
        {#if kicker}
          <div class="screen-card-kicker label-caps">{kicker}</div>
        {/if}
        {#if title}
          <h3 class="screen-card-title">{title}</h3>
        {/if}
      </div>
    </header>
  {/if}

  <div class="screen-card-body">
    {#if children}
      {@render children()}
    {/if}
  </div>

  {#if footer}
    <footer class="screen-card-footer">
      {@render footer()}
    </footer>
  {/if}
</div>

<style>
  .screen-card {
    background-color: rgba(30, 32, 32, 0.85);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-default);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    color: var(--on-surface);
    font-family: var(--font-body);
    font-size: var(--text-body);
    line-height: var(--lh-body);
  }

  .screen-card-overlay {
    background-color: rgba(30, 32, 32, 0.92);
    box-shadow: 0 0 0 1px rgba(255, 180, 168, 0.04),
      0 12px 32px rgba(0, 0, 0, 0.45);
  }

  .screen-card-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sp-md);
    padding: var(--sp-md) var(--sp-lg);
    border-bottom: 1px solid var(--outline-variant);
    flex-shrink: 0;
  }

  .screen-card-header-text {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .screen-card-kicker {
    color: var(--primary);
    font-family: var(--font-data);
    font-size: var(--text-label);
    font-weight: 700;
    letter-spacing: var(--ls-label);
    text-transform: uppercase;
  }

  .screen-card-title {
    margin: 0;
    font-family: var(--font-display);
    font-size: var(--text-headline-mobile);
    font-weight: 600;
    line-height: var(--lh-headline);
    letter-spacing: var(--ls-headline);
    color: var(--on-surface);
  }

  .screen-card-body {
    padding: var(--sp-lg);
    overflow-y: auto;
    flex: 1;
    min-height: 0;
  }

  .screen-card-footer {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: var(--sp-sm);
    padding: var(--sp-md) var(--sp-lg);
    border-top: 1px solid var(--outline-variant);
    flex-shrink: 0;
  }
</style>