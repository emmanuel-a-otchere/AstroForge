<template>
  <section class="recipes-screen" aria-labelledby="recipes-title">
    <header class="recipes-header">
      <div>
        <h1 id="recipes-title" class="font-display">Recipes</h1>
        <p class="recipes-subtitle font-body">
          Reusable processing workflows, filterable by target, equipment, and palette.
        </p>
      </div>
      <input
        v-model="query"
        type="search"
        class="search font-body"
        placeholder="Search recipes…"
        aria-label="Search recipes"
      />
    </header>

    <p v-if="loading" class="status-line font-body" role="status">Loading recipes…</p>
    <p v-else-if="error" class="error-line font-body" role="alert">{{ error }}</p>
    <ul v-else class="recipe-grid" aria-label="Recipe list">
      <li
        v-for="recipe in filtered"
        :key="recipe.name"
        class="recipe-card"
        role="button"
        tabindex="0"
        @click="selected = recipe"
        @keydown.enter="selected = recipe"
      >
        <h2 class="font-display recipe-title">{{ recipe.name }}</h2>
        <p class="font-body recipe-meta">
          {{ recipe.targetType }} · {{ recipe.equipmentHints.camera }} · v{{ recipe.recipeVersion }}
        </p>
        <p class="font-body recipe-desc">{{ recipe.description }}</p>
      </li>
    </ul>

    <div
      v-if="selected"
      class="drawer-backdrop"
      role="presentation"
      @click="selected = null"
    >
      <aside
        class="drawer"
        role="dialog"
        aria-modal="true"
        :aria-label="selected.name"
        @click.stop
      >
        <header class="drawer-header">
          <h2 class="font-display">{{ selected.name }}</h2>
          <button
            type="button"
            class="close-btn font-body"
            aria-label="Close details"
            @click="selected = null"
          >
            ✕
          </button>
        </header>
        <div class="drawer-body">
          <p class="font-body">{{ selected.description }}</p>
          <dl class="meta-grid font-body">
            <dt>Author</dt><dd>{{ selected.author }}</dd>
            <dt>Target</dt><dd>{{ selected.targetType }}</dd>
            <dt>Camera</dt><dd>{{ selected.equipmentHints.camera }}</dd>
            <dt>Telescope</dt><dd>{{ selected.equipmentHints.telescope }}</dd>
            <dt>Filters</dt><dd>{{ selected.equipmentHints.filters.join(", ") }}</dd>
            <dt>Version</dt><dd>v{{ selected.recipeVersion }} · app {{ selected.appVersion }}</dd>
            <dt>Integrity</dt><dd>{{ selected.integrity.algorithm }} · {{ selected.integrity.digest.slice(0, 16) }}…</dd>
          </dl>
          <h3 class="font-display stages-title">Pipeline stages</h3>
          <ol class="stages-list font-body">
            <li v-for="stage in selected.pipeline" :key="stage.stage_type">
              {{ stage.label ?? stage.stage_type }}
            </li>
          </ol>
          <h3 class="font-display stages-title">Model versions</h3>
          <ul class="font-body">
            <li v-for="(v, k) in selected.modelVersions" :key="k">{{ k }} {{ v }}</li>
          </ul>
        </div>
      </aside>
    </div>
  </section>
</template>

<script setup>
import { computed, onMounted, ref } from "vue";
import { useStore } from "@/composables/useStore";
import { galleryStore, searchLocal } from "@/lib/recipeFeed";

const recipes = useStore(galleryStore);
const query = ref("");
const loading = ref(false);
const error = ref(null);

const selected = ref(null);

const filtered = computed(() =>
  searchLocal(recipes.value ?? [], { query: query.value.trim() || undefined }),
);

onMounted(() => {
  loading.value = false;
});
</script>

<style scoped>
.recipes-screen {
  display: flex;
  flex-direction: column;
  gap: var(--sp-lg);
  padding: var(--sp-xl);
  max-width: 1200px;
  margin: 0 auto;
  width: 100%;
  box-sizing: border-box;
  overflow-y: auto;
  color: var(--on-surface);
}

.recipes-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--sp-md);
  flex-wrap: wrap;
}

.recipes-header h1 {
  font-size: 2rem;
  margin: 0 0 var(--sp-xs);
}

.recipes-subtitle {
  margin: 0;
  color: var(--on-surface-variant);
  font-size: 0.95rem;
}

.search {
  background: var(--surface-container-low);
  border: 1px solid var(--outline-variant);
  border-radius: var(--radius-md);
  color: var(--on-surface);
  padding: var(--sp-sm) var(--sp-md);
  font-size: 0.9rem;
  min-width: 260px;
}

.recipe-grid {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
  gap: var(--sp-md);
}

.recipe-card {
  background: var(--surface-container-low);
  border: 1px solid var(--outline-variant);
  border-radius: var(--radius-lg);
  padding: var(--sp-md) var(--sp-lg);
  display: flex;
  flex-direction: column;
  gap: var(--sp-xs);
  cursor: pointer;
}

.recipe-card:hover,
.recipe-card:focus-visible {
  border-color: var(--primary);
}

.drawer-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  justify-content: flex-end;
  z-index: 80;
}

.drawer {
  width: min(480px, 92vw);
  height: 100%;
  background: var(--surface-container);
  border-left: 1px solid var(--outline-variant);
  display: flex;
  flex-direction: column;
  color: var(--on-surface);
}

.drawer-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--sp-lg);
  border-bottom: 1px solid var(--outline-variant);
}

.drawer-header h2 {
  margin: 0;
  font-size: 1.2rem;
}

.close-btn {
  background: transparent;
  border: none;
  color: var(--on-surface-variant);
  cursor: pointer;
  font-size: 1rem;
}

.drawer-body {
  padding: var(--sp-lg);
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: var(--sp-md);
  font-size: 0.88rem;
}

.meta-grid {
  display: grid;
  grid-template-columns: auto 1fr;
  gap: 4px var(--sp-md);
  margin: 0;
}

.meta-grid dt {
  color: var(--on-surface-variant);
}

.meta-grid dd {
  margin: 0;
}

.stages-title {
  font-size: 0.95rem;
  margin: var(--sp-sm) 0 0;
}

.stages-list {
  margin: 0;
  padding-left: var(--sp-lg);
}

.recipe-title {
  margin: 0;
  font-size: 1.05rem;
}

.recipe-meta,
.recipe-desc {
  margin: 0;
  font-size: 0.8rem;
  color: var(--on-surface-variant);
}

.status-line,
.error-line {
  margin: 0;
}

.error-line {
  color: #ff8a80;
}
</style>
