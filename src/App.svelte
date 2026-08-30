<script lang="ts">
  import { probeGpu, type GpuCapability } from "./lib/gpu";

  let gpuCapability: GpuCapability = "canvas2d";
  let gpuChecked = false;

  onMount(() => {
    gpuCapability = probeGpu();
    gpuChecked = true;
  });

  function onMount(fn: () => void) {
    if (document.readyState !== "loading") {
      fn();
    } else {
      document.addEventListener("DOMContentLoaded", fn, { once: true });
    }
  }
</script>

<main class="app">
  <header class="app-header">
    <div class="logo">
      <svg width="32" height="32" viewBox="0 0 64 64" fill="none">
        <circle cx="32" cy="32" r="20" stroke="currentColor" stroke-width="2.5" fill="none"/>
        <circle cx="32" cy="32" r="6" fill="currentColor"/>
        <path d="M32 6 L32 14 M32 50 L32 58 M6 32 L14 32 M50 32 L58 32" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
      </svg>
      <span class="app-name">AstroForge</span>
    </div>
    <div class="gpu-badge" class:gpu-checked={gpuChecked}>
      {#if gpuChecked}
        GPU: {gpuCapability === "webgpu" ? "WebGPU" : "Canvas2D"}
      {:else}
        Detecting GPU...
      {/if}
    </div>
  </header>

  <section class="workspace">
    <div class="workspace-empty">
      <h1>AstroForge</h1>
      <p class="tagline">Raw telescope data to publication-ready images</p>
      <div class="version">v0.1.0 — Foundation</div>
    </div>
  </section>

  <footer class="app-footer">
    <span>AstroForge v0.1.0</span>
    <span>Phase 0 — Foundation &amp; Scaffolding</span>
  </footer>
</main>

<style>
  .app {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--bg-primary);
  }

  .app-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 1.5rem;
    height: 3.5rem;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .logo {
    display: flex;
    align-items: center;
    gap: 0.625rem;
    color: var(--accent);
  }

  .app-name {
    font-size: 1.25rem;
    font-weight: 600;
    color: var(--text-primary);
    letter-spacing: 0.02em;
  }

  .gpu-badge {
    font-size: 0.8125rem;
    color: var(--text-muted);
    padding: 0.25rem 0.75rem;
    border-radius: 0.375rem;
    background: var(--bg-tertiary);
  }

  .gpu-badge.gpu-checked {
    color: var(--success);
  }

  .workspace {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-primary);
  }

  .workspace-empty {
    text-align: center;
  }

  .workspace-empty h1 {
    font-size: 3rem;
    font-weight: 700;
    color: var(--accent);
    margin-bottom: 0.5rem;
    letter-spacing: -0.02em;
  }

  .tagline {
    font-size: 1.125rem;
    color: var(--text-secondary);
    margin-bottom: 1rem;
  }

  .version {
    font-size: 0.875rem;
    color: var(--text-muted);
    padding: 0.25rem 0.75rem;
    border-radius: 0.375rem;
    background: var(--bg-secondary);
    display: inline-block;
  }

  .app-footer {
    display: flex;
    justify-content: space-between;
    padding: 0 1.5rem;
    height: 2.5rem;
    background: var(--bg-secondary);
    border-top: 1px solid var(--border);
    font-size: 0.75rem;
    color: var(--text-muted);
    align-items: center;
    flex-shrink: 0;
  }
</style>
