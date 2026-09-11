//! CR-06 P5 — User-defined masks.
//!
//! Per CR-06 §12, user masks cover brush / polygon /
//! gradient / radial inputs. P5 ships the four
//! constructors:
//!
//! - `brush` — a circular brush of `radius` pixels,
//!   centred at `(cx, cy)`, painting a soft falloff
//!   into the underlying canvas. The brush only adds
//!   to existing weights (`max`); subtract brushes
//!   ship in a future slice.
//! - `polygon` — a closed polygon shape with linear
//!   edge rasterisation. The result is a binary mask
//!   (inside = 1, outside = 0).
//! - `gradient` — a horizontal / vertical linear ramp
//!   from `start_weight` to `end_weight`.
//! - `radial` — a radial falloff from a centre point;
//!   `inner_radius` is fully included, `outer_radius`
//!   is the cutoff, with a smooth falloff between.
//!
//! All constructors return a fresh `Mask`; the caller
//! composites the result with the existing mask via
//! `composite::union` / `intersect` / `difference`.

use super::{Mask, MaskKind};

/// Paint a circular brush stroke into a fresh mask.
/// The returned mask has the brush weight at every
/// pixel within `radius` of `(cx, cy)` (with a soft
/// falloff to the edge) and zero elsewhere.
pub fn brush(width: u32, height: u32, cx: f32, cy: f32, radius: f32, intensity: f32) -> Mask {
    let mut mask = Mask::zeros(width, height, MaskKind::User, "brush".into());
    let r2 = (radius * radius).max(0.0001);
    let x_min = (cx - radius).max(0.0).floor() as i32;
    let x_max = (cx + radius).min(width as f32).ceil() as i32;
    let y_min = (cy - radius).max(0.0).floor() as i32;
    let y_max = (cy + radius).min(height as f32).ceil() as i32;
    for y in y_min..y_max {
        for x in x_min..x_max {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let d2 = dx * dx + dy * dy;
            if d2 >= r2 {
                continue;
            }
            let d = d2.sqrt();
            // Linear falloff from `intensity` at the
            // centre to 0 at the edge.
            let w = intensity * (1.0 - d / radius);
            mask.set(x as u32, y as u32, w);
        }
    }
    mask
}

/// Build a polygon mask. `points` is a list of `(x, y)`
/// vertices; the polygon is closed implicitly
/// (`points[0]` connects back to `points[-1]`). The
/// interior is filled with `1.0`; the exterior with
/// `0.0`. The rasteriser uses the scan-line
/// even-odd rule, which handles non-convex shapes
/// (the user can paint an L-shape without artefacts).
pub fn polygon(width: u32, height: u32, points: &[(f32, f32)]) -> Mask {
    let mut mask = Mask::zeros(width, height, MaskKind::User, "polygon".into());
    for y in 0..height {
        // Scan each row for x-intersections with the
        // polygon's edges.
        let mut xs: Vec<f32> = Vec::new();
        let n = points.len();
        if n < 3 {
            return mask;
        }
        for i in 0..n {
            let (x1, y1) = points[i];
            let (x2, y2) = points[(i + 1) % n];
            let y1f = y1;
            let y2f = y2;
            if (y1f <= y as f32 && y2f > y as f32) || (y2f <= y as f32 && y1f > y as f32) {
                let t = (y as f32 - y1f) / (y2f - y1f);
                let x = x1 + t * (x2 - x1);
                xs.push(x);
            }
        }
        xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        // Fill between pairs of intersections.
        let mut i = 0;
        while i + 1 < xs.len() {
            let x_start = xs[i].ceil() as i32;
            let x_end = xs[i + 1].floor() as i32;
            for x in x_start.max(0)..=x_end.min(width as i32 - 1) {
                mask.set(x as u32, y, 1.0);
            }
            i += 2;
        }
    }
    mask
}

/// Build a horizontal linear-gradient mask. The weight
/// ramps linearly from `start_weight` at x=0 to
/// `end_weight` at x=width.
pub fn gradient_h(width: u32, height: u32, start_weight: f32, end_weight: f32) -> Mask {
    let mut mask = Mask::zeros(width, height, MaskKind::User, "gradient_h".into());
    let w = width.max(1) as f32;
    for y in 0..height {
        for x in 0..width {
            let t = x as f32 / (w - 1.0).max(1.0);
            mask.set(x, y, start_weight + t * (end_weight - start_weight));
        }
    }
    mask
}

/// Build a radial-falloff mask centred on `(cx, cy)`.
/// Pixels at distance `inner_radius` get `1.0`;
/// pixels at distance `outer_radius` get `0.0`; the
/// falloff is linear.
pub fn radial(
    width: u32,
    height: u32,
    cx: f32,
    cy: f32,
    inner_radius: f32,
    outer_radius: f32,
) -> Mask {
    let mut mask = Mask::zeros(width, height, MaskKind::User, "radial".into());
    let span = (outer_radius - inner_radius).max(0.0001);
    let x_min = (cx - outer_radius).max(0.0).floor() as i32;
    let x_max = (cx + outer_radius).min(width as f32).ceil() as i32;
    let y_min = (cy - outer_radius).max(0.0).floor() as i32;
    let y_max = (cy + outer_radius).min(height as f32).ceil() as i32;
    for y in y_min..y_max {
        for x in x_min..x_max {
            let d = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt();
            let w = if d <= inner_radius {
                1.0
            } else if d >= outer_radius {
                0.0
            } else {
                1.0 - (d - inner_radius) / span
            };
            mask.set(x as u32, y as u32, w.clamp(0.0, 1.0));
        }
    }
    mask
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brush_paints_centre_and_falls_off_at_edge() {
        let m = brush(8, 8, 4.0, 4.0, 2.0, 1.0);
        assert_eq!(m.get(4, 4), 1.0);
        assert!(m.get(5, 4) < 1.0);
        assert_eq!(m.get(0, 0), 0.0);
    }

    #[test]
    fn polygon_fills_interior() {
        let m = polygon(6, 6, &[(1.0, 1.0), (4.0, 1.0), (4.0, 4.0), (1.0, 4.0)]);
        assert_eq!(m.get(0, 0), 0.0);
        assert_eq!(m.get(2, 2), 1.0);
        assert_eq!(m.get(5, 5), 0.0);
    }

    #[test]
    fn gradient_h_runs_start_to_end() {
        let m = gradient_h(4, 1, 0.0, 1.0);
        assert!((m.get(0, 0) - 0.0).abs() < 0.01);
        assert!((m.get(3, 0) - 1.0).abs() < 0.01);
    }

    #[test]
    fn radial_inner_outer_falloff() {
        let m = radial(10, 10, 5.0, 5.0, 1.0, 3.0);
        assert_eq!(m.get(5, 5), 1.0);
        assert_eq!(m.get(0, 0), 0.0);
        // (6, 5) is at distance 1.0 — equal to the inner
        // radius, so still 1.0.
        assert_eq!(m.get(6, 5), 1.0);
        // (7, 5) is at distance 2.0 — between inner
        // (1.0) and outer (3.0); falloff is 1 - (2 - 1)
        // / 2 = 0.5.
        assert!(m.get(7, 5) > 0.0);
        assert!(m.get(7, 5) < 1.0);
        // Beyond the outer radius: 0.
        assert_eq!(m.get(9, 5), 0.0);
    }
}
