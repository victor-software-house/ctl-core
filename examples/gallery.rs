//! Every candidate presentation style, rendered by ctl-core itself, so the
//! defaults are picked from real output.
//!
//! ```text
//! cargo run --example gallery -- [--color never] [WIDTH ...]
//! ```
//!
//! Each variant carries a short code (`R2`, `L5`, `J3`) to answer with.

use std::io::{self, Write as _};

use ctl_core::{
    ColorMode, Document, Fields, JsonLayout, ListStyle, OutputFormat, Present, RecordStyle,
    RenderOptions, RowSeparation, Table, Text, View,
};
use serde::Serialize;

const RECORDS: [(&str, RecordStyle, &str); 3] = [
    ("R1", RecordStyle::Boxed, "boxed, as today"),
    (
        "R2",
        RecordStyle::KeysRight,
        "borderless, keys right-aligned",
    ),
    ("R3", RecordStyle::KeysLeft, "borderless, keys left-aligned"),
];

const LISTS: [(ListStyle, &str); 3] = [
    (ListStyle::Grid, "full grid"),
    (ListStyle::HeaderRule, "header rule only"),
    (ListStyle::Plain, "no rules"),
];

const SEPARATIONS: [(RowSeparation, &str); 3] = [
    (RowSeparation::None, "rows together"),
    (RowSeparation::Rule, "rule between rows"),
    (RowSeparation::Blank, "blank line between rows"),
];

const JSON: [(&str, JsonLayout, &str); 2] = [
    ("J1", JsonLayout::Compact, "compact, as today"),
    ("J2", JsonLayout::Pretty, "pretty, two-space indent"),
];

#[derive(Serialize)]
struct Row {
    priority: u8,
    id: &'static str,
    title: &'static str,
    state: &'static str,
    outcome: &'static str,
}

const ROWS: [Row; 4] = [
    Row {
        priority: 1,
        id: "QCTL-014",
        title: "Let a ledger declare whether rows are blank-separated",
        state: "active",
        outcome: "fmt keeps one blank line between rows when the style asks for it, so a lost \
                  separator is repaired rather than found by eye.",
    },
    Row {
        priority: 2,
        id: "QCTL-015",
        title: "Write a ledger atomically so a crash cannot truncate it",
        state: "queued",
        outcome: "A ledger write either lands whole or not at all.",
    },
    Row {
        priority: 3,
        id: "QCTL-016",
        title: "Name the next row a ledger can actually start",
        state: "queued",
        outcome: "status names one startable row, or says why none is.",
    },
    Row {
        priority: 4,
        id: "QCTL-017",
        title: "Carry examples in command help",
        state: "blocked",
        outcome: "Frequently mistyped verbs show an Examples block in their own help.",
    },
];

#[derive(Serialize)]
struct Queue {
    active: &'static str,
    rows: &'static [Row],
}

impl Present for Queue {
    fn present(&self) -> Document {
        Document::new().table(queue_table())
    }
}

fn state(value: &str) -> Text {
    match value {
        "active" => Text::new().success(value.to_owned()),
        "blocked" => Text::new().warning(value.to_owned()),
        _ => Text::new().muted(value.to_owned()),
    }
}

fn queue_table() -> Table {
    ROWS.iter().fold(
        Table::new(["", "id", "title", "state"]).id_column(1),
        |table, row| {
            table.row([
                Text::plain(row.priority.to_string()),
                Text::plain(row.id),
                Text::plain(row.title),
                state(row.state),
            ])
        },
    )
}

fn record(row: &Row) -> Document {
    Document::new()
        .paragraph(Text::new().id(row.id).then("  ").then(row.title))
        .fields(
            Fields::new()
                .row("state", state(row.state))
                .row("priority", Text::plain(row.priority.to_string()))
                .row("outcome", Text::plain(row.outcome)),
        )
}

fn banner(code: &str, label: &str, width: u16) -> Document {
    Document::new().rule(Some(
        Text::new()
            .token(code.to_owned())
            .then(format!("  {label}  · width {width}")),
    ))
}

fn render(out: &mut impl io::Write, document: &Document, options: RenderOptions) -> io::Result<()> {
    out.write_all(document.render(options).as_bytes())?;
    out.write_all(b"\n")
}

fn gallery(out: &mut impl io::Write, color: ColorMode, width: u16) -> io::Result<()> {
    let base = RenderOptions::new(color).width(width);
    for (code, style, label) in RECORDS {
        render(out, &banner(code, label, width), base)?;
        let options = base.record_style(style);
        render(out, &record(&ROWS[0]), options)?;
        render(out, &record(&ROWS[1]), options)?;
    }
    for (list_index, (list, list_label)) in LISTS.into_iter().enumerate() {
        for (separation_index, (separation, separation_label)) in
            SEPARATIONS.into_iter().enumerate()
        {
            let code = format!("L{}", list_index * SEPARATIONS.len() + separation_index + 1);
            let label = format!("{list_label}, {separation_label}");
            render(out, &banner(&code, &label, width), base)?;
            let options = base.list_style(list).row_separation(separation);
            render(out, &Document::new().table(queue_table()), options)?;
        }
    }
    let queue = Queue {
        active: "QCTL-014",
        rows: &ROWS[..2],
    };
    for (code, layout, label) in JSON {
        render(out, &banner(code, label, width), base)?;
        let captured = View::new(OutputFormat::Json, color)
            .json_layout(layout)
            .capture(&queue)?;
        out.write_all(captured.bytes())?;
        out.write_all(b"\n")?;
    }
    Ok(())
}

fn main() -> io::Result<()> {
    let mut color = ColorMode::Always;
    let mut widths = Vec::new();
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        if argument == "--color" {
            if arguments.next().as_deref() == Some("never") {
                color = ColorMode::Never;
            }
        } else if let Ok(width) = argument.parse::<u16>() {
            widths.push(width);
        }
    }
    if widths.is_empty() {
        widths.push(80);
    }
    let stdout = io::stdout();
    let mut out = stdout.lock();
    for width in widths {
        gallery(&mut out, color, width)?;
    }
    out.flush()
}
