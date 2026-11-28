import { defineStore } from "pinia";

export const useProjectContextStore = defineStore("projectContext", {
  state: () => ({ project: null, dirty: false }),
  getters: {
    hasOpenProject: (s) => s.project !== null,
    isProjectDirty: (s) => s.dirty,
  },
  actions: {
    open(project) {
      this.project = project;
      this.dirty = false;
    },
    close() {
      this.project = null;
      this.dirty = false;
    },
    markDirty(dirty) {
      this.dirty = dirty;
    },
  },
});
