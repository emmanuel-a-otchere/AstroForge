import { defineStore } from "pinia";
import * as api from "../lib/astroforge-api";
import { countByStatus } from "../common/utils.js";
export { countByStatus };

export const useProjectsStore = defineStore("projects", {
  state: () => ({ items: [], loading: false, error: null }),
  getters: {
    isEmpty: (s) => !s.loading && s.items.length === 0 && s.error === null,
  },
  actions: {
    async refresh() {
      this.loading = true;
      this.error = null;
      try {
        this.items = await api.projectList();
      } catch (e) {
        this.items = [];
        this.error = e instanceof Error ? e.message : String(e);
      } finally {
        this.loading = false;
      }
    },
  },
});
