//! CR-07 §11 — Quality assessment + natural-language summary.
//!
//! B2 of the CR-07 implementation plan. Aggregates findings from the
//! existing `quality_gates` module into a [`QualityAssessment`] (from
//! B1 `comparison.rs`) and produces a §11-style natural-language
//! summary.
//!
//! ## Existing integration
//!
//! - `quality_gates::GateFinding` carries the per-stage integrity
//!   checks (`clipping`, `halos`, `ringing`, `star_artifacts`, etc.).
//! - `quality_gates::QualityVerdict` carries the existing
//!   Ok / Info / Warning / Failure classification.
//! - `quality_gates::report::QualityGateReport::verdict()` aggregates
//!   findings into a verdict.
//!
//! B2 reuses the verdict + findings + integrity checks and adds:
//! 1. A `QualityAssessment` (B1 type) populated from the gate
//!    findings.
//! 2. A §11 natural-language summary generator.
//!
//! ## §11 example output
//!
//! ```text
//! Assessment
//! [+] Noise substantially reduced
//! [+] Background gradient reduced
//! [-] Star profiles preserved
//! [-] Highlight clipping increased
//! [-] Moderate AI detail artifacts detected
//! Overall:
//! Strong improvement with minor trade-offs.
//! ```

use crate::comparison::DeltaDirection;
use crate::comparison::{
    AssessmentFinding, AssessmentSeverity, ComparisonItem, IntegrityCheck, QualityAssessment,
    QualityVerdict,
};
use crate::metric_registry::MetricDeltaRow;
use crate::quality_gates::report::QualityVerdict as GateVerdict;
use crate::quality_gates::{GateFinding, GateId};

/// Build a `QualityAssessment` (B1 type) from a list of gate
/// findings + metric delta rows.
///
/// `session_id` is the parent `ComparisonSession::id`. `baseline` and
/// `compared` are the `ComparisonItem` slots for the two versions
/// being compared.
pub fn from_gate_findings(
    session_id: impl Into<String>,
    findings: &[GateFinding],
    delta_rows: &[MetricDeltaRow],
    baseline: &ComparisonItem,
    compared: &ComparisonItem,
) -> QualityAssessment {
    let assessment_findings: Vec<AssessmentFinding> = findings
        .iter()
        .map(|f| AssessmentFinding {
            kind: f.gate.as_str().to_string(),
            severity: severity_from_gate(&f.severity_as_str()),
            message: f.message.clone(),
        })
        .collect();

    let integrity_checks: Vec<IntegrityCheck> = findings
        .iter()
        .map(|f| IntegrityCheck {
            kind: f.gate.as_str().to_string(),
            passed: matches!(
                gate_verdict(&f.severity_as_str()),
                GateVerdict::Ok | GateVerdict::Info
            ),
            detail: f.message.clone(),
        })
        .collect();

    let gate_verdict = verdict_from_findings(findings);
    let verdict = assessment_verdict(&gate_verdict, delta_rows);
    let summary = natural_language_summary(delta_rows, findings, baseline, compared);

    QualityAssessment {
        id: format!(
            "qa-{}-{}",
            crate::comparison::now_iso8601().replace([':', '-', 'T', 'Z'], ""),
            crate::comparison::next_nonce()
        ),
        session_id: session_id.into(),
        summary,
        findings: assessment_findings,
        integrity_checks,
        verdict: Some(verdict),
    }
}

/// Render a §11-style natural-language summary from the delta rows
/// + gate findings.
///
/// The summary uses the four-tier sign convention:
///
/// - `[+] Improved` for `Improved` deltas and improvements
/// - `[-] Degraded` for `Degraded` deltas and new failures
/// - `[~] Unchanged` for `Unchanged` deltas (omitted in the summary
///   unless they were the dominant signal)
/// - `[?] Inconclusive` for ambiguous metrics and warning-level gates
///
/// The "Overall:" line follows the CR-07 §11 template.
pub fn natural_language_summary(
    delta_rows: &[MetricDeltaRow],
    findings: &[GateFinding],
    baseline: &ComparisonItem,
    compared: &ComparisonItem,
) -> String {
    let mut improved = Vec::new();
    let mut degraded = Vec::new();
    for row in delta_rows {
        match row.direction {
            DeltaDirection::Improved => improved.push(row.label),
            DeltaDirection::Degraded => degraded.push(row.label),
            DeltaDirection::Unchanged | DeltaDirection::Inconclusive => {}
        }
    }
    // Take the top 3 of each to keep the summary readable.
    improved.truncate(3);
    degraded.truncate(3);

    let mut lines: Vec<String> = Vec::new();
    for label in &improved {
        lines.push(format!("[+] {} {}", label, improved_verb(label)));
    }
    for label in &degraded {
        lines.push(format!("[-] {} {}", label, degraded_verb(label)));
    }
    // Add gate findings as part of the summary.
    for finding in findings {
        let verdict = gate_verdict(&finding.severity_as_str());
        let marker = match verdict {
            GateVerdict::Ok => continue, // omit "ok" findings from summary
            GateVerdict::Info => "[i]",
            GateVerdict::Warning => "[!]",
            GateVerdict::Failure => "[!]",
        };
        let summary_kind = gate_summary_label(&finding.gate);
        lines.push(format!(
            "{} {} {}",
            marker,
            summary_kind,
            finding_summary_verb(&finding.gate, verdict)
        ));
    }

    let header = format!(
        "Comparison: {} vs {}",
        slot_label(baseline),
        slot_label(compared)
    );
    let overall = overall_summary(delta_rows, findings);

    if lines.is_empty() {
        format!("{}\nOverall:\n{}", header, overall)
    } else {
        format!("{}\n{}\nOverall:\n{}", header, lines.join("\n"), overall)
    }
}

