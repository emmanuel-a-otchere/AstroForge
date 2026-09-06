// CR-03 P6 — dialog state for cross-component coordination.
//
// Extracted so the keyboard-shortcut handler can read whether a
// dialog is open and let the dialog handle its own Escape.

import { writable } from "svelte/store";

export const dialogOpen = writable(false);
export const deleteDialogOpen = writable(false);