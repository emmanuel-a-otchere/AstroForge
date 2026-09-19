//! CR-07 §29.2: cached difference images integration tests.
//!
//! Pins the contract for `DiffCache`:
//!
//! - Cache hit returns a reference to a buffer that is
//!   byte-equivalent to a direct `compute_diff` call with
//!   the same inputs.
//! - Cache miss populates the cache.
//! - Different `gain` values produce different cache entries
//!   (matters for `Amplified` mode).
//! - Different `version_a_id` / `version_b_id` produce
//!   different cache entries.
//! - Different `DiffKind` produce different cache entries.
//! - `invalidate_version` drops every entry where
//!   `version_id` appears on either side of the pair.
//! - `clear` drops every entry.
//! - Stats: `hits` + `misses` track cache effectiveness.
//! - The cache does NOT mutate the input buffers.

use astroforge_core::diff_cache::{DiffCache, DiffCacheKey, DiffCacheStats};
use astroforge_core::difference::{compute_diff, DiffKind};

// ─── helpers ────────────────────────────────────────────────

/// Build a deterministic RGBA8 buffer of `width × height`
/// pixels with `width * 4 * height` bytes total.
fn rgba_grid(width: u32, height: u32, seed: u8) -> Vec<u8> {
    let mut out = Vec::with_capacity((width * height * 4) as usize);
    for i in 0..(width * height) {
        let v = (i as u8).wrapping_add(seed);
        out.push(v);
        out.push(v.wrapping_add(7));
        out.push(v.wrapping_mul(3));
        out.push(255);
    }
    out
}

/// Standard fixture pair used by the hit/miss/invalidation
/// tests.
fn fixture_pair() -> (Vec<u8>, Vec<u8>, u32) {
    let a = rgba_grid(8, 4, 0);
    let b = rgba_grid(8, 4, 17);
    (a, b, 8)
}

// ─── §29.2.1: cache hit / miss + byte-equivalence ─────────

#[test]
fn cache_miss_populates_and_returns_correct_buffer() {
    let (a, b, width) = fixture_pair();
    let mut cache = DiffCache::new();
    assert!(cache.is_empty());
    let key = DiffCacheKey::new("v-a", "v-b", DiffKind::Absolute, 1.0);
    let cached = cache
        .get_or_compute(key.clone(), &a, &b, 1.0, width)
        .clone();
    let direct = compute_diff(DiffKind::Absolute, &a, &b, 1.0, width);
    assert_eq!(
        cached, direct,
        "cached miss-populated buffer must match direct compute_diff"
    );
    assert_eq!(cache.len(), 1);
    assert_eq!(cache.stats().misses, 1);
    assert_eq!(cache.stats().hits, 0);
}

#[test]
fn cache_hit_returns_identical_buffer() {
    let (a, b, width) = fixture_pair();
    let mut cache = DiffCache::new();
    let key = DiffCacheKey::new("v-a", "v-b", DiffKind::Absolute, 1.0);
    // First call: miss + populate.
    let _ = cache
        .get_or_compute(key.clone(), &a, &b, 1.0, width)
        .clone();
    // Second call: hit, same buffer.
    let cached = cache
        .get_or_compute(key.clone(), &a, &b, 1.0, width)
        .clone();
    let direct = compute_diff(DiffKind::Absolute, &a, &b, 1.0, width);
    assert_eq!(cached, direct);
    assert_eq!(cache.len(), 1);
    assert_eq!(cache.stats().hits, 1);
    assert_eq!(cache.stats().misses, 1);
}

#[test]
fn cache_hit_does_not_recompute() {
    // Negative-control: the cache hit must not depend on
    // the input buffers. If we call get_or_compute with
    // the same key but mutated inputs, the cached buffer
    // (from the original inputs) should be returned.
    let (a, _b, width) = fixture_pair();
    let mut b = vec![0u8; a.len()];
    let mut cache = DiffCache::new();
    let key = DiffCacheKey::new("v-a", "v-b", DiffKind::Absolute, 1.0);
    let _ = cache
        .get_or_compute(key.clone(), &a, &b, 1.0, width)
        .clone();
    // Mutate b to all-255. Cache hit should return the
    // buffer computed from the original (zero) b, not
    // the new (all-255) b.
    for v in b.iter_mut() {
        *v = 255;
    }
    let cached = cache
        .get_or_compute(key.clone(), &a, &b, 1.0, width)
        .clone();
    let direct_with_zero_b = compute_diff(DiffKind::Absolute, &a, &vec![0u8; a.len()], 1.0, width);
    assert_eq!(cached, direct_with_zero_b);
}

