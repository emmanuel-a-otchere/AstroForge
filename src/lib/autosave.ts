import { createClient } from "@supabase/supabase-js";
import { serializeSession, deserializeSession, sessionStore } from "./pipeline-store";

const supabaseUrl = import.meta.env.VITE_SUPABASE_URL;
const supabaseAnonKey = import.meta.env.VITE_SUPABASE_ANON_KEY;

const supabase = createClient(supabaseUrl, supabaseAnonKey);

let autosaveTimer: ReturnType<typeof setTimeout> | null = null;
let isSaving = false;

export function startAutosave(intervalMs: number = 5000): void {
  if (autosaveTimer) clearTimeout(autosaveTimer);

  autosaveTimer = setInterval(async () => {
    await saveSession();
  }, intervalMs);
}

export function stopAutosave(): void {
  if (autosaveTimer) {
    clearTimeout(autosaveTimer);
    autosaveTimer = null;
  }
}

export async function saveSession(): Promise<void> {
  if (isSaving) return;
  isSaving = true;

  try {
    const stateJson = serializeSession();
    const state = JSON.parse(stateJson);
    const sessionId = state.sessionId;

    const { error } = await supabase
      .from("processing_sessions")
      .upsert({
        id: sessionId,
        state: state,
        updated_at: new Date().toISOString(),
      });

    if (error) {
      console.warn("Autosave failed:", error.message);
    }
  } catch {
    // autosave is best-effort; don't crash the app
  } finally {
    isSaving = false;
  }
}

export async function loadSession(sessionId: string): Promise<boolean> {
  try {
    const { data, error } = await supabase
      .from("processing_sessions")
      .select("state")
      .eq("id", sessionId)
      .maybeSingle();

    if (error || !data) {
      return false;
    }

    deserializeSession(JSON.stringify(data.state));
    return true;
  } catch {
    return false;
  }
}

export async function loadMostRecentSession(): Promise<string | null> {
  try {
    const { data, error } = await supabase
      .from("processing_sessions")
      .select("id, updated_at")
      .order("updated_at", { ascending: false })
      .limit(1)
      .maybeSingle();

    if (error || !data) {
      return null;
    }

    return data.id;
  } catch {
    return null;
  }
}

export async function deleteSession(sessionId: string): Promise<void> {
  try {
    await supabase.from("processing_sessions").delete().eq("id", sessionId);
  } catch {
    // best-effort cleanup
  }
}

export async function listSessions(): Promise<Array<{ id: string; updated_at: string }>> {
  try {
    const { data, error } = await supabase
      .from("processing_sessions")
      .select("id, updated_at")
      .order("updated_at", { ascending: false });

    if (error || !data) {
      return [];
    }

    return data;
  } catch {
    return [];
  }
}
