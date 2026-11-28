<template>
  <div class="application-shell" :class="{ 'nav-collapsed': !navOpen }">
    <TitleBar />
    <header class="app-header">
      <button
        type="button"
        class="nav-toggle material-symbols-outlined"
        :aria-label="navOpen ? $t('nav.hideNav') : $t('nav.showNav')"
        @click="navOpen = !navOpen"
      >
        {{ navOpen ? "menu_open" : "menu" }}
      </button>
      <span class="brand font-display">{{ $t("app.name") }}</span>
      <span class="project-slot font-body" data-testid="project-slot">
        {{ projectLabel ?? $t("app.noProject") }}
      </span>
      <slot name="projectTools"></slot>
      <span class="header-spacer"></span>
      <SaveIndicator />
    </header>

    <div class="app-body">
      <nav v-if="navOpen" class="app-nav" aria-label="Application navigation">
        <ul class="nav-list">
          <li v-for="item in navItems" :key="item.id">
            <button
              type="button"
              class="nav-item"
              :class="{ active: appStore.navTarget === item.id }"
              :aria-current="appStore.navTarget === item.id ? 'page' : undefined"
              :aria-disabled="appStore.hasOpenProject && item.id !== 'home'"
              :title="navTitle(item)"
              @click="select(item.id)"
            >
              <span class="material-symbols-outlined nav-icon" aria-hidden="true">
                {{ item.icon }}
              </span>
              <span class="nav-text">
                <span class="nav-label font-display">{{ $t(item.labelKey) }}</span>
                <span class="nav-hint font-body">{{ $t(item.hintKey) }}</span>
              </span>
            </button>
          </li>
        </ul>
      </nav>

      <main class="app-workspace" aria-label="Workspace">
        <slot></slot>
      </main>
    </div>
  </div>
</template>

<script setup>
import { computed, ref } from "vue";
import { useRouter } from "vue-router";
import { storeToRefs } from "pinia";
import TitleBar from "./TitleBar.vue";
import SaveIndicator from "./SaveIndicator.vue";
import { APPLICATION_NAV_ITEMS, useApplicationStore } from "@/stores/application";

const props = defineProps({
  projectLabel: { type: String, default: null },
});

const appStore = useApplicationStore();
const { navTarget } = storeToRefs(appStore);
const router = useRouter();

const navOpen = ref(true);
const navItems = APPLICATION_NAV_ITEMS;

const routeByNav = {
  home: "/",
  projects: "/projects",
  recipes: "/recipes",
  "ai-models": "/ai-models",
  settings: "/settings",
  help: "/help",
};

function select(target) {
  if (appStore.hasOpenProject && target !== "home") return;
  appStore.setNav(target);
  void router.push(routeByNav[target] ?? "/");
}

function navTitle(item) {
  return appStore.hasOpenProject && item.id !== "home"
    ? "Close the project (Esc) to return to the application screens"
    : undefined;
}
</script>

<style scoped>
.application-shell {
  display: grid;
  grid-template-rows: auto auto 1fr;
  height: 100vh;
  background: var(--surface);
  color: var(--on-surface);
  font-family: var(--font-body);
  border-radius: var(--radius-lg);
  overflow: hidden;
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
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}
</style>
