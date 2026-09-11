//! CR-06 P5 — Mask encoding.
//!
//! The mask persistence layer (`AiMask::mask_json`) is
//! intentionally JSON-friendly. P5 ships a single
//! canonical wire shape:
//!
//! ```json
//! {
//!   "version": 1,
//!   "width": 1024,
//!   "height": 1024,
//!   "kind": "auto",
//!   "provenance": "stars",
//!   "encoding": "row_major_f32",
//!   "pixels": [0.0, 0.0, ...]
//! }
//! ```
//!
//! For large rasters, the JSON size can balloon — P5's
//! default policy is to write all pixels inline (the
//! analysis engine + user strokes both produce modest
//! raster sizes in practice; e.g. a 1024×1024 mask is
//! ~4 MB of JSON). A future slice can swap the
//! encoding for a binary blob (e.g. base64 + zlib) if
//! the size becomes a problem.
//!
//! The encoding is versioned so a future schema change
//! can branch without breaking older rows. P5 ships
//! version 1 only.

use serde::{Deserialize, Serialize};

use super::{Mask, MaskKind};

/// The wire shape `AiMask::mask_json` carries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaskWire {
    /// Schema version. Always 1 for P5.
    pub version: u32,
    pub width: u32,
    pub height: u32,
    pub kind: MaskKind,
    pub provenance: String,
    /// Encoding tag. Always `"row_major_f32"` for P5.
    #[serde(default = "default_encoding")]
    pub encoding: String,
    pub pixels: Vec<f32>,
}

fn default_encoding() -> String {
    "row_major_f32".to_string()
}

/// Serialise a `Mask` to its wire shape (JSON string).
pub fn to_json(mask: &Mask) -> Result<String, serde_json::Error> {
    let wire = MaskWire {
        version: 1,
        width: mask.width,
        height: mask.height,
        kind: mask.kind,
        provenance: mask.provenance.clone(),
        encoding: "row_major_f32".to_string(),
        pixels: mask.pixels.clone(),
    };
    serde_json::to_string(&wire)
}

/// Parse a `MaskWire` JSON string back into a `Mask`.
/// Unknown schema versions return an error so a future
/// migration can branch explicitly.
pub fn from_json(raw: &str) -> Result<Mask, MaskError> {
    let wire: MaskWire = serde_json::from_str(raw).map_err(|e| MaskError::Json(e.to_string()))?;
    if wire.version != 1 {
        return Err(MaskError::UnsupportedVersion(wire.version));
    }
    let expected = (wire.width as usize) * (wire.height as usize);
    if wire.pixels.len() != expected {
        return Err(MaskError::PixelCountMismatch {
            expected,
            actual: wire.pixels.len(),
        });
    }
    Ok(Mask {
        width: wire.width,
        height: wire.height,
        kind: wire.kind,
        provenance: wire.provenance,
        pixels: wire.pixels,
    })
}

/// Round-trip convenience. `from_json(to_json(mask)?)?`
/// without losing precision (f32 ↔ JSON number is
/// lossy for very large/small values but the mask
/// pixel range is `[0, 1]` so this is exact in
/// practice).
pub fn round_trip(mask: &Mask) -> Result<Mask, MaskError> {
    let raw = to_json(mask).map_err(|e| MaskError::Json(e.to_string()))?;
    from_json(&raw)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MaskError {
    Json(String),
    UnsupportedVersion(u32),
    PixelCountMismatch { expected: usize, actual: usize },
}

impl std::fmt::Display for MaskError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MaskError::Json(msg) => write!(f, "mask JSON error: {msg}"),
            MaskError::UnsupportedVersion(v) => {
                write!(f, "unsupported mask schema version: {v}")
            }
            MaskError::PixelCountMismatch { expected, actual } => write!(
                f,
                "mask pixel count mismatch: expected {expected}, got {actual}"
            ),
        }
    }
}

impl std::error::Error for MaskError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_mask() -> Mask {
        Mask::from_pixels(2, 2, MaskKind::Auto, "stars".into(), &[0.1, 0.2, 0.3, 0.4])
    }

    #[test]
    fn round_trip_preserves_pixels() {
        let original = sample_mask();
        let restored = round_trip(&original).expect("round trip");
        assert_eq!(restored, original);
    }

    #[test]
    fn json_carries_schema_metadata() {
        let raw = to_json(&sample_mask()).expect("to_json");
        assert!(raw.contains("\"version\":1"));
        assert!(raw.contains("\"kind\":\"auto\""));
        assert!(raw.contains("\"encoding\":\"row_major_f32\""));
        assert!(raw.contains("\"provenance\":\"stars\""));
    }

    #[test]
    fn unsupported_version_errors() {
        let raw = r#"{
            "version": 99,
            "width": 1,
            "height": 1,
            "kind": "auto",
            "provenance": "x",
            "encoding": "row_major_f32",
            "pixels": [0.5]
        }"#;
        let err = from_json(raw).unwrap_err();
        assert_eq!(err, MaskError::UnsupportedVersion(99));
    }

    #[test]
    fn pixel_count_mismatch_errors() {
        let raw = r#"{
            "version": 1,
            "width": 2,
            "height": 2,
            "kind": "auto",
            "provenance": "x",
            "encoding": "row_major_f32",
            "pixels": [0.1]
        }"#;
        let err = from_json(raw).unwrap_err();
        assert!(matches!(err, MaskError::PixelCountMismatch { .. }));
    }

    #[test]
    fn malformed_json_errors() {
        let err = from_json("not json {").unwrap_err();
        assert!(matches!(err, MaskError::Json(_)));
    }
}