// ---------- helpers ----------

fn slot_label(item: &ComparisonItem) -> String {
    let slot = match item.slot {
        crate::comparison::ComparisonSlot::A => "A",
        crate::comparison::ComparisonSlot::B => "B",
        crate::comparison::ComparisonSlot::C => "C",
        crate::comparison::ComparisonSlot::D => "D",
    };
    item.label
        .clone()
        .unwrap_or_else(|| format!("{} ({})", slot, item.version_id))
}

fn improved_verb(label: &str) -> &'static str {
    // Higher-is-better metrics get "improved"; lower-is-better get
    // "reduced". This matches the §11 example's "Noise substantially
    // reduced" (lower-is-better) and "Background gradient reduced"
    // (lower-is-better).
    match label {
        "Star count"
        | "Star/background contrast"
        | "Estimated SNR"
        | "Local SNR"
        | "AI segmentation confidence"
        | "AI model confidence" => "improved",
        _ => "reduced",
    }
}

fn degraded_verb(label: &str) -> &'static str {
    match label {
        "Star count"
        | "Star/background contrast"
        | "Estimated SNR"
        | "Local SNR"
        | "AI segmentation confidence"
        | "AI model confidence" => "degraded",
        _ => "increased",
    }
}

fn gate_summary_label(gate: &GateId) -> &'static str {
    match gate {
        GateId::Clipping => "Clipping",
        GateId::NoiseAmplification => "Noise amplification",
        GateId::StarArtifacts => "Star artifacts",
        GateId::Halos => "Halos",
        GateId::Ringing => "Ringing",
        GateId::FalseStructures => "False structures",
        GateId::ColorShifts => "Color shifts",
        GateId::EdgeArtifacts => "Edge artifacts",
        GateId::SegmentationLeakage => "Segmentation leakage",
        GateId::ExcessiveSmoothing => "Excessive smoothing",
    }
}

fn finding_summary_verb(gate: &GateId, verdict: GateVerdict) -> &'static str {
    // Per §11 example: "AI detail artifacts detected" / "Highlight
    // clipping increased" — the verb describes the situation, not
    // the direction.
    let _ = gate;
    match verdict {
        GateVerdict::Ok => "ok",
        GateVerdict::Info => "informational",
        GateVerdict::Warning => "warning",
        GateVerdict::Failure => "failed",
    }
}

fn gate_verdict(severity: &str) -> GateVerdict {
    match severity {
        "ok" => GateVerdict::Ok,
        "info" => GateVerdict::Info,
        "warning" => GateVerdict::Warning,
        "failure" => GateVerdict::Failure,
        // Unknown severities treated as warnings to surface them.
        _ => GateVerdict::Warning,
    }
}

fn verdict_from_findings(findings: &[GateFinding]) -> GateVerdict {
    // Mirrors `quality_gates::report::verdict()` semantics without
    // taking the dependency on the report module — keep this pure.
    let mut has_failure = false;
    let mut has_warning = false;
    for f in findings {
        match gate_verdict(&f.severity_as_str()) {
            GateVerdict::Failure => has_failure = true,
            GateVerdict::Warning => has_warning = true,
            _ => {}
        }
    }
    if has_failure {
        GateVerdict::Failure
    } else if has_warning {
        GateVerdict::Warning
    } else {
        GateVerdict::Ok
    }
}

