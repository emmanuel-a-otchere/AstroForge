//! CR-07 §32.2: visual regression tests for the diff renderer.
//!
//! The audit lists §32 "visual regression" as covering
//! "split alignment, blink consistency, difference rendering,
//! overlay accuracy". Of these four sub-modes:
//!
//! - **split alignment**: Svelte canvas-level alignment of two
//!   images at a slider position; not Rust-testable.
//! - **blink consistency**: Svelte canvas-level A/B toggle at
//!   configurable intervals; not Rust-testable.
//! - **overlay accuracy**: Svelte canvas-level alpha blending;
//!   not Rust-testable.
//! - **difference rendering**: the Rust `compute_diff` function
//!   in `crates/astroforge-core/src/difference.rs` produces the
//!   pixel-exact absolute / signed / amplified / structural
//!   diffs that the frontend renders. **This is Rust-testable.**
//!
//! These integration tests pin `compute_diff` against
//! deterministic RGBA8 fixtures (constant fields, gradient
//! fields, edge patterns) and assert pixel-exact expected
//! output. They are the visual-regression equivalent of the
//! §32.1 metric-validation tests: small, deterministic, and
//! pinned to the canonical Rust reference.
//!
//! The split / blink / overlay sub-modes are out of scope for
//! this slice. They live in `CompareTools.svelte` and require
//! a DOM-rendering test runner (e.g. Playwright) that the
//! codebase doesn't ship yet. Documented as a future slice.

use astroforge_core::difference::{compute_diff, DiffKind};

/// 4-byte RGBA tuple. RGBA8 layout is the wire format
/// `compute_diff` consumes + emits.
type Rgba = (u8, u8, u8, u8);

/// Build a flat RGBA8 buffer filled with a single colour.
fn flat_rgba(width: u32, height: u32, colour: Rgba) -> Vec<u8> {
    let mut buf = Vec::with_capacity((width * height * 4) as usize);
    for _ in 0..(width * height) {
        buf.extend_from_slice(&[colour.0, colour.1, colour.2, colour.3]);
    }
    buf
}

/// Build a flat RGBA8 buffer filled with a horizontal gradient:
/// pixel(x, y) = (start_r + (x / (width-1)) * (end_r - start_r), ...).
fn gradient_rgba(width: u32, height: u32, start: Rgba, end: Rgba) -> Vec<u8> {
    let mut buf = Vec::with_capacity((width * height * 4) as usize);
    let denom = (width - 1).max(1) as f32;
    for y in 0..height {
        for x in 0..width {
            let t = x as f32 / denom;
            let r = (start.0 as f32 + t * (end.0 as f32 - start.0 as f32)) as u8;
            let g = (start.1 as f32 + t * (end.1 as f32 - start.1 as f32)) as u8;
            let b = (start.2 as f32 + t * (end.2 as f32 - start.2 as f32)) as u8;
            buf.extend_from_slice(&[r, g, b, start.3]);
            let _ = y;
        }
    }
    buf
}

/// Read a single pixel's RGBA from a flat buffer.
fn pixel(buf: &[u8], x: u32, y: u32, width: u32) -> (u8, u8, u8, u8) {
    let i = ((y * width + x) * 4) as usize;
    (buf[i], buf[i + 1], buf[i + 2], buf[i + 3])
}

// ─── §32.2.1: Absolute diff pixel-exactness ─────────────────

#[test]
fn absolute_diff_is_zero_for_identical_inputs() {
    let img = gradient_rgba(8, 8, (0, 0, 0, 255), (255, 255, 255, 255));
    let out = compute_diff(DiffKind::Absolute, &img, &img, 1.0, 8);
    for (i, chunk) in out.as_chunks::<4>().0.iter().enumerate() {
        let _ = i;
        // Alpha preserved; RGB all zero (no diff).
        assert_eq!(chunk[0], 0, "R should be zero on identical input");
        assert_eq!(chunk[1], 0, "G should be zero on identical input");
        assert_eq!(chunk[2], 0, "B should be zero on identical input");
        assert_eq!(chunk[3], 255, "alpha should be preserved");
    }
}

