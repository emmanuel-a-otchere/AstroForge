import { createRouter, createWebHashHistory } from "vue-router";
import HomeView from "@/views/HomeView.vue";
import ProjectsView from "@/views/ProjectsView.vue";
import PlaceholderView from "@/views/PlaceholderView.vue";
import StudioPlaceholder from "@/views/StudioPlaceholder.vue";

const routes = [
  { path: "/", name: "home", component: HomeView },
  { path: "/projects", name: "projects", component: ProjectsView },
  { path: "/recipes", name: "recipes", component: PlaceholderView, props: { titleKey: "nav.recipes" } },
  { path: "/ai-models", name: "ai-models", component: PlaceholderView, props: { titleKey: "nav.aiModels" } },
  { path: "/settings", name: "settings", component: PlaceholderView, props: { titleKey: "nav.settings" } },
  { path: "/help", name: "help", component: PlaceholderView, props: { titleKey: "nav.help" } },
  { path: "/studio", name: "studio", component: StudioPlaceholder },
  { path: "/:pathMatch(.*)*", redirect: "/" },
];

export default createRouter({
  history: createWebHashHistory(),
  routes,
  scrollBehavior: () => ({ top: 0 }),
});