// ─── §29.2.2: cache key independence ──────────────────────

#[test]
fn different_version_a_id_produces_different_entries() {
    let (a, b, width) = fixture_pair();
    let mut cache = DiffCache::new();
    let _ = cache
        .get_or_compute(
            DiffCacheKey::new("v-a", "v-b", DiffKind::Absolute, 1.0),
            &a,
            &b,
            1.0,
            width,
        )
        .clone();
    let _ = cache
        .get_or_compute(
            DiffCacheKey::new("v-x", "v-b", DiffKind::Absolute, 1.0),
            &a,
            &b,
            1.0,
            width,
        )
        .clone();
    assert_eq!(cache.len(), 2);
}

#[test]
fn different_version_b_id_produces_different_entries() {
    let (a, b, width) = fixture_pair();
    let mut cache = DiffCache::new();
    let _ = cache
        .get_or_compute(
            DiffCacheKey::new("v-a", "v-b", DiffKind::Absolute, 1.0),
            &a,
            &b,
            1.0,
            width,
        )
        .clone();
    let _ = cache
        .get_or_compute(
            DiffCacheKey::new("v-a", "v-x", DiffKind::Absolute, 1.0),
            &a,
            &b,
            1.0,
            width,
        )
        .clone();
    assert_eq!(cache.len(), 2);
}

#[test]
fn different_diff_mode_produces_different_entries() {
    let (a, b, width) = fixture_pair();
    let mut cache = DiffCache::new();
    let _ = cache
        .get_or_compute(
            DiffCacheKey::new("v-a", "v-b", DiffKind::Absolute, 1.0),
            &a,
            &b,
            1.0,
            width,
        )
        .clone();
    let _ = cache
        .get_or_compute(
            DiffCacheKey::new("v-a", "v-b", DiffKind::Signed, 1.0),
            &a,
            &b,
            1.0,
            width,
        )
        .clone();
    let _ = cache
        .get_or_compute(
            DiffCacheKey::new("v-a", "v-b", DiffKind::Amplified, 1.0),
            &a,
            &b,
            1.0,
            width,
        )
        .clone();
    let _ = cache
        .get_or_compute(
            DiffCacheKey::new("v-a", "v-b", DiffKind::Structural, 1.0),
            &a,
            &b,
            1.0,
            width,
        )
        .clone();
    assert_eq!(cache.len(), 4);
}

#[test]
fn different_gain_produces_different_entries() {
    // Different gain values produce different cache entries
    // even when the mode (Amplified) is the only thing that
    // uses gain. This matches `compute_diff`'s behavior:
    // gain only affects the Amplified mode.
    let (a, b, width) = fixture_pair();
    let mut cache = DiffCache::new();
    let _ = cache
        .get_or_compute(
            DiffCacheKey::new("v-a", "v-b", DiffKind::Amplified, 1.0),
            &a,
            &b,
            1.0,
            width,
        )
        .clone();
    let _ = cache
        .get_or_compute(
            DiffCacheKey::new("v-a", "v-b", DiffKind::Amplified, 2.0),
            &a,
            &b,
            2.0,
            width,
        )
        .clone();
    let _ = cache
        .get_or_compute(
            DiffCacheKey::new("v-a", "v-b", DiffKind::Amplified, 4.0),
            &a,
            &b,
            4.0,
            width,
        )
        .clone();
    assert_eq!(cache.len(), 3);
}

#[test]
fn same_gain_with_nan_produces_different_entries_than_zero() {
    // f32::to_bits distinguishes NaN from zero. This test
    // pins the cache key correctness: NaN-gain and 0.0-gain
    // are different cache entries.
    let (a, b, width) = fixture_pair();
    let mut cache = DiffCache::new();
    let key_zero = DiffCacheKey::new("v-a", "v-b", DiffKind::Amplified, 0.0);
    let key_nan = DiffCacheKey::new("v-a", "v-b", DiffKind::Amplified, f32::NAN);
    assert_ne!(key_zero, key_nan);
    let _ = cache.get_or_compute(key_zero, &a, &b, 0.0, width).clone();
    let _ = cache
        .get_or_compute(key_nan, &a, &b, f32::NAN, width)
        .clone();
    assert_eq!(cache.len(), 2);
}

// ─── §29.2.3: invalidation ────────────────────────────────

