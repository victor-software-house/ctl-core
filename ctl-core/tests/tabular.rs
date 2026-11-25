//! The built-in tabular formats, `tsv`, `csv`, and `markdown`: what each writes
//! for each block, how a hostile value is escaped, and that `tsv` is the
//! default when stdout is a pipe.
//!
//! The output of each format on each case is a reviewed snapshot in
//! `tests/snapshots`. A CSV snapshot shows each carriage return as `<CR>`,
//! since insta normalizes line breaks. Edition 2024 makes `set_var` unsafe, so
//! each case that reads the process streams reruns this test binary as a child.
#![allow(
    missing_docs,
    reason = "an integration test crate has no public API to document"
)]
#![cfg(feature = "app")]

use std::iter;

use anyhow::anyhow;
use clap::{Parser, Subcommand};
use ctl_core::format::{CSV, Format, FormatName, Formats, MARKDOWN, TSV};
use ctl_core::prelude::*;
use ctl_core::unsafe_chars::is_unsafe;
use serde::Serialize;

#[path = "support/child.rs"]
mod child;

/// A result that is only its document.
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

fn case(document: Document) -> Case {
    Case { document }
}

fn headers_table() -> Document {
    Document::new().table(
        Table::new(["id", "title", "age"])
            .row(["A-1", "Ship the release", "3"])
            .row(["A-2", "Fix the parser", "12"]),
    )
}

fn plain_table() -> Document {
    Document::new().table(Table::plain().row(["alpha", "1"]).row(["beta", "2"]))
}

fn fields() -> Document {
    Document::new().fields(
        Fields::new()
            .row("pending", "2")
            .row("latest", Text::new().id("A-1").then(" shipped")),
    )
}

fn report() -> Document {
    let notes = Section::new("Notes", Document::new().paragraph("Nothing else."));
    let open = Section::new(
        "Open rows",
        Document::new()
            .table(Table::new(["id", "title"]).row(["A-1", "Ship"]))
            .notice(Notice::new(NoticeLevel::Warning, "one row is stale").code("stale-row"))
            .fields(Fields::new().row("rows", "1"))
            .section(notes),
    );
    Document::new()
        .heading("Ledger report")
        .paragraph("One row is open.")
        .verbatim(indoc! {"
            Run:
                ```sh
                qctl status
                ```
        "})
        .section(open)
        .rule(Some(Text::plain("End")))
        .notice(Notice::new(NoticeLevel::Success, "done"))
        .notice(Notice::new(NoticeLevel::Error, "line one\nline two"))
}

fn hostile() -> Document {
    Document::new()
        .table(
            Table::new(["name", "value"])
                .row(["tab", "a\tb"])
                .row(["quote", "say \"hi\""])
                .row(["comma", "x,y"])
                .row(["pipe", "p|q"])
                .row(["escaped pipe", "p\\|q"])
                .row(["backslash", "c:\\dir"])
                .row(["break", "line1\nline2"])
                .row(["html break", "a<br>b"])
                .row(["entity", "x &lt; y"])
                .row(["clear screen", "a\u{1b}[2Jb"])
                .row(["clipboard", "a\u{1b}]52;c;aGk=\u{7}b"])
                .row(["bidi", "a\u{202e}b"])
                .row(["separator", "a\u{2028}b"])
                .row(["short"]),
        )
        .fields(Fields::new().row("a\tkey", "a\\tb\r\nc"))
}

/// Builds the document of one case.
type Build = fn() -> Document;

const CASES: [(&str, Build); 5] = [
    ("headers", headers_table),
    ("plain", plain_table),
    ("fields", fields),
    ("report", report),
    ("hostile", hostile),
];

const FORMATS: [(&str, &dyn Format); 3] = [("tsv", &TSV), ("csv", &CSV), ("markdown", &MARKDOWN)];

#[expect(
    clippy::unwrap_used,
    reason = "a test helper: a failure here is the calling test failing"
)]
fn written(format: &'static dyn Format, document: Document) -> String {
    let captured = View::new(format, ColorMode::Always)
        .capture(&case(document))
        .unwrap();
    assert_eq!(captured.stream(), Destination::Stdout);
    captured.text().to_owned()
}

#[test]
fn every_format_on_every_case_is_a_reviewed_snapshot() {
    for (format, writer) in FORMATS {
        for (name, build) in CASES {
            let text = written(writer, build());
            let unsafe_character = text
                .chars()
                .find(|c| is_unsafe(*c) && !matches!(c, '\t' | '\n' | '\r'));
            assert_eq!(unsafe_character, None, "{format} {name}: {text:?}");
            insta::assert_snapshot!(format!("{format}_{name}"), text.replace('\r', "<CR>"));
        }
    }
}