fn assessment_verdict(gate_verdict: &GateVerdict, delta_rows: &[MetricDeltaRow]) -> QualityVerdict {
    let improved = delta_rows
        .iter()
        .filter(|r| matches!(r.direction, DeltaDirection::Improved))
        .count();
    let degraded = delta_rows
        .iter()
        .filter(|r| matches!(r.direction, DeltaDirection::Degraded))
        .count();
    let net = improved as i64 - degraded as i64;

    // QualityVerdict is a tradeoff between gate findings + metric
    // net direction.
    match (gate_verdict, net) {
        (GateVerdict::Failure, _) => QualityVerdict::Degradation,
        (GateVerdict::Ok | GateVerdict::Info, n) if n >= 3 => QualityVerdict::StrongImprovement,
        (_, n) if n >= 1 => QualityVerdict::ImprovementWithTradeoffs,
        (_, 0) => QualityVerdict::Neutral,
        (_, _) => QualityVerdict::Degradation,
    }
}

fn overall_summary(delta_rows: &[MetricDeltaRow], findings: &[GateFinding]) -> String {
    let improved = delta_rows
        .iter()
        .filter(|r| matches!(r.direction, DeltaDirection::Improved))
        .count();
    let degraded = delta_rows
        .iter()
        .filter(|r| matches!(r.direction, DeltaDirection::Degraded))
        .count();
    let warnings = findings
        .iter()
        .filter(|f| {
            matches!(
                gate_verdict(&f.severity_as_str()),
                GateVerdict::Warning | GateVerdict::Failure
            )
        })
        .count();

    let metric_summary = match (improved, degraded) {
        (0, 0) => "no material metric changes".to_string(),
        (i, 0) => format!("{} metric(s) improved", i),
        (0, d) => format!("{} metric(s) degraded", d),
        (i, d) => format!("{} improved, {} degraded", i, d),
    };
    let warning_summary = if warnings == 0 {
        "no quality warnings".to_string()
    } else {
        format!("{} quality warning(s)", warnings)
    };
    format!("{}\n{}", metric_summary, warning_summary)
}

fn severity_from_gate(severity: &str) -> AssessmentSeverity {
    match gate_verdict(severity) {
        GateVerdict::Ok | GateVerdict::Info => AssessmentSeverity::Info,
        GateVerdict::Warning => AssessmentSeverity::Warn,
        GateVerdict::Failure => AssessmentSeverity::Fail,
    }
}

// ---------- helpers for GateFinding::severity_as_str + gate ----------

/// Lightweight wrapper trait to read severity + gate fields from
/// `GateFinding` without adding an `impl` block in the
/// `quality_gates` module (B2 should not modify other modules
/// unless absolutely necessary).
mod finding_access {
    use super::*;
    pub trait GateFindingAccess {
        fn severity_as_str(&self) -> String;
    }
    impl GateFindingAccess for GateFinding {
        fn severity_as_str(&self) -> String {
            // GateFinding's `severity` is the `quality_gates::Severity`
            // enum; we delegate to its `as_str` via a public API.
            format!("{:?}", self.severity).to_lowercase()
        }
    }
}

use finding_access::GateFindingAccess;