#[test]
fn invalidate_version_drops_entries_with_matching_id() {
    let (a, b, width) = fixture_pair();
    let mut cache = DiffCache::new();
    // Populate with 4 entries: 2 involving v-a, 2 not.
    let _ = cache
        .get_or_compute(
            DiffCacheKey::new("v-a", "v-b", DiffKind::Absolute, 1.0),
            &a,
            &b,
            1.0,
            width,
        )
        .clone();
    let _ = cache
        .get_or_compute(
            DiffCacheKey::new("v-b", "v-a", DiffKind::Absolute, 1.0),
            &a,
            &b,
            1.0,
            width,
        )
        .clone();
    let _ = cache
        .get_or_compute(
            DiffCacheKey::new("v-c", "v-d", DiffKind::Absolute, 1.0),
            &a,
            &b,
            1.0,
            width,
        )
        .clone();
    let _ = cache
        .get_or_compute(
            DiffCacheKey::new("v-e", "v-f", DiffKind::Absolute, 1.0),
            &a,
            &b,
            1.0,
            width,
        )
        .clone();
    assert_eq!(cache.len(), 4);
    let dropped = cache.invalidate_version("v-a");
    assert_eq!(dropped, 2, "must drop the two entries with v-a");
    assert_eq!(cache.len(), 2);
    // The remaining entries (v-c/v-d and v-e/v-f) are
    // untouched. Subsequent get_or_compute for them is a
    // hit, not a miss. Stats: 4 misses (the initial populate)
    // + 2 hits (the post-invalidate re-lookups).
    let _ = cache
        .get_or_compute(
            DiffCacheKey::new("v-c", "v-d", DiffKind::Absolute, 1.0),
            &a,
            &b,
            1.0,
            width,
        )
        .clone();
    let _ = cache
        .get_or_compute(
            DiffCacheKey::new("v-e", "v-f", DiffKind::Absolute, 1.0),
            &a,
            &b,
            1.0,
            width,
        )
        .clone();
    assert_eq!(
        cache.stats().misses,
        4,
        "all 4 entries were populated as misses"
    );
    assert_eq!(
        cache.stats().hits,
        2,
        "post-invalidate re-lookups for the 2 surviving entries are hits"
    );
}

#[test]
fn invalidate_version_with_no_matches_returns_zero() {
    let (a, b, width) = fixture_pair();
    let mut cache = DiffCache::new();
    let _ = cache
        .get_or_compute(
            DiffCacheKey::new("v-a", "v-b", DiffKind::Absolute, 1.0),
            &a,
            &b,
            1.0,
            width,
        )
        .clone();
    let dropped = cache.invalidate_version("nonexistent");
    assert_eq!(dropped, 0);
    assert_eq!(cache.len(), 1);
}

#[test]
fn invalidate_version_after_invalidation_forces_recompute() {
    let (a, b, width) = fixture_pair();
    let mut cache = DiffCache::new();
    let key = DiffCacheKey::new("v-a", "v-b", DiffKind::Absolute, 1.0);
    let _ = cache
        .get_or_compute(key.clone(), &a, &b, 1.0, width)
        .clone();
    assert_eq!(cache.stats().misses, 1);
    assert_eq!(cache.stats().hits, 0);
    cache.invalidate_version("v-a");
    // After invalidation, the next get_or_compute is a miss.
    let _ = cache
        .get_or_compute(key.clone(), &a, &b, 1.0, width)
        .clone();
    assert_eq!(cache.stats().misses, 2);
    assert_eq!(cache.stats().hits, 0);
}

#[test]
fn clear_drops_all_entries_preserves_stats() {
    let (a, b, width) = fixture_pair();
    let mut cache = DiffCache::new();
    let _ = cache
        .get_or_compute(
            DiffCacheKey::new("v-a", "v-b", DiffKind::Absolute, 1.0),
            &a,
            &b,
            1.0,
            width,
        )
        .clone();
    let _ = cache
        .get_or_compute(
            DiffCacheKey::new("v-a", "v-b", DiffKind::Absolute, 1.0),
            &a,
            &b,
            1.0,
            width,
        )
        .clone();
    assert_eq!(cache.stats().misses, 1);
    assert_eq!(cache.stats().hits, 1);
    cache.clear();
    assert!(cache.is_empty());
    // Stats are preserved.
    assert_eq!(cache.stats().misses, 1);
    assert_eq!(cache.stats().hits, 1);
}

// ─── §29.2.4: stats tracking ──────────────────────────────

