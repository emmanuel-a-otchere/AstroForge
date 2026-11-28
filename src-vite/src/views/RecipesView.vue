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
      <li v-for="recipe in filtered" :key="recipe.id" class="recipe-card">
        <h2 class="font-display recipe-title">{{ recipe.name }}</h2>
        <p class="font-body recipe-meta">
          {{ recipe.targetType }} · {{ recipe.equipmentHints.camera }} · v{{ recipe.recipeVersion }}
        </p>
        <p class="font-body recipe-desc">{{ recipe.description }}</p>
      </li>
    </ul>
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
