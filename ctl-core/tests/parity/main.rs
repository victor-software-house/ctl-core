//! Parity corpus: the documents verctl, qctl, forkctl, and rawr render,
//! through ctl-core's public API, at four widths with and without colour.
//!
//! Each render is compared byte for byte with its file in `engine/`, and
//! every engine file has a file of the same name in `baseline/`. The
//! baselines come from the comfy-table renderer and stay frozen as the 0.6.x
//! reference, so a diff of the two directories shows every change the
//! render engine made; `DIFFERENCES.md` gives the reason for each kind.
//! `PARITY_UPDATE=1` writes missing or changed engine files and never
//! touches a baseline.
//!
//! A file holds the render with every ESC byte written as the six characters
//! `\u{1b}`, so a diff shows where a style starts and ends.
#![allow(
    missing_docs,
    reason = "an integration test crate has no public API to document"
)]
#![cfg(feature = "help")]

mod corpus;
mod engine;

use ctl_core::prelude::{ColorMode, Frame, RecordStyle, RenderOptions, RowSeparation};

const WIDTHS: [u16; 4] = [20, 40, 80, 120];

const MODES: [(ColorMode, &str); 2] = [(ColorMode::Never, "plain"), (ColorMode::Always, "color")];

/// An explicit width and explicit styles, so no variable in the environment
/// (`COLUMNS`, `CTL_CORE_RECORD_STYLE`, and the like) changes a render.
fn options(color: ColorMode, width: u16) -> RenderOptions {
    RenderOptions::new(color)
        .with_width(width)
        .with_record_style(RecordStyle::KeysRight)
        .with_frame(Frame::Grid)
        .with_row_separation(RowSeparation::None)
}

#[test]
fn every_case_matches_its_engine_file() {
    let mut reports = Vec::new();
    for (case, build) in corpus::CASES {
        let document = build();
        for width in WIDTHS {
            for (color, label) in MODES {
                let rendered = document.render(options(color, width));
                let name = format!("{case}-w{width}-{label}.txt");
                let encoded = rendered.replace('\u{1b}', "\\u{1b}");
                reports.extend(engine::check(&name, &encoded));
            }
        }
    }
    assert!(
        reports.is_empty(),
        "{} engine file(s) do not match\n\n{}",
        reports.len(),
        reports.join("\n")
    );
}

#[test]
fn every_engine_file_has_a_baseline() {
    let reports = engine::without_baseline();
    assert!(reports.is_empty(), "{}", reports.join("\n"));
}

/// `Document::render` encodes at full colour depth with ctl-term's encoder,
/// and a view encodes at the depth its stream's environment gives. Both must
/// give the same bytes for the same document, width, and colour decision. The
/// default theme uses only the 16 named colours, so no colour depth changes
/// them.
#[cfg(feature = "view")]
#[test]
fn a_view_writes_what_render_writes() {
    use ctl_core::prelude::{Document, OutputFormat, Present, View};
    use serde::Serialize;

    #[derive(Serialize)]
    struct Case {
        #[serde(skip)]
        document: Document,
    }

    impl Present for Case {
        fn present(&self) -> Document {
            self.document.clone()
        }
    }

    for (case, build) in corpus::CASES {
        let document = build();
        for width in WIDTHS {
            for (color, label) in MODES {
                let rendered = document.render(options(color, width));
                let view = View::new(OutputFormat::Pretty, color)
                    .with_width(width)
                    .with_styles(options(color, width));
                let captured = view
                    .capture(&Case {
                        document: document.clone(),
                    })
                    .unwrap_or_else(|error| panic!("{case}: {error}"));
                assert_eq!(
                    captured.text(),
                    rendered,
                    "{case} at width {width}, {label}"
                );
            }
        }
    }
}
