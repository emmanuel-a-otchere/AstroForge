//! CR-06 P2 — Image Analysis Engine.
//!
//! Produces a structured `ImageAnalysisReport` for an
//! `F32Image`, with per-observation evidence (pixel ranges or
//! locations) and confidence scores (0.0–1.0). The report is
//! what the recommendation engine (P3) reads to drive the
//! "Observation → Evidence → Confidence → Recommendation"
//! contract from CR-06 §6 / §7.
//!
//! The module is split into four sub-modules:
//!
//! - [`metrics`] — image-characteristics metrics (noise,
//!   background, clipping, chromatic noise, local contrast).
//! - [`structures`] — astronomical-structure detection
//!   (stars, nebula, galaxy). Heuristic-only; ML hooks land
//!   in a later phase if the heuristics prove insufficient.
//! - [`defects`] — image defects (hot pixels, trails).
//! - [`report`] — the `ImageAnalysisReport` aggregation and
//!   its JSON wire shape.
//!
//! The entry point is [`report::analyze`]; the rest of the
//! helpers are exposed for unit testing.

pub mod defects;
pub mod metrics;
pub mod report;
pub mod structures;