#[test]
fn a_tsv_cell_with_a_tab_stays_on_one_line() {
    let text = written(
        &TSV,
        Document::new().table(Table::new(["k", "v"]).row(["a\tb", "c"])),
    );
    assert_eq!(text, "k\tv\na\\tb\tc\n");
}

#[test]
fn a_csv_field_with_a_comma_and_quotes_is_quoted() {
    let text = written(
        &CSV,
        Document::new().table(Table::new(["k"]).row(["a, \"b\""])),
    );
    assert_eq!(text, "k\r\n\"a, \"\"b\"\"\"\r\n");
}

#[test]
fn a_record_of_one_empty_field_is_not_a_blank_line_in_csv() {
    let document = || Document::new().table(Table::plain().row([""]).row(["x"]));
    assert_eq!(written(&CSV, document()), "\"\"\r\nx\r\n");
    assert_eq!(
        written(&CSV, Document::new().table(Table::plain().row(["", ""]))),
        ",\r\n"
    );
    assert_eq!(written(&TSV, document()), "\nx\n");
}

#[test]
fn a_markdown_cell_escapes_so_each_value_reads_back() {
    let document = Document::new().table(
        Table::new(["a", "b", "c", "d", "e", "f"])
            .row(["p|q", "p\\|q", "a<br>b", "a\r\nb", "x &lt; y", "x & y"]),
    );
    assert_eq!(
        written(&MARKDOWN, document),
        "| a | b | c | d | e | f |\n\
         | --- | --- | --- | --- | --- | --- |\n\
         | p\\|q | p\\\\\\|q | a&lt;br>b | a<br>b | x &amp;lt; y | x &amp; y |\n"
    );
}

#[test]
fn markdown_ends_in_one_line_feed_and_a_titled_rule_is_not_a_heading() {
    let document = Document::new()
        .paragraph("text")
        .rule(Some(Text::plain("End")));
    assert_eq!(written(&MARKDOWN, document), "text\n\n**End**\n\n---\n");
}

#[test]
fn a_markdown_title_stays_on_one_line_and_an_empty_one_is_not_written() {
    let document = Document::new()
        .heading("one\ntwo\r\nthree")
        .section(Section::new("  \n", Document::new().paragraph("body")))
        .rule(Some(Text::plain(" \n ")))
        .rule(Some(Text::plain("a\nb")));
    assert_eq!(
        written(&MARKDOWN, document),
        "# one two three\n\nbody\n\n---\n\n**a b**\n\n---\n"
    );
}

#[test]
fn a_block_that_writes_nothing_adds_no_blank_line() {
    let document = || {
        Document::new()
            .table(Table::new(["a"]).row(["1"]))
            .paragraph("left out of csv")
            .notice(Notice::new(NoticeLevel::Warning, "left out of csv"))
            .rule(None)
            .table(Table::new(["b"]).row(["2"]))
    };
    assert_eq!(written(&CSV, document()), "a\r\n1\r\n\r\nb\r\n2\r\n");
    assert_eq!(
        written(&TSV, document()),
        "a\n1\n\nleft out of csv\n\nwarning\tleft out of csv\n\nb\n2\n"
    );
}

#[test]
fn an_empty_document_writes_nothing() {
    for (name, format) in FORMATS {
        assert_eq!(written(format, Document::new()), "", "{name}");
        assert_eq!(
            written(format, Document::new().table(Table::plain())),
            "",
            "{name}"
        );
    }
}

#[test]
fn the_tabular_formats_are_machine_formats_with_no_shorthand() {
    for (name, format) in FORMATS {
        assert!(format.machine(), "{name}");
        assert_eq!(format.shorthand(), None, "{name}");
        assert_eq!(format.name().as_str(), name);
    }
}

#[test]
fn the_built_in_set_pipes_tsv_and_shows_pretty_on_a_terminal() {
    let formats = Formats::builtin();
    assert_eq!(
        formats.names().collect::<Vec<_>>(),
        [
            FormatName::PRETTY,
            FormatName::JSON,
            FormatName::TSV,
            FormatName::CSV,
            FormatName::MARKDOWN
        ]
    );
    let unset = View::requested(None, ColorMode::Never);
    assert_eq!(
        unset.resolve(&formats, true).unwrap().format(),
        FormatName::PRETTY
    );
    assert_eq!(
        unset.resolve(&formats, false).unwrap().format(),
        FormatName::TSV
    );
    let kept = Formats::builtin().piped_default(FormatName::PRETTY);
    assert_eq!(
        unset.resolve(&kept, false).unwrap().format(),
        FormatName::PRETTY
    );
}

