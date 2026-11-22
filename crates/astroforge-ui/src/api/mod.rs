//! Direct-call API layer — the egui replacement for
//! `src/lib/astroforge-api.ts` (124 `invoke()` wrappers).
//!
//! One module per command group, mirroring the TS file's group
//! order. Each function is a thin wrapper over `astroforge-core`
//! calls, with bodies ported from the corresponding Tauri command so
//! behavior stays identical during the migration window. Errors are
//! `String` (same message text the Tauri commands surface) so screens
//! can render one consistent error path; a later slice may introduce
//! a typed error enum.

pub mod project;
