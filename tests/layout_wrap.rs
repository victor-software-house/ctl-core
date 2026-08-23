//! Public semantic layout contract.
#![allow(missing_docs)]
#![cfg(feature = "render")]

use ctl_core::{ColorMode, Document, RenderOptions, Table, Text};

#[test]
fn paragraph_wraps_to_explicit_width() {
    let text = "one two three four five six seven eight";
    let rendered = Document::new()
        .paragraph(text)
        .render(RenderOptions::new(ColorMode::Never).width(16));
    assert!(rendered.lines().count() > 1, "{rendered:?}");
    assert!(rendered.ends_with('\n'), "{rendered:?}");
}

#[test]
fn narrow_table_stacks_labels_and_description() {
    let table = Table::new(Vec::<Text>::new()).stacked_below(64, 2).row([
        "-f",
        "--format",
        "Output representation",
    ]);
    let rendered = Document::new()
        .table(table)
        .render(RenderOptions::new(ColorMode::Never).width(40));
    assert_eq!(rendered, "  -f --format\n    Output representation\n");
}
