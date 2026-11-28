import { test } from "node:test";
import assert from "node:assert/strict";
import { createPinia, setActivePinia } from "pinia";
import { useApplicationStore, STUDIO_VIEWS } from "../src/stores/application.js";
import { useProjectsStore, countByStatus } from "../src/stores/projects.js";
import { useProjectContextStore } from "../src/stores/projectContext.js";

test("application store navigates and tracks the studio viewport", () => {
  setActivePinia(createPinia());
  const app = useApplicationStore();
  assert.equal(app.navTarget, "home");
  assert.equal(app.hasOpenProject, false);
  app.openProject({ project_id: "p1", name: "M42" });
  assert.equal(app.hasOpenProject, true);
  assert.equal(app.projectLabel, "M42");
  app.setView("process");
  assert.equal(app.studioView, "process");
  app.setView("bogus");
  assert.equal(app.studioView, "process");
  assert.ok(STUDIO_VIEWS.includes("export"));
  app.closeProject();
  assert.equal(app.hasOpenProject, false);
});

test("projects store starts empty and countByStatus tallies", () => {
  setActivePinia(createPinia());
  const projects = useProjectsStore();
  assert.equal(projects.isEmpty, true);
  assert.deepEqual(
    countByStatus([
      { status: "Active" },
      { status: "Active" },
      { status: "Archived" },
    ]),
    { Active: 2, Archived: 1, Exported: 0 },
  );
});

test("project context open/close/dirty", () => {
  setActivePinia(createPinia());
  const ctx = useProjectContextStore();
  assert.equal(ctx.hasOpenProject, false);
  ctx.open({ project_id: "p1", name: "M42" });
  assert.equal(ctx.hasOpenProject, true);
  ctx.markDirty(true);
  assert.equal(ctx.isProjectDirty, true);
  ctx.close();
  assert.equal(ctx.hasOpenProject, false);
});
