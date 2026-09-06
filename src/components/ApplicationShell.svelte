<!--
  CR-03 P1 — application shell (§4 + §5 + §27 responsive).

  Persistent across the application context. Composed of:
    - Header: brand + project identity slot (filled in P3)
    - Left navigation: Home / Projects / Recipes / AI Models / Settings / Help
    - Workspace area: a children snippet the parent provides

  Designed to coexist with the wizard pattern. P3 will introduce the Studio
  shell swap that takes over when a Project is open.

  Layout follows §5: navigation ~10-15%, workspace ~85-90%. Below 1024px
  the left nav collapses into a drawer.
-->
<script lang="ts">
  import type { Snippet } from "svelte";
  import {
    APPLICATION_NAV_ITEMS,
    applicationNavTarget,
    type ApplicationNavTarget,
  } from "../state/application";
  import SaveIndicator from "./SaveIndicator.svelte";

  let {
    children,
    /** Project identity slot for P3. P1 ships it as a fixed placeholder. */
    projectLabel = "No project open",
  }: {
    children?: Snippet;
    projectLabel?: string;
  } = $props();

  let navOpen = $state(true);

  function select(target: ApplicationNavTarget) {
    applicationNavTarget.set(target);
  }

  function toggleNav() {
    navOpen = !navOpen;
  }
</script>

<div class="application-shell" class:nav-collapsed={!navOpen}>
  <header class="app-header">
    <button
      type="button"
      class="nav-toggle material-symbols-outlined"
      aria-label={navOpen ? "Hide navigation" : "Show navigation"}
      onclick={toggleNav}
    >
      {navOpen ? "menu_open" : "menu"}
    </button>
    <span class="brand font-display">AstroForge</span>
    <span class="project-slot font-body" data-testid="project-slot">
      {projectLabel}
    </span>
    <span class="header-spacer"></span>
    <SaveIndicator />
  </header>

  <div class="app-body">
    {#if navOpen}
      <nav class="app-nav" aria-label="Application navigation">
        <ul class="nav-list">
          {#each APPLICATION_NAV_ITEMS as item (item.id)}
            {@const isActive = $applicationNavTarget === item.id}
            <li>
              <button
                type="button"
                class="nav-item"
                class:active={isActive}
                aria-current={isActive ? "page" : undefined}
                onclick={() => select(item.id)}
              >
                <span
                  class="material-symbols-outlined nav-icon"
                  aria-hidden="true"
                >
                  {item.icon}
                </span>
                <span class="nav-text">
                  <span class="nav-label font-display">{item.label}</span>
                  <span class="nav-hint font-body">{item.hint}</span>
                </span>
              </button>
            </li>
          {/each}
        </ul>
      </nav>
    {/if}

    <main class="app-workspace" aria-label="Workspace">
      {#if children}{@render children()}{/if}
    </main>
  </div>
</div>

<style>
  .application-shell {
    display: grid;
    grid-template-rows: auto 1fr;
    height: 100vh;
    background: var(--surface);
    color: var(--on-surface);
    font-family: var(--font-body);
  }

  .app-header {
    display: flex;
    align-items: center;
    gap: var(--sp-md);
    padding: 0 var(--sp-lg);
    height: 56px;
    background: var(--surface-container-lowest);
    border-bottom: 1px solid var(--outline-variant);
    flex: 0 0 auto;
  }

  .nav-toggle {
    background: transparent;
    border: none;
    color: var(--on-surface-variant);
    cursor: pointer;
    font-size: 24px;
    padding: var(--sp-xs);
    border-radius: var(--radius-sm);
  }

  .nav-toggle:hover {
    color: var(--on-surface);
    background: var(--surface-container);
  }

  .brand {
    font-size: 1.1rem;
    font-weight: 700;
    letter-spacing: 0.02em;
    color: var(--on-surface);
  }

  .project-slot {
    margin-left: var(--sp-md);
    padding: var(--sp-xs) var(--sp-md);
    border-left: 1px solid var(--outline-variant);
    color: var(--on-surface-variant);
    font-size: 0.9rem;
  }

  .header-spacer {
    flex: 1 1 auto;
  }

  /* SaveIndicator handles its own styling (see SaveIndicator.svelte). */

  .app-body {
    display: grid;
    grid-template-columns: 240px 1fr;
    min-height: 0;
    overflow: hidden;
  }

  .application-shell.nav-collapsed .app-body {
    grid-template-columns: 1fr;
  }

  .app-nav {
    background: var(--surface-container-low);
    border-right: 1px solid var(--outline-variant);
    overflow-y: auto;
    padding: var(--sp-sm) 0;
  }

  .nav-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .nav-item {
    display: flex;
    align-items: center;
    gap: var(--sp-sm);
    width: calc(100% - var(--sp-md));
    margin: 0 var(--sp-sm);
    padding: var(--sp-sm) var(--sp-md);
    background: transparent;
    border: none;
    border-radius: var(--radius-md);
    color: var(--on-surface-variant);
    cursor: pointer;
    text-align: left;
    transition: background 0.12s ease, color 0.12s ease;
  }

  .nav-item:hover {
    background: var(--surface-container);
    color: var(--on-surface);
  }

  .nav-item.active {
    background: var(--surface-container-high);
    color: var(--on-surface);
    border-left: 3px solid var(--primary);
    padding-left: calc(var(--sp-md) - 3px);
  }

  .nav-icon {
    font-size: 22px;
    flex: 0 0 auto;
  }

  .nav-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .nav-label {
    font-size: 0.95rem;
    font-weight: 600;
  }

  .nav-hint {
    font-size: 0.78rem;
    color: var(--on-surface-variant);
    opacity: 0.85;
    line-height: 1.2;
  }

  .app-workspace {
    background: var(--surface);
    overflow: hidden;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  @media (max-width: 1024px) {
    .app-body {
      grid-template-columns: 1fr;
    }
    .app-nav {
      position: absolute;
      top: 56px;
      left: 0;
      bottom: 0;
      width: 240px;
      z-index: 10;
      box-shadow: 2px 0 8px rgba(0, 0, 0, 0.4);
    }
    .project-slot {
      display: none;
    }
  }
</style>