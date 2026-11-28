import { createRouter, createWebHashHistory } from "vue-router";
import HomeView from "@/views/HomeView.vue";
import ProjectsView from "@/views/ProjectsView.vue";
import RecipesView from "@/views/RecipesView.vue";
import AiModelsView from "@/views/AiModelsView.vue";
import SettingsView from "@/views/SettingsView.vue";
import HelpView from "@/views/HelpView.vue";
import StudioShell from "@/components/StudioShell.vue";

const routes = [
  { path: "/", name: "home", component: HomeView },
  { path: "/projects", name: "projects", component: ProjectsView },
  { path: "/recipes", name: "recipes", component: RecipesView },
  { path: "/ai-models", name: "ai-models", component: AiModelsView },
  { path: "/settings", name: "settings", component: SettingsView },
  { path: "/help", name: "help", component: HelpView },
  { path: "/studio", name: "studio", component: StudioShell },
  { path: "/:pathMatch(.*)*", redirect: "/" },
];

export default createRouter({
  history: createWebHashHistory(),
  routes,
  scrollBehavior: () => ({ top: 0 }),
});
