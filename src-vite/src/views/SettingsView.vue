<template>
  <section class="screen" aria-labelledby="settings-title">
    <header>
      <h1 id="settings-title" class="font-display">Settings</h1>
      <p class="subtitle font-body">Application preferences. Persisted locally on this device.</p>
    </header>

    <section class="panel" aria-labelledby="appearance-title">
      <h2 id="appearance-title" class="font-display panel-title">Appearance</h2>
      <p class="font-body hint">
        AstroForge ships the "AstroForge Precision" dark theme (docs/DESIGN.md).
        In the Vue tree the theme is generated from those tokens into
        Tailwind/daisyUI (`src/assets/tokens.css`).
      </p>
    </section>

    <section class="panel" aria-labelledby="data-title">
      <h2 id="data-title" class="font-display panel-title">Data</h2>
      <p class="font-body hint">
        Projects live in the app data directory
        (<code>com.astroforge.app</code>). The window position and size are
        persisted via the window-state plugin.
      </p>
    </section>
  </section>
</template>

<style scoped>
.screen {
  display: flex;
  flex-direction: column;
  gap: var(--sp-lg);
  padding: var(--sp-xl);
  max-width: 1100px;
  margin: 0 auto;
  width: 100%;
  box-sizing: border-box;
  overflow-y: auto;
  color: var(--on-surface);
}

.screen h1 {
  font-size: 2rem;
  margin: 0 0 var(--sp-xs);
}

.subtitle {
  margin: 0;
  color: var(--on-surface-variant);
  font-size: 0.95rem;
}

.panel {
  background: var(--surface-container-low);
  border: 1px solid var(--outline-variant);
  border-radius: var(--radius-lg);
  padding: var(--sp-lg);
}

.panel-title {
  font-size: 1.05rem;
  margin: 0 0 var(--sp-sm);
}

.hint {
  color: var(--on-surface-variant);
  font-size: 0.88rem;
  margin: 0;
}

code {
  font-family: var(--font-mono);
  font-size: 0.85em;
}
</style>
