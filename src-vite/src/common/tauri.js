import { invoke } from "@tauri-apps/api/core";

export const inTauri =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export function invokeCommand(cmd, args) {
  if (inTauri && window.__TAURI__?.core?.invoke) {
    return window.__TAURI__.core.invoke(cmd, args);
  }
  return invoke(cmd, args);
}