#[test]
fn absolute_diff_matches_hand_computation_pixel_exact() {
    // 4×1 RGBA8 image. a = [50, 60, 70, 255, 100, 110, 120, 255]
    //                    b = [200, 30, 75, 255, 50, 200, 100, 255]
    // Expected abs-diff:
    //   pixel 0: |50-200|=150, |60-30|=30, |70-75|=5, alpha=255
    //   pixel 1: |100-50|=50, |110-200|=90, |120-100|=20, alpha=255
    let a = vec![50, 60, 70, 255, 100, 110, 120, 255];
    let b = vec![200, 30, 75, 255, 50, 200, 100, 255];
    let out = compute_diff(DiffKind::Absolute, &a, &b, 1.0, 4);
    assert_eq!(pixel(&out, 0, 0, 4), (150, 30, 5, 255));
    assert_eq!(pixel(&out, 1, 0, 4), (50, 90, 20, 255));
}

#[test]
fn absolute_diff_clamps_to_255_at_extremes() {
    // |0 - 255| = 255 (max). |255 - 0| = 255 (max). Both
    // should produce 255 in the diff, NOT overflow.
    let a = vec![0u8, 0, 0, 255];
    let b = vec![255u8, 255, 255, 255];
    let out = compute_diff(DiffKind::Absolute, &a, &b, 1.0, 1);
    assert_eq!(pixel(&out, 0, 0, 1), (255, 255, 255, 255));
}

#[test]
fn absolute_diff_preserves_alpha_byte() {
    // The alpha byte (index 3 of each RGBA quad) must always
    // be 255 in the output, regardless of the input alpha.
    // This pins the "alpha stays opaque" promise in
    // `render_absolute`.
    let a = vec![100u8, 100, 100, 0, 50, 50, 50, 128];
    let b = vec![0u8, 0, 0, 0, 100, 100, 100, 64];
    let out = compute_diff(DiffKind::Absolute, &a, &b, 1.0, 2);
    // Diff channels:
    //   pixel 0: R=100, G=100, B=100; alpha forced to 255
    //   pixel 1: R=50, G=50, B=50; alpha forced to 255
    assert_eq!(pixel(&out, 0, 0, 2), (100, 100, 100, 255));
    assert_eq!(pixel(&out, 1, 0, 2), (50, 50, 50, 255));
}

// ─── §32.2.2: Signed diff pixel-exactness ───────────────────

#[test]
fn signed_diff_is_midpoint_for_identical_inputs() {
    // When a == b, signed diff is (0 + 128) = 128 (mid-grey).
    let img = flat_rgba(8, 8, (100, 100, 100, 255));
    let out = compute_diff(DiffKind::Signed, &img, &img, 1.0, 8);
    for chunk in out.as_chunks::<4>().0.iter() {
        assert_eq!(
            chunk[0], 128,
            "R should be 128 (mid-grey) on identical input"
        );
        assert_eq!(
            chunk[1], 128,
            "G should be 128 (mid-grey) on identical input"
        );
        assert_eq!(
            chunk[2], 128,
            "B should be 128 (mid-grey) on identical input"
        );
        assert_eq!(chunk[3], 255);
    }
}

#[test]
fn signed_diff_brighter_a_pushes_above_128() {
    // a brighter than b: (a - b) > 0, so output > 128.
    // a = 200, b = 50, diff = 150 + 128 = 278 -> clamped to 255.
    let a = vec![200u8, 200, 200, 255];
    let b = vec![50u8, 50, 50, 255];
    let out = compute_diff(DiffKind::Signed, &a, &b, 1.0, 1);
    assert_eq!(pixel(&out, 0, 0, 1), (255, 255, 255, 255));
}

#[test]
fn signed_diff_brighter_b_pushes_below_128() {
    // a darker than b: (a - b) < 0, so output < 128.
    // a = 50, b = 200, diff = -150 + 128 = -22 -> clamped to 0.
    let a = vec![50u8, 50, 50, 255];
    let b = vec![200u8, 200, 200, 255];
    let out = compute_diff(DiffKind::Signed, &a, &b, 1.0, 1);
    assert_eq!(pixel(&out, 0, 0, 1), (0, 0, 0, 255));
}

