<template>
  <section class="screen" aria-labelledby="help-title">
    <header>
      <h1 id="help-title" class="font-display">Help</h1>
      <p class="subtitle font-body">Docs and keyboard shortcuts.</p>
    </header>

    <section class="panel">
      <h2 class="font-display panel-title">Keyboard shortcuts</h2>
      <ul class="shortcuts font-body">
        <li><kbd>⌘N</kbd> New project</li>
        <li><kbd>⌘O</kbd> Open projects</li>
        <li><kbd>⌘S</kbd> Mark project saved</li>
        <li><kbd>⌘1</kbd>–<kbd>⌘6</kbd> Switch studio tab</li>
        <li><kbd>Esc</kbd> Close project / dialog</li>
      </ul>
    </section>

    <section class="panel">
      <h2 class="font-display panel-title">Workflow</h2>
      <p class="font-body hint">
        Home → create or open a project → Import raw captures → Process runs
        the pipeline → Enhance proposes AI tweaks → Compare picks a winner →
        Export writes the final file.
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

.shortcuts {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: var(--sp-xs);
  font-size: 0.9rem;
}

kbd {
  display: inline-block;
  padding: 1px 6px;
  background: var(--surface-container);
  border: 1px solid var(--outline-variant);
  border-radius: var(--radius-sm);
  font-family: var(--font-data);
  font-size: 0.75rem;
  margin: 0 2px;
}

.hint {
  color: var(--on-surface-variant);
  font-size: 0.88rem;
  margin: 0;
}
</style>
