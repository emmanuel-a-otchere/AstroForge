import { defineStore } from "pinia";

export const APPLICATION_NAV_ITEMS = [
  { id: "home", labelKey: "nav.home", icon: "home", hintKey: "nav.homeHint" },
  { id: "projects", labelKey: "nav.projects", icon: "folder", hintKey: "nav.projectsHint" },
  { id: "recipes", labelKey: "nav.recipes", icon: "bookmark", hintKey: "nav.recipesHint" },
  { id: "ai-models", labelKey: "nav.aiModels", icon: "auto_awesome", hintKey: "nav.aiModelsHint" },
  { id: "settings", labelKey: "nav.settings", icon: "tune", hintKey: "nav.settingsHint" },
  { id: "help", labelKey: "nav.help", icon: "help", hintKey: "nav.helpHint" },
];

export const STUDIO_VIEWS = ["overview", "import", "process", "enhance", "compare", "export"];

export const STUDIO_NAV_ITEMS = [
  { id: "overview", label: "Overview", icon: "dashboard" },
  { id: "import", label: "Import", icon: "upload_file" },
  { id: "process", label: "Process", icon: "auto_fix_high" },
  { id: "enhance", label: "Enhance", icon: "tune" },
  { id: "compare", label: "Compare", icon: "compare" },
  { id: "export", label: "Export", icon: "download" },
];

export const useApplicationStore = defineStore("application", {
  state: () => ({
    navTarget: "home",
    studioProject: null,
    studioView: "overview",
  }),
  getters: {
    hasOpenProject: (s) => s.studioProject !== null,
    projectLabel: (s) => s.studioProject?.name ?? null,
  },
  actions: {
    setNav(target) {
      this.navTarget = target;
    },
    openProject(project) {
      this.studioProject = { project_id: project.project_id, name: project.name };
      this.studioView = "overview";
    },
    closeProject() {
      this.studioProject = null;
      this.studioView = "overview";
    },
    setView(view) {
      if (STUDIO_VIEWS.includes(view)) this.studioView = view;
    },
  },
  persist: { pick: ["navTarget"] },
});