#[test]
fn signed_diff_hand_computed_pixel_exact() {
    // a = 100, b = 60: diff = 40 + 128 = 168.
    let a = vec![100u8, 80, 120, 255];
    let b = vec![60u8, 100, 80, 255];
    let out = compute_diff(DiffKind::Signed, &a, &b, 1.0, 1);
    assert_eq!(pixel(&out, 0, 0, 1), (168, 108, 168, 255));
}

// ─── §32.2.3: Amplified diff pixel-exactness ───────────────

#[test]
fn amplified_diff_with_unit_gain_matches_absolute() {
    // gain = 1.0 should produce the same result as
    // DiffKind::Absolute (the "default gain" semantics).
    let a = vec![50u8, 60, 70, 255, 100, 110, 120, 255];
    let b = vec![200u8, 30, 75, 255, 50, 200, 100, 255];
    let abs = compute_diff(DiffKind::Absolute, &a, &b, 1.0, 4);
    let amp = compute_diff(DiffKind::Amplified, &a, &b, 1.0, 4);
    assert_eq!(abs, amp, "gain=1.0 amplified should match absolute");
}

#[test]
fn amplified_diff_scales_correctly_pixel_exact() {
    // |100 - 50| = 50; gain=2.0 -> 100 (still in range).
    // |100 - 0| = 100; gain=2.0 -> 200.
    // |100 - 0| = 100; gain=3.0 -> 300 -> clamped to 255.
    let a = vec![100u8, 100, 100, 255];
    let b = vec![50u8, 0, 100, 255];
    let out_2x = compute_diff(DiffKind::Amplified, &a, &b, 2.0, 1);
    assert_eq!(pixel(&out_2x, 0, 0, 1), (100, 200, 0, 255));
    let out_3x = compute_diff(DiffKind::Amplified, &a, &b, 3.0, 1);
    assert_eq!(pixel(&out_3x, 0, 0, 1), (150, 255, 0, 255));
}

#[test]
fn amplified_diff_handles_non_finite_gain_safely() {
    // The renderer coerces non-finite or non-positive gain
    // to 1.0 (defensive default). This test pins that
    // behaviour so a future change doesn't silently let NaN
    // propagate into the diff buffer.
    let a = vec![100u8, 100, 100, 255];
    let b = vec![50u8, 50, 50, 255];
    let nan_out = compute_diff(DiffKind::Amplified, &a, &b, f32::NAN, 1);
    let unit_out = compute_diff(DiffKind::Amplified, &a, &b, 1.0, 1);
    assert_eq!(
        nan_out, unit_out,
        "NaN gain should coerce to 1.0 (defensive default)"
    );
    let zero_out = compute_diff(DiffKind::Amplified, &a, &b, 0.0, 1);
    assert_eq!(
        zero_out, unit_out,
        "zero gain should coerce to 1.0 (defensive default)"
    );
}

// ─── §32.2.4: Structural diff edge behaviour ───────────────

#[test]
fn structural_diff_is_zero_for_uniform_image() {
    // Uniform image: no edges anywhere, so structural diff
    // is zero everywhere.
    let img = flat_rgba(8, 8, (100, 100, 100, 255));
    let out = compute_diff(DiffKind::Structural, &img, &img, 1.0, 8);
    for chunk in out.as_chunks::<4>().0.iter() {
        // The structural path falls back to absolute for
        // uniform inputs, which gives zero for identical
        // images.
        assert_eq!(chunk[0], 0);
        assert_eq!(chunk[1], 0);
        assert_eq!(chunk[2], 0);
        assert_eq!(chunk[3], 255);
    }
}

