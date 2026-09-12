//! CR-06 P6 — Quality gate report + orchestrator.
//!
//! The `run` function is the entry point. It takes
//! `(source, result, thresholds)`, calls each of
//! the ten gates in `gates::*`, and returns a
//! `QualityGateReport` carrying every finding +
//! an overall verdict.
//!
//! The verdict is computed from the severities:
//! any `Failure` finding makes the verdict
//! `Failure`; otherwise any `Warning` makes it
//! `Warning`; otherwise any `Info` makes it
//! `Info`; otherwise `Ok`.

use serde::{Deserialize, Serialize};

use crate::image::F32Image;
use crate::masks::Mask;
use crate::quality_gates::{gates, GateFinding, GateId, GateThresholds, Severity};

/// Overall verdict. Mirrors `Severity` but applies
/// to the whole report rather than a single gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualityVerdict {
    Ok,
    Info,
    Warning,
    Failure,
}

impl QualityVerdict {
    pub fn as_str(&self) -> &'static str {
        match self {
            QualityVerdict::Ok => "ok",
            QualityVerdict::Info => "info",
            QualityVerdict::Warning => "warning",
            QualityVerdict::Failure => "failure",
        }
    }
}

/// Aggregate quality report returned alongside each
/// applied AI operation. The schema mirrors the §37
/// example output so the frontend can render it
/// without translation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QualityGateReport {
    pub verdict: QualityVerdict,
    pub findings: Vec<GateFinding>,
    /// Identifier of the source Image Version.
    pub source_image_version_id: String,
    /// Identifier of the result Image Version.
    pub result_image_version_id: String,
    /// Identifier of the operation that produced
    /// the result.
    pub operation_id: String,
}

/// Compute the overall verdict from the per-gate
/// severities.
pub fn verdict(findings: &[GateFinding]) -> QualityVerdict {
    let mut verdict = QualityVerdict::Ok;
    for f in findings {
        match f.severity {
            Severity::Failure => return QualityVerdict::Failure,
            Severity::Warning => verdict = QualityVerdict::Warning,
            Severity::Info => {
                if verdict == QualityVerdict::Ok {
                    verdict = QualityVerdict::Info;
                }
            }
            Severity::Ok => {}
        }
    }
    verdict
}

/// Run all ten gates against the (source, result)
/// pair and return the aggregate report. P6 ships
/// the orchestrator + the per-gate helpers; the
/// Tauri command layer (P6 task 3) calls this after
/// every `enhancement_apply_operation` round. P5.1
/// threads the operation mask through so the
/// `SegmentationLeakage` gate compares real
/// included/excluded region deltas.
pub fn run(
    source: &F32Image,
    result: &F32Image,
    thresholds: &GateThresholds,
    mask: Option<&Mask>,
    source_image_version_id: String,
    result_image_version_id: String,
    operation_id: String,
) -> QualityGateReport {
    let findings = vec![
        gates::clipping(source, result, thresholds),
        gates::noise_amplification(source, result, thresholds),
        gates::star_artifacts(source, result, thresholds),
        gates::halos(source, result, thresholds),
        gates::ringing(source, result, thresholds),
        gates::false_structures(source, result, thresholds),
        gates::color_shifts(source, result, thresholds),
        gates::edge_artifacts(source, result, thresholds),
        gates::segmentation_leakage(source, result, mask, thresholds),
        gates::excessive_smoothing(source, result, thresholds),
    ];
    let verdict = verdict(&findings);
    QualityGateReport {
        verdict,
        findings,
        source_image_version_id,
        result_image_version_id,
        operation_id,
    }
}

/// Summarise the gate counts (e.g. "5 ok, 2 info, 2
/// warning, 1 failure"). P6's frontend renders this
/// as the headline numbers in the QualityGatePanel.
pub fn count_by_severity(findings: &[GateFinding]) -> (usize, usize, usize, usize) {
    let mut ok = 0;
    let mut info = 0;
    let mut warn = 0;
    let mut fail = 0;
    for f in findings {
        match f.severity {
            Severity::Ok => ok += 1,
            Severity::Info => info += 1,
            Severity::Warning => warn += 1,
            Severity::Failure => fail += 1,
        }
    }
    (ok, info, warn, fail)
}

