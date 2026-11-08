//! Properties of every widget, over random content with wide, combining,
//! emoji, and control characters, and widths from 1 to 200.
#![expect(
    clippy::disallowed_macros,
    reason = "proptest! and prop_assert! expand to concat!, which the repository bans in its own code"
)]

use ctl_layout::{Column, Frame, Indent, Record, RecordStyle, RowSeparation, Rule, Stack, Table};
use ctl_text::{Context, Line, Overflow, Renderable, Text};
use proptest::prelude::*;

const FRAGMENTS: &[&str] = &[
    "a",
    "word",
    "longerword",
    " ",
    "   ",
    "日",
    "本語",
    "한",
    "e\u{301}",
    "\u{301}",
    "🇧🇷",
    "\u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f467}\u{200d}\u{1f466}",
    "https://example.test/releases/tag/pkg@1.2.3",
    "\n",
    "\t",
    "\u{1b}",
    "\u{202e}",
];

fn any_text() -> impl Strategy<Value = String> {
    prop::collection::vec(prop::sample::select(FRAGMENTS), 0..6).prop_map(|parts| parts.concat())
}

fn any_width() -> impl Strategy<Value = u16> {
    1u16..=200
}

fn any_overflow() -> impl Strategy<Value = Overflow> {
    prop::sample::select(vec![
        Overflow::KeepWords,
        Overflow::Fold,
        Overflow::Ellipsis,
    ])
}

fn frames(allowed: Vec<Frame>) -> impl Strategy<Value = Frame> {
    prop::sample::select(allowed)
}

fn any_separation() -> impl Strategy<Value = RowSeparation> {
    prop::sample::select(vec![
        RowSeparation::None,
        RowSeparation::Rule,
        RowSeparation::Blank,
    ])
}

fn table_in(allowed: Vec<Frame>) -> impl Strategy<Value = Table> {
    (1_usize..=4)
        .prop_flat_map(move |columns| {
            (
                prop::collection::vec((any_overflow(), prop::bool::ANY, any_text()), columns),
                prop::collection::vec(prop::collection::vec(any_text(), columns), 0..5),
                frames(allowed.clone()),
                any_separation(),
            )
        })
        .prop_map(|(columns, rows, frame, separation)| {
            let columns = columns.into_iter().map(|(overflow, numeric, header)| {
                let column = Column::new(header).overflow(overflow);
                if numeric { column.numeric() } else { column }
            });
            Table::new(columns)
                .frame(frame)
                .row_separation(separation)
                .rows(
                    rows.into_iter()
                        .map(|row| row.into_iter().map(Text::plain).collect::<Vec<_>>()),
                )
        })
}

fn any_table() -> impl Strategy<Value = Table> {
    table_in(vec![Frame::Grid, Frame::HeaderRule, Frame::Plain])
}

fn any_record() -> impl Strategy<Value = Record> {
    (
        prop::collection::vec((any_text(), any_text()), 0..5),
        prop::sample::select(vec![
            RecordStyle::KeysRight,
            RecordStyle::KeysLeft,
            RecordStyle::Boxed,
        ]),
    )
        .prop_map(|(rows, style)| {
            rows.into_iter()
                .fold(Record::new().style(style), |record, (key, value)| {
                    record.row(Text::plain(key), Text::plain(value))
                })
        })
}

fn widest(lines: &[Line]) -> u16 {
    lines.iter().map(Line::width).max().unwrap_or(0)
}

/// No line is wider than the width, unless the widget cannot be narrower: a
/// kept-whole word or the frame itself then sets the bound.
fn assert_fits(widget: &dyn Renderable, width: u16) -> Result<(), TestCaseError> {
    let context = Context::new(width);
    let minimum = widget.measure(&context).minimum();
    let bound = width.max(minimum);
    for line in widget.render(&context) {
        prop_assert!(
            line.width() <= bound,
            "{line:?} is {} cells at width {width} with minimum {minimum}",
            line.width(),
        );
    }
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn a_table_fits_its_width_or_its_minimum(table in any_table(), width in any_width()) {
        assert_fits(&table, width)?;
    }

    #[test]
    fn a_grid_is_never_narrower_than_its_minimum(
        table in table_in(vec![Frame::Grid]),
        width in any_width(),
    ) {
        let context = Context::new(width);
        let measurement = table.measure(&context);
        prop_assert!(measurement.minimum() <= measurement.maximum());
        prop_assert!(
            width < measurement.minimum() || widest(&table.render(&context)) >= measurement.minimum(),
        );
    }

    #[test]
    fn a_grid_keeps_its_frame_aligned(table in table_in(vec![Frame::Grid]), width in any_width()) {
        let rendered = table.render(&Context::new(width));
        let first = rendered.first().map(Line::width);
        for line in &rendered {
            prop_assert_eq!(Some(line.width()), first, "{} is not as wide as the top", line);
        }
    }

    #[test]
    fn a_table_is_never_wider_than_its_natural_width(table in any_table(), width in any_width()) {
        let natural = table.measure(&Context::new(width)).maximum();
        prop_assert!(widest(&table.render(&Context::new(width))) <= natural);
    }

    #[test]
    fn a_record_fits_its_width_or_its_minimum(record in any_record(), width in any_width()) {
        assert_fits(&record, width)?;
    }

    #[test]
    fn a_stack_of_widgets_fits_its_width_or_its_minimum(
        table in any_table(),
        record in any_record(),
        title in any_text(),
        columns in 0_u16..8,
        width in any_width(),
    ) {
        let stack = Stack::new()
            .spacing(1)
            .push(Rule::titled(Text::plain(title)))
            .push(Indent::new(columns, table))
            .push(record);
        assert_fits(&stack, width)?;
    }

    #[test]
    fn a_rule_fills_its_width(title in any_text(), width in any_width()) {
        let rule = Rule::titled(Text::plain(title));
        for line in rule.render(&Context::new(width)) {
            prop_assert!(line.width() <= width, "{line} is wider than {width}");
        }
        let plain = Rule::new().render(&Context::new(width));
        prop_assert_eq!(plain.len(), 1);
        prop_assert_eq!(plain[0].width(), width);
    }

    #[test]
    fn no_line_ends_in_a_space(table in any_table(), width in any_width()) {
        for line in table.render(&Context::new(width)) {
            prop_assert!(!line.to_string().ends_with(' '), "{line:?} ends in a space");
        }
    }

    #[test]
    fn rendering_twice_gives_the_same_lines(table in any_table(), width in any_width()) {
        let context = Context::new(width);
        prop_assert_eq!(table.render(&context), table.render(&context));
    }
}
