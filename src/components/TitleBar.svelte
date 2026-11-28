<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { platform } from "@tauri-apps/plugin-os";

  const inTauri =
    typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

  let isMac = $state(false);
  let isMaximized = $state(false);

  onMount(() => {
    if (!inTauri) return;
    try {
      isMac = platform() === "macos";
    } catch {
      isMac = false;
    }
    void getCurrentWindow()
      .isMaximized()
      .then((v) => (isMaximized = v))
      .catch(() => {});
  });

  async function minimize() {
    if (!inTauri) return;
    await getCurrentWindow().minimize();
  }

  async function toggleMaximize() {
    if (!inTauri) return;
    const appWindow = getCurrentWindow();
    if (await appWindow.isMaximized()) {
      await appWindow.unmaximize();
    } else {
      await appWindow.maximize();
    }
    isMaximized = await appWindow.isMaximized();
  }

  async function closeWindow() {
    if (!inTauri) return;
    await getCurrentWindow().close();
  }
</script>

<div class="titlebar" class:mac={isMac} data-tauri-drag-region>
  <div class="titlebar-left" data-tauri-drag-region>
    <span class="titlebar-brand font-display" data-tauri-drag-region>AstroForge</span>
  </div>
  <div class="titlebar-center" data-tauri-drag-region></div>
  {#if inTauri && !isMac}
    <div
      class="titlebar-controls"
      onmousedown={(e) => e.stopPropagation()}
    >
      <button
        type="button"
        class="tb-btn material-symbols-outlined"
        aria-label="Minimize"
        onclick={minimize}>minimize</button
      >
      <button
        type="button"
        class="tb-btn material-symbols-outlined"
        aria-label={isMaximized ? "Restore" : "Maximize"}
        onclick={toggleMaximize}
        >{isMaximized
          ? "filter_none"
          : "check_box_outline_blank"}</button
      >
      <button
        type="button"
        class="tb-btn tb-close material-symbols-outlined"
        aria-label="Close"
        onclick={closeWindow}>close</button
      >
    </div>
  {/if}
</div>

<style>
  .titlebar {
    display: flex;
    align-items: stretch;
    height: 40px;
    flex: 0 0 auto;
    background: var(--surface-container-lowest);
    border-bottom: 1px solid var(--outline-variant);
    user-select: none;
  }

  .titlebar.mac {
    padding-left: 80px;
  }

  .titlebar-left {
    display: flex;
    align-items: center;
    padding: 0 var(--sp-md);
  }

  .titlebar-center {
    flex: 1 1 auto;
  }

  .titlebar-brand {
    font-size: 0.85rem;
    font-weight: 600;
    color: var(--on-surface-variant);
  }

  .titlebar-controls {
    display: flex;
    align-items: stretch;
  }

  .tb-btn {
    width: 46px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: none;
    background: transparent;
    color: var(--on-surface-variant);
    font-size: 18px;
    cursor: pointer;
  }

  .tb-btn:hover {
    background: var(--surface-container-high);
    color: var(--on-surface);
  }

  .tb-close:hover {
    background: #e53935;
    color: #fff;
  }
</style>
