//! Parity corpus: the documents verctl, qctl, forkctl, and rawr render,
//! through ctl-core's public API, at four widths with and without colour,
//! compared byte for byte with committed baselines.
//!
//! The baselines in `baseline/` come from the comfy-table renderer and stay
//! frozen as the 0.6.x reference. The render-engine change adds its own
//! outputs beside them, so each change in output shows as a diff. Write
//! missing or changed baselines with `PARITY_UPDATE=1`.
//!
//! A file holds the render with every ESC byte written as the six characters
//! `\u{1b}`, so a diff shows where a style starts and ends.
#![allow(
    missing_docs,
    reason = "an integration test crate has no public API to document"
)]
#![cfg(feature = "help")]

mod baseline;
mod corpus;

use ctl_core::prelude::{ColorMode, ListStyle, RecordStyle, RenderOptions, RowSeparation};

const WIDTHS: [u16; 4] = [20, 40, 80, 120];

const MODES: [(ColorMode, &str); 2] = [(ColorMode::Never, "plain"), (ColorMode::Always, "color")];

/// An explicit width and explicit styles, so no variable in the environment
/// (`COLUMNS`, `CTL_CORE_RECORD_STYLE`, and the like) changes a render.
fn options(color: ColorMode, width: u16) -> RenderOptions {
    RenderOptions::new(color)
        .width(width)
        .record_style(RecordStyle::KeysRight)
        .list_style(ListStyle::Grid)
        .row_separation(RowSeparation::None)
}

#[test]
fn every_case_matches_its_baseline() {
    let mut reports = Vec::new();
    for (case, build) in corpus::CASES {
        let document = build();
        for width in WIDTHS {
            for (color, label) in MODES {
                let rendered = document.render(options(color, width));
                let name = format!("{case}-w{width}-{label}.txt");
                let encoded = rendered.replace('\u{1b}', "\\u{1b}");
                reports.extend(baseline::check(&name, &encoded));
            }
        }
    }
    assert!(
        reports.is_empty(),
        "{} baseline file(s) do not match\n\n{}",
        reports.len(),
        reports.join("\n")
    );
}
