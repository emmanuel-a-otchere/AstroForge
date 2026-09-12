# DP#4 Catalog Audit Close-Out Checklist

This checklist documents the per-entry contract that closes DP#4
catalog license verification. The runtime machinery (`LicenseSpdx`,
`CatalogAuditGap`, `verify_catalog_audit`) is in place; each entry
that wants to ship must satisfy the checklist below.

## Per-entry contract

For every entry in `crates/astroforge-ai/src/inference.rs::CATALOG_MODELS`:

1. **Pinned entries** (sha256 ≠ `UNVERIFIED_SHA256`):
   - [ ] `license: Some(LicenseSpdx::Concrete)` is set.
   - [ ] Concrete SPDX is one of: `Apache-2.0`, `MIT`, `BSD-3-Clause`,
         `CC-BY-SA-4.0`, `CC-BY-NC-4.0` (no other SPDX accepted).
   - [ ] `LicenseSpdx::Unknown` is **forbidden** for pinned entries —
         the audit fails closed on this case.
   - [ ] If `LicenseSpdx::CcByNc40`, the apply path tags the output
         with a non-commercial integrity badge (consumer-facing
         signal; not yet wired in this slice).
   - [ ] Source URL for the model weights is published in this
         repository (`docs/MODEL_PROVENANCE.md`).
   - [ ] The hash matches the upstream publisher's published hash
         byte-for-byte.

2. **Unpinned entries** (sha256 == `UNVERIFIED_SHA256`):
   - [ ] `license: None` is set.
   - [ ] Listing `license: Some(_)` for an unpinned entry is
         **forbidden** — the audit fails closed on this case
         (the license is meaningless until the hash lands).
   - [ ] A follow-up issue links the upstream publisher that will
         publish the hash + license pair.

## Audit function

The canonical audit is `astroforge_ai::verify_catalog_audit()`. It
returns:

- `Ok(())` — every entry is contract-clean.
- `Err(Vec<CatalogAuditGap>)` — each gap row identifies the offending
  entry id + sha256 + reason.

The function is callable from startup logs, CI smoke tests, and the
DP#4 close-out PR workflow. It does not perform network I/O; the
catalog is the static `CATALOG_MODELS` table.

## Closing a real-catalog entry

When upstream publishes a hash + license for one of the unpinned
entries (e.g., SwinIR-denoise-astro):

1. Update the `sha256` field from `UNVERIFIED_SHA256` to the
   real hash.
2. Update the `license` field from `None` to
   `Some(LicenseSpdx::Apache20)` (or whichever SPDX applies).
3. Add a `MODEL_PROVENANCE.md` row with the source URL +
   publication date + license verification reference.
4. Run `cargo test -p astroforge-ai` — the audit test passes iff
   the entry is contract-clean.
5. Open a PR. CI runs `verify_catalog_audit()` implicitly via the
   `catalog_audit_passes_for_current_registry` test.

## Open DP#4 entries

| Entry id | Status | License | Hash source |
|---|---|---|---|
| swinir-denoise-astro | Unpinned | None (Apache-2.0 expected) | Upstream publishing pending |
| swinir-sr-astro-2x | Unpinned | None (Apache-2.0 expected) | Upstream publishing pending |
| swin2sr-dejpeg | Unpinned | None | Upstream publishing pending |
| star-seg-v1 | Unpinned | None | Upstream publishing pending |
| cloud-score-v1 | Unpinned | None | Upstream publishing pending |
| color-cal-net | Unpinned | None | Upstream publishing pending |
| trail-lama-tiny | Unpinned | None | Upstream publishing pending |

Each of these is a one-line table update once the upstream publisher
publishes a hash + license pair.

## History

- **2026-09-12** — DP#4 fail-closed machinery (`OnnxEngine::open_catalog`
  digest-pinning) shipped in PR #311.
- **2026-09-12** — DP#4 license audit + `LicenseSpdx` + GPU providers
  feature plumbing shipped in PR #315 (slice CD).