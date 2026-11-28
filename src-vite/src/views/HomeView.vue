<template>
  <section class="home-screen" aria-labelledby="home-title">
    <header class="home-header">
      <h1 id="home-title" class="font-display">{{ $t("home.title") }}</h1>
      <p class="home-subtitle font-body">{{ $t("home.subtitle") }}</p>
    </header>

    <section class="primary-actions" aria-label="Primary actions">
      <button
        v-for="action in primaryActions"
        :key="action.id"
        type="button"
        class="primary-action"
        :class="{ primary: action.primary }"
        @click="emit('action', action.id)"
      >
        <span class="material-symbols-outlined icon" aria-hidden="true">
          {{ action.icon }}
        </span>
        <span class="action-body">
          <span class="action-label font-display">{{ $t(action.labelKey) }}</span>
          <span class="action-description font-body">
            {{ $t(action.descriptionKey) }}
          </span>
        </span>
      </button>
    </section>

    <section class="recent-section" aria-label="Recent projects">
      <header class="recent-header">
        <h2 class="font-display">{{ $t("home.recent") }}</h2>
        <button type="button" class="ghost-link font-body" @click="goProjects">
          {{ $t("home.seeAll") }}
        </button>
      </header>

      <p v-if="projects.loading" class="empty-body font-body" aria-live="polite">
        {{ $t("home.loading") }}
      </p>
      <p v-else-if="projects.error" class="error-line font-body" role="alert">
        {{ $t("home.loadError", { error: projects.error }) }}
      </p>
      <div v-else-if="projects.items.length === 0" class="empty-state">
        <span class="material-symbols-outlined empty-icon" aria-hidden="true">
          collections
        </span>
        <p class="empty-title font-display">{{ $t("home.emptyTitle") }}</p>
        <p class="empty-body font-body">{{ $t("home.emptyBody") }}</p>
        <div class="empty-actions">
          <button
            type="button"
            class="primary-cta font-display"
            @click="emit('action', 'new-project')"
          >
            {{ $t("home.createProject") }}
          </button>
          <button
            type="button"
            class="secondary-cta font-display"
            @click="emit('action', 'open-project')"
          >
            {{ $t("home.importExisting") }}
          </button>
        </div>
      </div>
      <div v-else class="project-grid" role="list">
        <article
          v-for="project in projects.items"
          :key="project.project_id"
          class="project-card"
          role="listitem"
        >
          <h3 class="font-display">{{ project.name }}</h3>
          <p class="font-body">
            {{ project.status }}
            <template v-if="project.active_session_id">
              · {{ $t("home.sessionActive") }}
            </template>
          </p>
          <button
            type="button"
            class="primary-cta font-display"
            @click="emit('action', 'open-project', { name: project.name })"
          >
            {{ $t("home.open") }}
          </button>
        </article>
      </div>
    </section>
  </section>
</template>

<script setup>
import { onMounted } from "vue";
import { useRouter } from "vue-router";
import { storeToRefs } from "pinia";
import { useApplicationStore } from "@/stores/application";
import { useProjectsStore } from "@/stores/projects";

const emit = defineEmits(["action"]);

const appStore = useApplicationStore();
const projects = useProjectsStore();
const router = useRouter();

const primaryActions = [
  {
    id: "new-project",
    labelKey: "home.actions.newProject.label",
    descriptionKey: "home.actions.newProject.description",
    icon: "add_circle",
    primary: true,
  },
  {
    id: "open-project",
    labelKey: "home.actions.openProject.label",
    descriptionKey: "home.actions.openProject.description",
    icon: "folder_open",
    primary: false,
  },
  {
    id: "import-session",
    labelKey: "home.actions.importSession.label",
    descriptionKey: "home.actions.importSession.description",
    icon: "file_download",
    primary: false,
  },
  {
    id: "resume-processing",
    labelKey: "home.actions.resumeProcessing.label",
    descriptionKey: "home.actions.resumeProcessing.description",
    icon: "play_arrow",
    primary: false,
  },
];

onMounted(() => {
  void projects.refresh();
});

function goProjects() {
  appStore.setNav("projects");
  void router.push("/projects");
}
</script>

<style scoped>
.home-screen {
  display: grid;
  grid-template-rows: auto 1fr auto;
  gap: var(--sp-xl);
  padding: var(--sp-xl);
  max-width: 1200px;
  margin: 0 auto;
  width: 100%;
  box-sizing: border-box;
  overflow-y: auto;
  color: var(--on-surface);
}

.home-header h1 {
  font-size: 2.5rem;
  margin: 0 0 var(--sp-sm) 0;
  color: var(--on-surface);
}

.home-subtitle {
  font-size: 1rem;
  margin: 0;
  color: var(--on-surface-variant);
}

.primary-actions {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
  gap: var(--sp-md);
}

.primary-action {
  display: flex;
  align-items: flex-start;
  gap: var(--sp-md);
  padding: var(--sp-md) var(--sp-lg);
  background: var(--surface-container-low);
  border: 1px solid var(--outline-variant);
  border-radius: var(--radius-lg);
  color: var(--on-surface);
  cursor: pointer;
  text-align: left;
  transition: background 0.12s ease, border-color 0.12s ease;
}

.primary-action:hover {
  background: var(--surface-container);
}

.primary-action.primary {
  background: var(--primary-container);
  border-color: var(--primary);
}

.primary-action .icon {
  font-size: 28px;
  color: var(--primary);
}

.action-body {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.action-label {
  font-size: 1rem;
  font-weight: 600;
}

.action-description {
  font-size: 0.82rem;
  color: var(--on-surface-variant);
  line-height: 1.35;
}

.recent-header {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  margin-bottom: var(--sp-md);
}

.recent-header h2 {
  margin: 0;
  font-size: 1.3rem;
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

.error-line {
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

.primary-cta {
  background: var(--primary);
  color: var(--on-primary);
  border: none;
  padding: var(--sp-sm) var(--sp-lg);
  border-radius: var(--radius-md);
  cursor: pointer;
  font-size: 0.95rem;
  font-weight: 600;
}

.secondary-cta {
  background: transparent;
  color: var(--on-surface-variant);
  border: 1px solid var(--outline-variant);
  padding: var(--sp-sm) var(--sp-lg);
  border-radius: var(--radius-md);
  cursor: pointer;
  font-size: 0.95rem;
  font-weight: 600;
}

.project-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
  gap: var(--sp-md);
}

.project-card {
  display: flex;
  flex-direction: column;
  gap: var(--sp-xs);
  padding: var(--sp-md) var(--sp-lg);
  background: var(--surface-container-low);
  border: 1px solid var(--outline-variant);
  border-radius: var(--radius-lg);
}

.project-card h3 {
  margin: 0;
  font-size: 1.05rem;
}

.project-card p {
  margin: 0;
  font-size: 0.8rem;
  color: var(--on-surface-variant);
}

.project-card .primary-cta {
  align-self: flex-start;
  margin-top: var(--sp-xs);
  padding: var(--sp-xs) var(--sp-md);
  font-size: 0.85rem;
}
</style>
