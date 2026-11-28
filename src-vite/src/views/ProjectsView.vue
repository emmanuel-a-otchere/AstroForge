<template>
  <section class="projects-screen" aria-labelledby="projects-title">
    <header class="projects-header">
      <div>
        <h1 id="projects-title" class="font-display">{{ $t("projects.title") }}</h1>
        <p class="projects-subtitle font-body">{{ $t("projects.subtitle") }}</p>
      </div>
      <button
        type="button"
        class="primary-cta font-display"
        @click="emit('action', 'create')"
      >
        <span class="material-symbols-outlined cta-icon" aria-hidden="true">add</span>
        {{ $t("projects.newProject") }}
      </button>
    </header>

    <p v-if="projects.loading" class="status-line font-body" role="status">
      {{ $t("projects.loading") }}
    </p>
    <div v-else-if="projects.error" class="error-state" role="alert">
      <span class="material-symbols-outlined" aria-hidden="true">error</span>
      <p class="font-body">{{ $t("projects.loadError", { error: projects.error }) }}</p>
      <button type="button" class="ghost-link font-body" @click="projects.refresh()">
        {{ $t("projects.retry") }}
      </button>
    </div>
    <div v-else-if="projects.isEmpty" class="empty-state">
      <span class="material-symbols-outlined empty-icon" aria-hidden="true">
        collections
      </span>
      <p class="empty-title font-display">{{ $t("home.emptyTitle") }}</p>
      <p class="empty-body font-body">{{ $t("projects.emptyBody") }}</p>
      <div class="empty-actions">
        <button
          type="button"
          class="primary-cta font-display"
          @click="emit('action', 'create')"
        >
          {{ $t("home.createProject") }}
        </button>
      </div>
    </div>
    <ul v-else class="project-grid" aria-label="Project list">
      <li v-for="project in projects.items" :key="project.project_id" class="project-card">
        <button
          type="button"
          class="card-button"
          :aria-label="$t('projects.openAria', { name: project.name })"
          @click="emit('action', 'open', project)"
        >
          <div class="thumb" aria-hidden="true">
            <span class="material-symbols-outlined thumb-icon">image</span>
          </div>
          <div class="card-body">
            <div class="card-title-row">
              <h2 class="font-display card-title">{{ project.name }}</h2>
              <span class="status-pill" :data-status="project.status">
                {{ project.status }}
              </span>
            </div>
            <p class="card-meta font-body">
              {{ project.application_version }} ·
              {{ $t("projects.updated", { date: formatDate(project.updated_at) }) }}
            </p>
            <p class="card-meta font-body">
              Target:
              {{ project.active_session_id ? $t("projects.targetActive") : $t("projects.targetNone") }}
            </p>
          </div>
        </button>
        <div class="card-actions">
          <button
            type="button"
            class="ghost-link font-body"
            @click="emit('action', 'rename', project)"
          >
            {{ $t("projects.rename") }}
          </button>
          <button
            type="button"
            class="ghost-link font-body"
            @click="emit('action', 'archive', project)"
          >
            {{ $t("projects.archive") }}
          </button>
          <button
            type="button"
            class="ghost-link danger font-body"
            @click="emit('action', 'delete', project)"
          >
            {{ $t("projects.delete") }}
          </button>
        </div>
      </li>
    </ul>
  </section>
</template>

<script setup>
import { onMounted } from "vue";
import { useProjectsStore } from "@/stores/projects";
import { formatDate } from "@/common/utils";

const emit = defineEmits(["action"]);

const projects = useProjectsStore();

onMounted(() => {
  void projects.refresh();
});
</script>

<style scoped>
.projects-screen {
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

.projects-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--sp-md);
}

.projects-header h1 {
  font-size: 2rem;
  margin: 0 0 var(--sp-xs) 0;
}

.projects-subtitle {
  margin: 0;
  color: var(--on-surface-variant);
  font-size: 0.95rem;
}

.primary-cta {
  display: inline-flex;
  align-items: center;
  gap: var(--sp-xs);
  background: var(--primary);
  color: var(--on-primary);
  border: none;
  padding: var(--sp-sm) var(--sp-lg);
  border-radius: var(--radius-md);
  cursor: pointer;
  font-size: 0.95rem;
  font-weight: 600;
}

.cta-icon {
  font-size: 20px;
}

.status-line {
  color: var(--on-surface-variant);
}

.error-state {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
  padding: var(--sp-md) var(--sp-lg);
  border: 1px solid #e53935;
  border-radius: var(--radius-md);
  color: #ff8a80;
}

.error-state p {
  margin: 0;
  flex: 1 1 auto;
}

.ghost-link {
  background: transparent;
  border: none;
  color: var(--primary);
  cursor: pointer;
  font-size: 0.85rem;
  padding: var(--sp-xs) var(--sp-sm);
}

.ghost-link:hover {
  text-decoration: underline;
}

.ghost-link.danger {
  color: #ff8a80;
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  gap: var(--sp-sm);
  padding: var(--sp-xl);
  border: 1px dashed var(--outline-variant);
  border-radius: var(--radius-lg);
  color: var(--on-surface-variant);
}

.empty-icon {
  font-size: 48px;
  color: var(--outline);
}

.empty-title {
  font-size: 1.2rem;
  font-weight: 600;
  color: var(--on-surface);
  margin: 0;
}

.empty-body {
  margin: 0;
  max-width: 46ch;
}

.empty-actions {
  display: flex;
  gap: var(--sp-sm);
  margin-top: var(--sp-sm);
}

.project-grid {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  gap: var(--sp-md);
}

.project-card {
  display: flex;
  flex-direction: column;
  background: var(--surface-container-low);
  border: 1px solid var(--outline-variant);
  border-radius: var(--radius-lg);
  overflow: hidden;
}

.card-button {
  display: flex;
  gap: var(--sp-md);
  padding: var(--sp-md);
  background: transparent;
  border: none;
  color: var(--on-surface);
  cursor: pointer;
  text-align: left;
  flex: 1 1 auto;
}

.card-button:hover {
  background: var(--surface-container);
}

.thumb {
  width: 72px;
  height: 72px;
  flex: 0 0 auto;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--surface-container);
  border-radius: var(--radius-md);
}

.thumb-icon {
  font-size: 32px;
  color: var(--outline);
}

.card-body {
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.card-title-row {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
}

.card-title {
  margin: 0;
  font-size: 1.05rem;
}

.status-pill {
  font-size: 0.7rem;
  padding: 2px 8px;
  border-radius: var(--radius-full);
  background: var(--surface-container-high);
  color: var(--on-surface-variant);
}

.status-pill[data-status="Active"] {
  background: var(--primary-container);
  color: var(--on-primary-container);
}

.card-meta {
  margin: 0;
  font-size: 0.78rem;
  color: var(--on-surface-variant);
}

.card-actions {
  display: flex;
  border-top: 1px solid var(--outline-variant);
  padding: var(--sp-xs) var(--sp-sm);
}
</style>
