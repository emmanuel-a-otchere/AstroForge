<script lang="ts">
  // CR-10 P1.6.2.3 / Slice P1.6.2.3:
  // NodeGraphModeToggle. Renders a two-segment
  // toggle (Wizard / Node) in the top app header.
  // Writes to the `nodeGraphMode` store +
  // localStorage; reads the current mode back
  // reactively so the active segment is highlighted.
  //
  // The toggle is intentionally NOT a free-form
  // dropdown: spec §7.7 specifies the two-mode
  // pattern with a single click. P1.6.3 will add a
  // third option (Free-form layout) but it does not
  // expand the toggle into three segments until
  // then.

  import {
    nodeGraphMode,
    setNodeGraphMode,
    writePersistedNodeGraphMode,
    type NodeGraphMode,
  } from "../lib/node-graph-store";

  /** Resolve the browser's localStorage once at mount. */
  const storage: Storage | null =
    typeof window !== "undefined" && window.localStorage
      ? window.localStorage
      : null;

  function handleClick(mode: NodeGraphMode): void {
    setNodeGraphMode(mode);
    writePersistedNodeGraphMode(storage, mode);
  }

  const options: ReadonlyArray<{ id: NodeGraphMode; label: string; icon: string }> = [
    { id: "wizard", label: "Wizard", icon: "auto_awesome" },
    { id: "node", label: "Node", icon: "account_tree" },
  ];
</script>

<div
  class="node-graph-mode-toggle"
  role="group"
  aria-label="Pipeline mode"
  data-testid="node-graph-mode-toggle"
>
  {#each options as opt (opt.id)}
    <button
      type="button"
      class="seg"
      class:active={$nodeGraphMode === opt.id}
      aria-pressed={$nodeGraphMode === opt.id}
      data-mode={opt.id}
      onclick={() => handleClick(opt.id)}
      title={opt.label}
    >
      <span class="material-symbols-outlined seg-icon" aria-hidden="true">
        {opt.icon}
      </span>
      <span class="seg-label">{opt.label}</span>
    </button>
  {/each}
</div>

<style>
  .node-graph-mode-toggle {
    display: inline-flex;
    align-items: stretch;
    background: var(--surface-container-high, #2a2a2a);
    border: 1px solid var(--outline-variant, #444);
    border-radius: 999px;
    padding: 2px;
    gap: 0;
    box-sizing: border-box;
  }

  .seg {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 4px 10px;
    background: transparent;
    color: var(--on-surface-variant, #aaa);
    border: 0;
    border-radius: 999px;
    cursor: pointer;
    font: inherit;
    font-size: 12px;
    line-height: 1;
  }

  .seg:hover:not(.active) {
    background: var(--surface-container-highest, #333);
    color: var(--on-surface, #e0e0e0);
  }

  .seg.active {
    background: var(--primary, #4a9eff);
    color: var(--on-primary, #fff);
  }

  .seg-icon {
    font-size: 16px;
  }

  .seg-label {
    font-weight: 500;
  }

  @media (max-width: 640px) {
    .seg-label {
      display: none;
    }
    .seg {
      padding: 4px 6px;
    }
  }
</style>