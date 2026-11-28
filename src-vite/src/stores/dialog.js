import { defineStore } from "pinia";

export const useDialogStore = defineStore("dialog", {
  state: () => ({
    dialogOpen: false,
    dialogMode: "create",
    dialogProject: null,
    deleteDialogOpen: false,
    deleteTarget: null,
    dialogError: null,
    postCreateAction: null,
  }),
  actions: {
    openCreate(postCreateAction = null) {
      this.dialogMode = "create";
      this.dialogProject = null;
      this.dialogOpen = true;
      this.postCreateAction = postCreateAction;
      this.dialogError = null;
    },
    openRename(project) {
      this.dialogMode = "rename";
      this.dialogProject = project;
      this.dialogOpen = true;
      this.dialogError = null;
    },
    closeDialog() {
      this.dialogOpen = false;
      this.dialogError = null;
    },
    openDelete(project) {
      this.deleteTarget = project;
      this.deleteDialogOpen = true;
    },
    closeDelete() {
      this.deleteDialogOpen = false;
      this.deleteTarget = null;
    },
  },
});