#[test]
fn stats_hit_rate_zero_when_no_lookups() {
    let stats = DiffCacheStats::default();
    assert_eq!(stats.hit_rate(), 0.0);
}

#[test]
fn stats_hit_rate_one_when_only_hits() {
    let stats = DiffCacheStats { hits: 5, misses: 0 };
    assert_eq!(stats.hit_rate(), 1.0);
}

#[test]
fn stats_hit_rate_one_half_when_mixed() {
    let stats = DiffCacheStats { hits: 1, misses: 1 };
    assert!((stats.hit_rate() - 0.5).abs() < 1e-9);
}

#[test]
fn reset_stats_zeroes_counters_but_preserves_entries() {
    let (a, b, width) = fixture_pair();
    let mut cache = DiffCache::new();
    let _ = cache
        .get_or_compute(
            DiffCacheKey::new("v-a", "v-b", DiffKind::Absolute, 1.0),
            &a,
            &b,
            1.0,
            width,
        )
        .clone();
    let _ = cache
        .get_or_compute(
            DiffCacheKey::new("v-a", "v-b", DiffKind::Absolute, 1.0),
            &a,
            &b,
            1.0,
            width,
        )
        .clone();
    assert_eq!(cache.stats().hits, 1);
    assert_eq!(cache.stats().misses, 1);
    cache.reset_stats();
    assert_eq!(cache.stats().hits, 0);
    assert_eq!(cache.stats().misses, 0);
    assert_eq!(cache.len(), 1, "entries are preserved across reset_stats");
}

// ─── §29.2.5: cache key API surface ────────────────────────

#[test]
fn cache_key_equality_depends_on_all_fields() {
    let k1 = DiffCacheKey::new("v-a", "v-b", DiffKind::Absolute, 1.0);
    let k2 = DiffCacheKey::new("v-a", "v-b", DiffKind::Absolute, 1.0);
    assert_eq!(k1, k2);
    // Different version_a_id
    let k3 = DiffCacheKey::new("v-x", "v-b", DiffKind::Absolute, 1.0);
    assert_ne!(k1, k3);
    // Different mode
    let k4 = DiffCacheKey::new("v-a", "v-b", DiffKind::Signed, 1.0);
    assert_ne!(k1, k4);
    // Different gain
    let k5 = DiffCacheKey::new("v-a", "v-b", DiffKind::Absolute, 2.0);
    assert_ne!(k1, k5);
}

#[test]
fn cache_key_clone_preserves_equality() {
    let k1 = DiffCacheKey::new("v-a", "v-b", DiffKind::Absolute, 1.0);
    let k2 = k1.clone();
    assert_eq!(k1, k2);
    assert_eq!(k1.gain_bits, k2.gain_bits);
}

#[test]
fn cache_key_hash_is_consistent_with_eq() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let k1 = DiffCacheKey::new("v-a", "v-b", DiffKind::Absolute, 1.0);
    let k2 = DiffCacheKey::new("v-a", "v-b", DiffKind::Absolute, 1.0);
    let mut h1 = DefaultHasher::new();
    let mut h2 = DefaultHasher::new();
    k1.hash(&mut h1);
    k2.hash(&mut h2);
    assert_eq!(h1.finish(), h2.finish(), "Hash must agree with Eq");
}

// ─── §29.2.6: with_capacity constructor ──────────────────

#[test]
fn with_capacity_creates_empty_cache() {
    let cache = DiffCache::with_capacity(100);
    assert!(cache.is_empty());
    assert_eq!(cache.len(), 0);
    assert_eq!(cache.stats().hits, 0);
    assert_eq!(cache.stats().misses, 0);
}

// ─── §29.2.7: inputs are not mutated ──────────────────────

#[test]
fn get_or_compute_does_not_mutate_input_buffers() {
    // The cache must not modify the caller's input buffers.
    // (compute_diff only reads them; this test pins that
    // contract.)
    let a = rgba_grid(8, 4, 0);
    let b = rgba_grid(8, 4, 17);
    let a_before = a.clone();
    let b_before = b.clone();
    let mut cache = DiffCache::new();
    let _ = cache
        .get_or_compute(
            DiffCacheKey::new("v-a", "v-b", DiffKind::Absolute, 1.0),
            &a,
            &b,
            1.0,
            8,
        )
        .clone();
    assert_eq!(a, a_before, "a must be unchanged after get_or_compute");
    assert_eq!(b, b_before, "b must be unchanged after get_or_compute");
}
