//! Design-token invariants: the guarantees that keep look-and-feel
//! consistent as screens multiply. If one of these fails, a token was
//! changed in a way that breaks the design system contract.

use astroforge_ui::app::status_for_project;
use astroforge_ui::tokens as t;

#[test]
fn spacing_is_on_the_4px_grid() {
    let steps = [
        t::spacing::UNIT,
        t::spacing::XS,
        t::spacing::SM,
        t::spacing::MD,
        t::spacing::LG,
        t::spacing::XL,
        t::spacing::CONTAINER_SAFE,
    ];
    for v in steps {
        assert!(
            v > 0.0 && v % t::spacing::UNIT == 0.0,
            "spacing token {v} violates the 4px baseline grid"
        );
    }
}

#[test]
fn type_scale_descends_monotonically() {
    let scale = [
        t::type_scale::DISPLAY_XL.size,
        t::type_scale::HEADLINE_LG.size,
        t::type_scale::HEADLINE.size,
        t::type_scale::BODY.size,
        t::type_scale::DATA.size,
        t::type_scale::METADATA.size,
        t::type_scale::LABEL_CAPS.size,
    ];
    for pair in scale.windows(2) {
        assert!(
            pair[0] > pair[1],
            "type scale must strictly descend: {} !> {}",
            pair[0],
            pair[1]
        );
    }
}

#[test]
fn every_status_has_distinct_foreground() {
    let statuses = [
        t::Status::Neutral,
        t::Status::Info,
        t::Status::Success,
        t::Status::Processing,
        t::Status::Warning,
        t::Status::Error,
    ];
    for a in statuses {
        for b in statuses {
            if a != b {
                assert_ne!(
                    a.fg(),
                    b.fg(),
                    "statuses {a:?} and {b:?} share a foreground color; \
                     statuses must be distinguishable at a glance"
                );
            }
            // Pill fills stay translucent so content shows through.
            assert!(a.bg().a() < 64, "{a:?} pill fill too opaque");
        }
    }
}

#[test]
fn motion_stays_sharp_per_design_md() {
    use std::time::Duration;
    assert_eq!(t::motion::BASE, Duration::from_millis(150));
    assert!(t::motion::FAST <= t::motion::BASE);
    assert!(t::motion::BASE <= t::motion::SLOW);
    assert_eq!(t::egui_animation_time(), 0.15);
}

#[test]
fn domain_status_maps_to_expected_ui_status() {
    use astroforge_core::domain::ProjectStatus as P;

    let cases: [(P, t::Status, &str); 12] = [
        (P::New, t::Status::Neutral, "New"),
        (P::Imported, t::Status::Neutral, "New"),
        (P::Analyzing, t::Status::Processing, "Processing"),
        (P::Ready, t::Status::Success, "Ready"),
        (P::Processing, t::Status::Processing, "Processing"),
        (P::Paused, t::Status::Warning, "Paused"),
        (P::Failed, t::Status::Error, "Failed"),
        (P::Recovering, t::Status::Neutral, "New"),
        (P::Processed, t::Status::Success, "Ready"),
        (P::Enhancing, t::Status::Processing, "Processing"),
        (P::Review, t::Status::Info, "Review"),
        (P::Exported, t::Status::Success, "Exported"),
    ];
    for (domain, expected_status, expected_label) in cases {
        let (status, label) = status_for_project(&domain);
        assert_eq!(status, expected_status, "wrong status for {domain:?}");
        assert_eq!(label, expected_label, "wrong label for {domain:?}");
    }
}
