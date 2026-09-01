/*
# Create processing_sessions table for crash-safe autosave

1. New Tables
- `processing_sessions`
  - `id` (text, primary key) — the session_id from the frontend store
  - `state` (jsonb, not null) — serialized SessionState (mode, pipeline graph, history, stats)
  - `created_at` (timestamptz) — when the session was first saved
  - `updated_at` (timestamptz) — last autosave timestamp

2. Security
- Enable RLS on `processing_sessions`.
- Single-tenant app (no sign-in): allow anon + authenticated full CRUD so the
  anon-key frontend can autosave and restore session state.

3. Notes
- The `state` column stores the full serialized session as JSONB, enabling
  crash recovery on app launch by reading the most recent session.
- The `updated_at` column drives "most recent session" queries for crash recovery.
*/

CREATE TABLE IF NOT EXISTS processing_sessions (
  id text PRIMARY KEY,
  state jsonb NOT NULL,
  created_at timestamptz DEFAULT now(),
  updated_at timestamptz DEFAULT now()
);

ALTER TABLE processing_sessions ENABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS "anon_select_sessions" ON processing_sessions;
CREATE POLICY "anon_select_sessions" ON processing_sessions FOR SELECT
  TO anon, authenticated USING (true);

DROP POLICY IF EXISTS "anon_insert_sessions" ON processing_sessions;
CREATE POLICY "anon_insert_sessions" ON processing_sessions FOR INSERT
  TO anon, authenticated WITH CHECK (true);

DROP POLICY IF EXISTS "anon_update_sessions" ON processing_sessions;
CREATE POLICY "anon_update_sessions" ON processing_sessions FOR UPDATE
  TO anon, authenticated USING (true) WITH CHECK (true);

DROP POLICY IF EXISTS "anon_delete_sessions" ON processing_sessions;
CREATE POLICY "anon_delete_sessions" ON processing_sessions FOR DELETE
  TO anon, authenticated USING (true);
