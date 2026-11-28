import { defineStore } from "pinia";
import * as api from "../common/api.js";

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

export function countByStatus(items) {
  const counts = { Active: 0, Archived: 0, Exported: 0 };
  for (const p of items) counts[p.status] += 1;
  return counts;
}