// We need the trait extension in scope for `severity_as_str` to be
// callable on `&GateFinding`. The blanket impl above provides it.
#[allow(unused_imports)]
use GateFindingAccess as _;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comparison::{ComparisonItem, ComparisonSlot};
    use crate::quality_gates::{GateFinding, GateId, Severity};

    fn fixture() -> (Vec<MetricDeltaRow>, Vec<GateFinding>) {
        // Two improvements, one degradation in metrics; one warning.
        let deltas = vec![
            MetricDeltaRow {
                kind: crate::metric_registry::MetricKind::NoiseLuminance,
                baseline_value: Some(20.0),
                compared_value: Some(10.0),
                direction: DeltaDirection::Improved,
                percent_change: -50.0,
                materiality_threshold: 5.0,
                label: "Noise (luminance)",
                unit: "sigma",
                context: "",
            },
            MetricDeltaRow {
                kind: crate::metric_registry::MetricKind::BackgroundGradient,
                baseline_value: Some(0.5),
                compared_value: Some(0.35),
                direction: DeltaDirection::Improved,
                percent_change: -30.0,
                materiality_threshold: 10.0,
                label: "Background gradient",
                unit: "slope/100px",
                context: "",
            },
            MetricDeltaRow {
                kind: crate::metric_registry::MetricKind::DynamicRangeHighlightClipping,
                baseline_value: Some(0.3),
                compared_value: Some(1.1),
                direction: DeltaDirection::Degraded,
                percent_change: 266.7,
                materiality_threshold: 0.5,
                label: "Highlight clipping",
                unit: "%",
                context: "",
            },
        ];
        let findings = vec![GateFinding {
            gate: GateId::Clipping,
            severity: Severity::Warning,
            message: "Highlight clipping increased to 1.1%".into(),
            score: 1.1,
            recommendation: None,
        }];
        (deltas, findings)
    }

    fn item_a() -> ComparisonItem {
        ComparisonItem {
            slot: ComparisonSlot::A,
            version_id: "v18".into(),
            label: Some("Natural".into()),
        }
    }
    fn item_b() -> ComparisonItem {
        ComparisonItem {
            slot: ComparisonSlot::B,
            version_id: "v19".into(),
            label: Some("AI Enhanced".into()),
        }
    }

    #[test]
    fn assessment_contains_findings_and_integrity_checks() {
        let (deltas, findings) = fixture();
        let qa = from_gate_findings("session-1", &findings, &deltas, &item_a(), &item_b());
        assert_eq!(qa.findings.len(), 1);
        assert_eq!(qa.integrity_checks.len(), 1);
        assert!(!qa.summary.is_empty());
        assert!(qa.verdict.is_some());
    }

    #[test]
    fn assessment_summary_mentions_improved_and_degraded() {
        let (deltas, findings) = fixture();
        let qa = from_gate_findings("session-1", &findings, &deltas, &item_a(), &item_b());
        // Improved metrics: noise (luminance), background gradient
        assert!(qa.summary.contains("Noise (luminance)"));
        assert!(qa.summary.contains("Background gradient"));
        // Degraded metric: highlight clipping
        assert!(qa.summary.contains("Highlight clipping"));
        // Slot labels in the header
        assert!(qa.summary.contains("Natural"));
        assert!(qa.summary.contains("AI Enhanced"));
    }

    #[test]
    fn assessment_verdict_improvement_with_tradeoffs() {
        // 2 improved + 1 degraded + warning -> ImprovementWithTradeoffs
        let (deltas, findings) = fixture();
        let qa = from_gate_findings("session-1", &findings, &deltas, &item_a(), &item_b());
        // 1 improved (not 3+) + ok gate -> ImprovementWithTradeoffs.
        // StrongImprovement requires 3+ improved metrics per the §11
        // example which lists multiple improvements together.
        assert!(matches!(
            qa.verdict,
            Some(QualityVerdict::ImprovementWithTradeoffs)
        ));
    }

    #[test]
    fn assessment_verdict_strong_improvement_when_three_improvements() {
        // 3 improved + ok gate -> StrongImprovement.
        let deltas = vec![
            MetricDeltaRow {
                kind: crate::metric_registry::MetricKind::NoiseLuminance,
                baseline_value: Some(20.0),
                compared_value: Some(10.0),
                direction: DeltaDirection::Improved,
                percent_change: -50.0,
                materiality_threshold: 5.0,
                label: "Noise (luminance)",
                unit: "sigma",
                context: "",
            },
            MetricDeltaRow {
                kind: crate::metric_registry::MetricKind::BackgroundGradient,
                baseline_value: Some(0.5),
                compared_value: Some(0.3),
                direction: DeltaDirection::Improved,
                percent_change: -40.0,
                materiality_threshold: 10.0,
                label: "Background gradient",
                unit: "slope/100px",
                context: "",
            },
            MetricDeltaRow {
                kind: crate::metric_registry::MetricKind::DynamicRangeHighlightClipping,
                baseline_value: Some(1.0),
                compared_value: Some(0.3),
                direction: DeltaDirection::Improved,
                percent_change: -70.0,
                materiality_threshold: 0.5,
                label: "Highlight clipping",
                unit: "%",
                context: "",
            },
        ];
        let findings = vec![GateFinding {
            gate: GateId::Clipping,
            severity: Severity::Ok,
            message: "Clipping under threshold".into(),
            score: 0.0,
            recommendation: None,
        }];
        let qa = from_gate_findings("session-1", &findings, &deltas, &item_a(), &item_b());
        assert!(matches!(
            qa.verdict,
            Some(QualityVerdict::StrongImprovement)
        ));
    }

    #[test]
    fn assessment_verdict_degradation_when_failure_present() {
        let (deltas, mut findings) = fixture();
        findings.push(GateFinding {
            gate: GateId::Halos,
            severity: Severity::Failure,
            message: "Severe halos detected".into(),
            score: 0.0,
            recommendation: None,
        });
        let qa = from_gate_findings("session-1", &findings, &deltas, &item_a(), &item_b());
        assert!(matches!(qa.verdict, Some(QualityVerdict::Degradation)));
    }

    #[test]
    fn assessment_handles_empty_findings_and_deltas() {
        let qa = from_gate_findings("session-1", &[], &[], &item_a(), &item_b());
        assert!(qa.findings.is_empty());
        assert!(qa.integrity_checks.is_empty());
        assert!(matches!(qa.verdict, Some(QualityVerdict::Neutral)));
        assert!(qa.summary.contains("Comparison"));
    }
}
