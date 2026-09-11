//! CR-06 P5 — Boolean composition of masks.
//!
//! Per CR-06 §12, the user can compose multiple masks
//! into a single region using boolean operations:
//!
//! - `Union` (A ∪ B) — include either mask.
//! - `Intersect` (A ∩ B) — include only where both
//!   masks include.
//! - `Difference` (A − B) — include A but exclude B.
//!
//! Each operation returns a fresh `Mask` with kind
//! `Composite` and a provenance string that names the
//! operands (e.g. `"auto:stars + user:brush"`) so the
//! editor can reconstruct the recipe from the row
//! alone.
//!
//! The composition is pixel-wise and pure — no GPU, no
//! external state. Composition order matters for
//! non-commutative ops (Difference), so the operations
//! carry explicit names rather than relying on arg
//! position.

use serde::{Deserialize, Serialize};

use super::{Mask, MaskKind};

/// A boolean mask operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompositeOp {
    Union,
    Intersect,
    Difference,
}

impl CompositeOp {
    pub fn as_str(&self) -> &'static str {
        match self {
            CompositeOp::Union => "union",
            CompositeOp::Intersect => "intersect",
            CompositeOp::Difference => "difference",
        }
    }
}

/// Apply a binary operator to two masks.
///
/// The two inputs must share the same `width` and
/// `height` (the API does not interpolate). On size
/// mismatch, the function returns `Err` so the caller
/// surfaces a typed error rather than producing a
/// silently corrupted raster.
pub fn apply(a: &Mask, b: &Mask, op: CompositeOp) -> Result<Mask, CompositeError> {
    if a.width != b.width || a.height != b.height {
        return Err(CompositeError::SizeMismatch {
            a: (a.width, a.height),
            b: (b.width, b.height),
        });
    }
    let provenance = format!("{} {} {}", a.provenance, op.as_str(), b.provenance);
    let pixels = match op {
        CompositeOp::Union => a
            .pixels
            .iter()
            .zip(b.pixels.iter())
            .map(|(x, y)| x.max(*y))
            .collect(),
        CompositeOp::Intersect => a
            .pixels
            .iter()
            .zip(b.pixels.iter())
            .map(|(x, y)| x.min(*y))
            .collect(),
        CompositeOp::Difference => a
            .pixels
            .iter()
            .zip(b.pixels.iter())
            .map(|(x, y)| (x - y).max(0.0))
            .collect(),
    };
    Ok(Mask {
        width: a.width,
        height: a.height,
        pixels,
        kind: MaskKind::Composite,
        provenance,
    })
}

/// Reduce a sequence of masks via a left-fold
/// composition. The first mask is the identity; each
/// subsequent mask is combined with the running result
/// using `op`. Returns `Err` if the sequence is empty
/// (no identity element) or if any pair of operands
/// has mismatched dimensions.
pub fn reduce(initial: &Mask, rest: &[&Mask], op: CompositeOp) -> Result<Mask, CompositeError> {
    let mut acc = initial.clone();
    for m in rest {
        acc = apply(&acc, m, op)?;
    }
    Ok(acc)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompositeError {
    SizeMismatch { a: (u32, u32), b: (u32, u32) },
}

impl std::fmt::Display for CompositeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompositeError::SizeMismatch { a, b } => {
                write!(f, "mask size mismatch: {}x{} vs {}x{}", a.0, a.1, b.0, b.1)
            }
        }
    }
}

impl std::error::Error for CompositeError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn a_mask(width: u32, height: u32, vals: Vec<f32>) -> Mask {
        Mask::from_pixels(width, height, MaskKind::Auto, "a".into(), &vals)
    }

    #[test]
    fn union_takes_max() {
        let a = a_mask(2, 2, vec![0.2, 0.4, 0.6, 0.8]);
        let b = a_mask(2, 2, vec![0.5, 0.3, 0.7, 0.1]);
        let out = apply(&a, &b, CompositeOp::Union).unwrap();
        assert_eq!(out.pixels, vec![0.5, 0.4, 0.7, 0.8]);
        assert_eq!(out.kind, MaskKind::Composite);
    }

    #[test]
    fn intersect_takes_min() {
        let a = a_mask(2, 2, vec![0.2, 0.4, 0.6, 0.8]);
        let b = a_mask(2, 2, vec![0.5, 0.3, 0.7, 0.1]);
        let out = apply(&a, &b, CompositeOp::Intersect).unwrap();
        assert_eq!(out.pixels, vec![0.2, 0.3, 0.6, 0.1]);
    }

    #[test]
    fn difference_clamps_to_zero() {
        let a = a_mask(2, 2, vec![0.5, 0.5, 0.5, 0.5]);
        let b = a_mask(2, 2, vec![0.5, 0.25, 0.5, 0.0]);
        let out = apply(&a, &b, CompositeOp::Difference).unwrap();
        // Use approximate equality because f32
        // subtraction accumulates rounding.
        let expected = [0.0, 0.25, 0.0, 0.5];
        for (got, want) in out.pixels.iter().zip(expected.iter()) {
            assert!((got - want).abs() < 1e-6, "got {got} want {want}");
        }
    }

    #[test]
    fn provenance_records_operands() {
        let a = a_mask(1, 1, vec![0.5]);
        let b = Mask::from_pixels(1, 1, MaskKind::User, "b".into(), &[0.5]);
        let out = apply(&a, &b, CompositeOp::Union).unwrap();
        assert_eq!(out.provenance, "a union b");
    }

    #[test]
    fn size_mismatch_errors() {
        let a = a_mask(2, 2, vec![0.0; 4]);
        let b = a_mask(3, 3, vec![0.0; 9]);
        let err = apply(&a, &b, CompositeOp::Union).unwrap_err();
        assert!(matches!(err, CompositeError::SizeMismatch { .. }));
    }

    #[test]
    fn reduce_folds_left() {
        let a = a_mask(1, 1, vec![0.1]);
        let b = a_mask(1, 1, vec![0.5]);
        let c = a_mask(1, 1, vec![0.9]);
        let out = reduce(&a, &[&b, &c], CompositeOp::Union).unwrap();
        assert_eq!(out.pixels, vec![0.9]);
    }
}