#[test]
fn structural_diff_falls_back_to_absolute_for_tiny_images() {
    // The structural path requires width >= 2 AND at least
    // 2 rows. For a 1×1 image (width=1), the path falls
    // back to absolute. Pin that behaviour.
    let a = vec![100u8, 100, 100, 255];
    let b = vec![50u8, 50, 50, 255];
    let abs = compute_diff(DiffKind::Absolute, &a, &b, 1.0, 1);
    let structural = compute_diff(DiffKind::Structural, &a, &b, 1.0, 1);
    assert_eq!(
        structural, abs,
        "1×1 structural should fall back to absolute path"
    );
}

#[test]
fn structural_diff_detects_edge_in_a_not_b() {
    // 4×4 image where A has a vertical edge at the centre
    // column (gradient from 0 to 255 across the centre)
    // and B is uniform. The structural diff should
    // highlight the column where the edge lives.
    //
    // The Sobel-style neighbour diff uses `left` and `up`
    // neighbours. For a vertical edge at the centre column
    // (x=1 → x=2 transition), the interior pixel at x=2
    // has left=1 (which is 0 in the gradient) and is
    // itself 255, so the abs-diff to the left neighbour
    // is 255 → the structural diff at (2, y>=1) is
    // non-zero. The pixel at x=1 has left=0 and is
    // itself 0, so its structural diff is 0.
    let mut a = vec![0u8; 4 * 4 * 4];
    for y in 0..4 {
        for x in 0..4 {
            let i = (y * 4 + x) * 4;
            // Step at x=2: pixels 0,1 are 0; pixels 2,3 are 255.
            let v = if x < 2 { 0 } else { 255 };
            a[i] = v;
            a[i + 1] = v;
            a[i + 2] = v;
            a[i + 3] = 255;
        }
    }
    let b = flat_rgba(4, 4, (128, 128, 128, 255));
    let out = compute_diff(DiffKind::Structural, &a, &b, 1.0, 4);
    // Interior pixel at (x=2, y=1): A has left=0 and self=255
    // (edge!), B has left=128 and self=128 (no edge).
    // Structural diff at this position should be non-zero.
    let at_edge = pixel(&out, 2, 1, 4);
    assert!(
        at_edge.0 > 0 || at_edge.1 > 0 || at_edge.2 > 0,
        "structural diff at the edge pixel (x=2) should be non-zero, got {:?}",
        at_edge
    );
    // Pixel at (x=3, y=1): A has left=255 and self=255 (flat);
    // no edge. Diff should be 0.
    let away_from_edge = pixel(&out, 3, 1, 4);
    assert_eq!(
        away_from_edge,
        (0, 0, 0, 255),
        "structural diff away from the edge should be zero"
    );
}

// ─── §32.2.5: Cross-mode determinism ────────────────────────

#[test]
fn all_diff_modes_are_deterministic_for_same_input() {
    let a = gradient_rgba(16, 16, (0, 0, 0, 255), (200, 200, 200, 255));
    let b = gradient_rgba(16, 16, (50, 50, 50, 255), (255, 255, 255, 255));
    for kind in DiffKind::all() {
        let out_1 = compute_diff(kind, &a, &b, 2.5, 16);
        let out_2 = compute_diff(kind, &a, &b, 2.5, 16);
        assert_eq!(
            out_1, out_2,
            "diff mode {:?} must be deterministic for the same input",
            kind
        );
    }
}

#[test]
fn all_diff_modes_preserve_alpha_byte_to_255() {
    // Every diff mode (per `render_*` doc-contracts) must
    // force alpha to 255 in the output, regardless of the
    // input alpha. The "alpha stays opaque" promise is a
    // contract the frontend depends on (it composites the
    // diff buffer onto a canvas where transparency would
    // show through to the page background).
    let a = vec![100u8, 100, 100, 0, 50, 50, 50, 200];
    let b = vec![50u8, 50, 50, 0, 200, 200, 200, 100];
    for kind in [
        DiffKind::Absolute,
        DiffKind::Signed,
        DiffKind::Amplified,
        DiffKind::Structural,
    ] {
        let out = compute_diff(kind, &a, &b, 1.0, 2);
        assert_eq!(out[3], 255, "absolute alpha must be 255");
        assert_eq!(out[7], 255, "absolute alpha must be 255");
    }
}