/// True when a gate with the given id fired
/// (Warning or Failure severity).
pub fn gate_fired(findings: &[GateFinding], id: GateId) -> bool {
    findings
        .iter()
        .find(|f| f.gate == id)
        .map(|f| matches!(f.severity, Severity::Warning | Severity::Failure))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::F32Image;
    use ndarray::Array3;

    fn flat(value: f32) -> F32Image {
        let arr = Array3::<f32>::from_elem((1, 4, 4), value);
        F32Image::from(arr)
    }

    fn noisy() -> F32Image {
        let mut arr = Array3::<f32>::zeros((1, 4, 4));
        for y in 0..4 {
            for x in 0..4 {
                arr[(0, y, x)] = ((x + y * 4) as f32) / 16.0;
            }
        }
        F32Image::from(arr)
    }

    #[test]
    fn verdict_ok_when_all_gates_pass() {
        let src = flat(0.5);
        let res = flat(0.5);
        let t = GateThresholds::default();
        let report = run(
            &src,
            &res,
            &t,
            None,
            "src".into(),
            "res".into(),
            "op".into(),
        );
        assert_eq!(report.verdict, QualityVerdict::Ok);
    }

    #[test]
    fn verdict_warning_when_a_gate_warns() {
        let src = noisy();
        let res = flat(0.5);
        let t = GateThresholds::default();
        let report = run(
            &src,
            &res,
            &t,
            None,
            "src".into(),
            "res".into(),
            "op".into(),
        );
        // Excessive smoothing fires because the
        // noisy source was flattened.
        assert!(matches!(
            report.verdict,
            QualityVerdict::Warning | QualityVerdict::Failure
        ));
    }

    #[test]
    fn verdict_failure_when_a_gate_fails() {
        let src = flat(0.5);
        let res = flat(1.0);
        let t = GateThresholds::default();
        let report = run(
            &src,
            &res,
            &t,
            None,
            "src".into(),
            "res".into(),
            "op".into(),
        );
        // Clipping fires hard: every pixel pushed to
        // the rail.
        assert_eq!(report.verdict, QualityVerdict::Failure);
    }

    #[test]
    fn run_returns_ten_findings() {
        let src = flat(0.5);
        let res = flat(0.5);
        let t = GateThresholds::default();
        let report = run(
            &src,
            &res,
            &t,
            None,
            "src".into(),
            "res".into(),
            "op".into(),
        );
        assert_eq!(report.findings.len(), 10);
    }

    #[test]
    fn count_by_severity_returns_total_in_tuple() {
        let src = flat(0.5);
        let res = flat(0.5);
        let t = GateThresholds::default();
        let report = run(
            &src,
            &res,
            &t,
            None,
            "src".into(),
            "res".into(),
            "op".into(),
        );
        let (ok, info, warn, fail) = count_by_severity(&report.findings);
        assert_eq!(ok + info + warn + fail, 10);
    }

    #[test]
    fn gate_fired_true_for_warning_severity() {
        let src = noisy();
        let res = flat(0.5);
        let t = GateThresholds::default();
        let report = run(
            &src,
            &res,
            &t,
            None,
            "src".into(),
            "res".into(),
            "op".into(),
        );
        assert!(gate_fired(&report.findings, GateId::ExcessiveSmoothing));
    }

    #[test]
    fn verdict_strings_match_documentation() {
        assert_eq!(QualityVerdict::Ok.as_str(), "ok");
        assert_eq!(QualityVerdict::Info.as_str(), "info");
        assert_eq!(QualityVerdict::Warning.as_str(), "warning");
        assert_eq!(QualityVerdict::Failure.as_str(), "failure");
    }
}
