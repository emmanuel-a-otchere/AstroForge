# ADR-0002 — Smart-Telescope SDK Integration

- Status: Accepted (2026-09-12)
- Gates: `P4-M2-T1+` (PROJECT_PLAN § Phase 4.2 — Recipe Gallery hosting
  depends on smart-telescope session metadata; Phase 2 capture paths
  use file-only ingest regardless)
- Closes: GitHub issue #133 ("Smart-telescope SDK integration vs.
  file-only")

## Context

Smart telescopes (ZWO Seestar, Dwarf family, etc.) increasingly ship
with vendor SDKs that expose live session state, frame metadata,
target acquisition, and even on-device stacking. The architectural
question for AstroForge's v1.x contract is whether the import path
should pull from these SDKs directly (a "live" path) or stay file-only
(a "cold" path that consumes a captured folder after the fact).

Three options were on the table:

1. **Live SDK integration** — AstroForge imports from the vendor
   SDK in real time.
2. **File-only ingest (cold path)** — AstroForge consumes a captured
   folder after the session ends, identical to the Phase 1.5 path.
3. **Both** — file-only is the v1.x contract; live SDK is a Phase 4
   plugin under the recipe gallery.

## Decision

**File-only ingest is the v1.x contract. Live SDK integration is
deferred to Phase 4 as a plugin, not a first-party import path.**

### Why file-only for v1.x

- **Determinism.** File-only ingest produces a stable folder → session
  flow that is reproducible across machines; SDK integration adds a
  network-of-vendors dependency that complicates provenance, replay,
  and crash recovery.
- **Cross-vendor portability.** The file-only contract works for
  Seestar, Dwarf, ASIAIR, Stellina, and any future vendor — without
  per-vendor SDK maintenance.
- **Spec promise.** The spec's user-journey (CR-01: Discover → Create
  → Import → Understand → Process → Review → Enhance → Compare → Export
  → Reuse) treats Import as a folder drop; an SDK integration would
  shift Import from a folder drop to a vendor-aware wizard, which
  fragments the UX.
- **Spec-driven development workflow.** Spec edits are expensive
  (bump carrier, audit reconciliation); deferring SDK to a Phase 4
  plugin keeps the spec contract stable.

### Why defer (not reject) SDK integration

- Vendor SDKs add real value for users on a single telescope: live
  framing, target acquisition, on-device stacking preview. The Phase 4
  plugin architecture (P4-M1-T1..T5) is the right home for that.
- A Phase 4 SDK plugin can compose with the recipe gallery (P4-M2):
  a user with a Seestar subscribes to the Seestar recipe pack,
  publishes their own.

## Consequences

- **Import contract is unchanged.** A folder drop remains the v1.x
  contract; the user is not blocked by missing SDK support for their
  telescope.
- **No vendor-specific code in the AstroForge binary.** The Phase 4
  plugin API keeps the SDK code outside the core, which simplifies
  licensing and maintenance.
- **Smart-telescope recipes can still be authored.** A Seestar user
  captures a session, drops the folder, picks the Seestar recipe from
  the gallery, processes — the same UX as for a DSLR user.
- **Phase 4 plugin API contract (P4-M1-T1) must accommodate vendor
  SDKs.** The plugin scope includes "import session from device"
  capability, scoped per-plugin to its declared vendor.

## Follow-ups

- File follow-up issue for Phase 4 SDK plugin scoping (P4-M1-T1).
- Update the spec (1.4.0 bump) to call out the file-only contract
  as v1.x canonical, SDK integration as a Phase 4 plugin.
- Phase 4 recipe gallery (P4-M2-T1) hosting decision (#119) should
  also factor in vendor-tagging so Seestar recipes are discoverable
  by Seestar users.