#[test]
fn quiet_hides_a_default_tsv_but_not_a_named_one() {
    let formats = Formats::builtin();
    let unset = View::requested(None, ColorMode::Never).quiet(true);
    let defaulted = unset.resolve(&formats, false).unwrap();
    let named = View::requested(Some(FormatName::TSV), ColorMode::Never)
        .quiet(true)
        .resolve(&formats, false)
        .unwrap();
    let status = case(fields());
    assert_eq!(
        defaulted.capture(&status).unwrap().stream(),
        Destination::None
    );
    assert_eq!(
        named.capture(&status).unwrap().stream(),
        Destination::Stdout
    );
}

#[derive(Parser)]
#[command(name = "toy", version, about = "toy", env_prefix = "TOY")]
struct Toy {
    #[command(flatten)]
    output: OutputArgs,
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// List rows.
    List,
    /// Fail with a message.
    Fail,
}

const CHILD: &str = "CTL_CORE_TABULAR_CHILD";

fn run(formats: Formats, args: &[&str]) -> ExitCode {
    App::<Toy>::new("toy")
        .formats(formats)
        .view(|toy| toy.output.view())
        .run_from(
            iter::once("toy").chain(args.iter().copied()),
            |toy| match toy.command {
                Cmd::List => Ok(case(headers_table())),
                Cmd::Fail => Err(anyhow!("ledger missing")),
            },
        )
}

#[test]
fn a_pipe_reads_tsv_when_no_format_is_named() {
    let Some(output) = child::in_child(
        CHILD,
        "a_pipe_reads_tsv_when_no_format_is_named",
        &[("CLICOLOR_FORCE", "1"), ("FORCE_COLOR", "3")],
    ) else {
        assert_eq!(
            child::case(|| run(Formats::builtin(), &["list"])),
            ExitCode::SUCCESS
        );
        return;
    };
    let stdout = child::case_stdout(&output);
    assert!(stdout.starts_with("id\ttitle\tage\n"), "{stdout:?}");
    assert!(!stdout.contains('\u{1b}'), "{stdout:?}");
}

#[test]
fn a_format_variable_names_csv_over_the_piped_default() {
    let Some(output) = child::in_child(
        CHILD,
        "a_format_variable_names_csv_over_the_piped_default",
        &[("TOY_FORMAT", "csv")],
    ) else {
        assert_eq!(
            child::case(|| run(Formats::builtin(), &["list"])),
            ExitCode::SUCCESS
        );
        return;
    };
    assert_eq!(
        child::case_stdout(&output),
        "id,title,age\r\nA-1,Ship the release,3\r\nA-2,Fix the parser,12\r\n"
    );
}

#[test]
fn a_failure_in_csv_writes_the_error_form_to_stderr_and_nothing_to_stdout() {
    let Some(output) = child::in_child(
        CHILD,
        "a_failure_in_csv_writes_the_error_form_to_stderr_and_nothing_to_stdout",
        &[],
    ) else {
        assert_eq!(
            child::case(|| run(Formats::builtin(), &["fail", "--format", "csv"])),
            ExitCode::FAILURE
        );
        return;
    };
    assert_eq!(child::case_stdout(&output), "");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("toy: ledger missing\n"), "{stderr}");
    assert!(!stderr.contains('\u{1b}'), "{stderr}");
}

#[test]
fn a_cli_that_keeps_pretty_when_piped_prints_the_colourless_table() {
    let Some(output) = child::in_child(
        CHILD,
        "a_cli_that_keeps_pretty_when_piped_prints_the_colourless_table",
        &[],
    ) else {
        let formats = Formats::builtin().piped_default(FormatName::PRETTY);
        assert_eq!(child::case(|| run(formats, &["list"])), ExitCode::SUCCESS);
        return;
    };
    let stdout = child::case_stdout(&output);
    assert!(!stdout.contains('\u{1b}'), "{stdout:?}");
    insta::assert_snapshot!("pretty_when_piped", stdout);
}

#[test]
fn a_machine_format_named_with_forced_colour_writes_no_escape() {
    let Some(output) = child::in_child(
        CHILD,
        "a_machine_format_named_with_forced_colour_writes_no_escape",
        &[("CLICOLOR_FORCE", "1"), ("FORCE_COLOR", "3")],
    ) else {
        child::case(|| {
            for format in ["json", "tsv", "csv", "markdown"] {
                let args = ["list", "--format", format, "--color", "always"];
                assert_eq!(
                    run(Formats::builtin(), &args),
                    ExitCode::SUCCESS,
                    "{format}"
                );
            }
        });
        return;
    };
    let stdout = child::case_stdout(&output);
    assert!(!stdout.contains('\u{1b}'), "{stdout:?}");
    for written in ["id\ttitle", "id,title", "| id | title |"] {
        assert!(stdout.contains(written), "{written}: {stdout:?}");
    }
}
