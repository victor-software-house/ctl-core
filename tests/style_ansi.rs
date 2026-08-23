//! Public semantic color contract.
#![allow(missing_docs)]
#![cfg(feature = "render")]

use ctl_core::{ColorMode, Document, RenderOptions, Text};

#[test]
fn semantic_token_is_colored_only_when_requested() {
    let document = Document::new().paragraph(Text::new().token("--help"));
    let colored = document.render(RenderOptions::new(ColorMode::Always).width(80));
    let plain = document.render(RenderOptions::new(ColorMode::Never).width(80));
    assert!(colored.contains('\u{1b}'), "{colored:?}");
    assert_eq!(plain, "--help\n");
}

#[test]
fn empty_semantic_text_stays_empty() {
    let rendered = Document::new()
        .paragraph(Text::new().token(""))
        .render(RenderOptions::new(ColorMode::Always).width(80));
    assert!(rendered.is_empty(), "{rendered:?}");
}
