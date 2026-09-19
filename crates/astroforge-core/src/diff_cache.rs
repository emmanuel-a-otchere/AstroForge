// CR-07 §29.2: cached difference images.
//
// Wraps `compute_diff` with a content-addressed cache
// keyed by `(version_a_id, version_b_id, diff_mode, gain)`.
//
// The cache lets the comparison surface re-render a
// previously-computed difference buffer in O(1) instead
// of recomputing the O(WHC) per-mode walk on every toggle
// click. The win is most visible when the user toggles
// between Absolute / Signed / Amplified / Structural modes
// for the same pair of versions: each toggle is a hash
// lookup, not a full recompute.
//
// The cache is **single-threaded by construction**: the
// borrow signature on `get_or_compute` returns `&Vec<u8>`,
// so the caller cannot mutate the cache while holding a
// reference to the cached buffer. Callers that need to
// share the cache across threads should wrap it in a
// `Mutex` (or `RwLock` if reads dominate). The §29.2
// slice does not ship a thread-safe wrapper; that is a
// follow-on concern if the Tauri IPC layer needs it.
//
// Cache key design:
//
// - `version_a_id` and `version_b_id` are stable per
//   `ImageVersion` row in the domain store. They are
//   passed by the caller (typically the IPC handler
//   that loaded the version row).
// - `diff_mode` is the `DiffKind` enum.
// - `gain` only matters for `DiffKind::Amplified`; for
//   the other modes, the gain parameter to `compute_diff`
//   is ignored. We include `gain` in the key anyway so
//   the cache is uniformly keyed on all inputs; an
//   Amplified lookup with gain=2 will miss on a previous
//   Amplified lookup with gain=1.
//
// Cache invalidation:
//
// - `invalidate_version(version_id)` drops every cache
//   entry where either side of the pair matches. This
//   is the operation to call when a version's primary
//   artifact changes (e.g. the user re-runs an
//   enhancement that updates the underlying TIFF).
// - The cache does NOT watch the filesystem; the caller
//   is responsible for invoking `invalidate_version` at
//   the appropriate time. The §29.2 slice does not wire
//   the invalidation into the IPC layer; that's a
//   follow-on concern.
//
// Stats:
//
// - `hits` + `misses` counters track cache effectiveness.
//   `hit_rate()` returns `hits / (hits + misses)` as a
//   fraction in [0, 1]. A high hit rate is the desired
//   steady state; a low hit rate indicates the cache
//   is not earning its memory cost and should be sized
//   down.

use crate::difference::{compute_diff, DiffKind};
use std::collections::HashMap;

/// Cache key. `(version_a_id, version_b_id, mode, gain)`.
/// Two requests with different gains produce different
/// cache entries (matters for `Amplified` mode).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DiffCacheKey {
    pub version_a_id: String,
    pub version_b_id: String,
    pub mode: DiffKind,
    /// Stored as bits (`f32::to_bits`) so the key can
    /// derive `Eq + Hash` (`f32` itself does not).
    pub gain_bits: u32,
}

impl DiffCacheKey {
    pub fn new(
        version_a_id: impl Into<String>,
        version_b_id: impl Into<String>,
        mode: DiffKind,
        gain: f32,
    ) -> Self {
        Self {
            version_a_id: version_a_id.into(),
            version_b_id: version_b_id.into(),
            mode,
            gain_bits: gain.to_bits(),
        }
    }
}

/// Stats counters for the cache.
#[derive(Debug, Clone, Default)]
pub struct DiffCacheStats {
    pub hits: u64,
    pub misses: u64,
}

impl DiffCacheStats {
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

/// Single-threaded cache for `compute_diff` output.
/// Keyed by `(version_a_id, version_b_id, mode, gain)`.
#[derive(Debug, Default)]
pub struct DiffCache {
    entries: HashMap<DiffCacheKey, Vec<u8>>,
    stats: DiffCacheStats,
}

impl DiffCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a cache pre-sized for `capacity` entries.
    /// The HashMap will grow beyond this if more entries
    /// are inserted; the parameter is purely a hint to
    /// avoid rehashing during warmup.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: HashMap::with_capacity(capacity),
            stats: DiffCacheStats::default(),
        }
    }

    /// Number of cached entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Snapshot of the current stats.
    pub fn stats(&self) -> &DiffCacheStats {
        &self.stats
    }

    /// Reset the stats counters without dropping cached
    /// entries. Useful for per-render-window measurements.
    pub fn reset_stats(&mut self) {
        self.stats = DiffCacheStats::default();
    }

    /// Look up an entry; if miss, compute + insert. The
    /// returned reference borrows the cache, so the caller
    /// cannot mutate the cache while holding the reference.
    /// If you need to own the buffer, clone it.
    ///
    /// `a` + `b` are the RGBA8 source buffers (the caller's
    /// pixel data). `width` is the image width in pixels
    /// (used by `Structural` mode). `gain` is the per-mode
    /// gain factor (only used by `Amplified`).
    pub fn get_or_compute(
        &mut self,
        key: DiffCacheKey,
        a: &[u8],
        b: &[u8],
        gain: f32,
        width: u32,
    ) -> &Vec<u8> {
        if self.entries.contains_key(&key) {
            self.stats.hits += 1;
            return self.entries.get(&key).expect("just checked contains_key");
        }
        self.stats.misses += 1;
        let buf = compute_diff(key.mode, a, b, gain, width);
        self.entries.entry(key).or_insert(buf)
    }

    /// Drop every cache entry where `version_id` appears
    /// on either side of the pair. Used when a version's
    /// primary artifact changes.
    pub fn invalidate_version(&mut self, version_id: &str) -> usize {
        let before = self.entries.len();
        self.entries
            .retain(|k, _| k.version_a_id != version_id && k.version_b_id != version_id);
        before - self.entries.len()
    }

    /// Drop every cache entry. The stats counters are
    /// preserved.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}
