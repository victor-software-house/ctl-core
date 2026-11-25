//! A CLI's own formats: the registry, the flags it adds, and what the format
//! interface guarantees every format.
//!
//! Edition 2024 makes `set_var` unsafe, so each case that reads the
//! environment or the streams reruns this test binary as a child.
#![allow(
    missing_docs,
    reason = "an integration test crate has no public API to document"
)]
#![cfg(feature = "app")]

use std::io::{self, Write as _};
use std::iter;

use clap::{CommandFactory, Parser, Subcommand};
use ctl_core::format::{
    CSV, Format, FormatName, Formats, JSON, MARKDOWN, Output, PRETTY, Sink, TSV,
};
use ctl_core::prelude::*;
use serde::Serialize;

#[path = "support/child.rs"]
mod child;

const YAML_NAME: FormatName = FormatName::new("yaml");

/// A machine format of the CLI's own, with a shorthand.
struct Yaml;

static YAML: Yaml = Yaml;

impl Format for Yaml {
    fn name(&self) -> FormatName {
        YAML_NAME
    }

    fn about(&self) -> &'static str {
        "Indented key-value text"
    }

    fn shorthand(&self) -> Option<&'static str> {
        Some("yaml")
    }

    fn write(&self, output: &Output<'_>, sink: &mut Sink<'_>) -> io::Result<()> {
        write!(sink, "yaml:")?;
        serde_json::to_writer(&mut *sink, output.data())?;
        writeln!(sink)
    }
}

/// A format that replaces the built-in `json`.
struct Replacement;

static REPLACEMENT: Replacement = Replacement;

impl Format for Replacement {
    fn name(&self) -> FormatName {
        FormatName::JSON
    }

    fn about(&self) -> &'static str {
        "Replaced"
    }

    fn write(&self, _output: &Output<'_>, sink: &mut Sink<'_>) -> io::Result<()> {
        writeln!(sink, "replaced")
    }
}

/// A format that lays the document out as `pretty` does, and is or is not a
/// machine format as its field says.
struct Layout(bool);

static MACHINE_LAYOUT: Layout = Layout(true);
static HUMAN_LAYOUT: Layout = Layout(false);

impl Format for Layout {
    fn name(&self) -> FormatName {
        FormatName::new(if self.0 {
            "machine-layout"
        } else {
            "human-layout"
        })
    }

    fn about(&self) -> &'static str {
        "The document, laid out"
    }

    fn machine(&self) -> bool {
        self.0
    }

    fn write(&self, output: &Output<'_>, sink: &mut Sink<'_>) -> io::Result<()> {
        let text = sink.render(output.document());
        sink.write_all(text.as_bytes())
    }
}

#[derive(Serialize)]
struct Status {
    pending: usize,
}

impl Present for Status {
    fn present(&self) -> Document {
        Document::new().paragraph(Text::new().error("pending"))
    }
}

fn status() -> Status {
    Status { pending: 2 }
}

fn text(captured: &Captured) -> &str {
    captured.text()
}

#[test]
fn a_registered_format_writes_the_result() {
    let formats = Formats::builtin().add(&YAML);
    let view = View::requested(Some(YAML_NAME), ColorMode::Never)
        .resolve(&formats, true)
        .unwrap();
    let captured = view.capture(&status()).unwrap();
    assert_eq!(captured.stream(), Destination::Stdout);
    assert_eq!(text(&captured), "yaml:{\"pending\":2}\n");
    assert_eq!(view.format(), YAML_NAME);
}

#[test]
fn a_format_of_the_same_name_replaces_a_built_in() {
    let formats = Formats::builtin().add(&REPLACEMENT);
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
    let view = View::requested(Some(FormatName::JSON), ColorMode::Never)
        .resolve(&formats, true)
        .unwrap();
    assert_eq!(text(&view.capture(&status()).unwrap()), "replaced\n");
}

#[test]
fn a_removed_format_cannot_be_requested() {
    let formats = Formats::builtin().add(&YAML).remove(FormatName::JSON);
    assert_eq!(
        formats.names().collect::<Vec<_>>(),
        [
            FormatName::PRETTY,
            FormatName::TSV,
            FormatName::CSV,
            FormatName::MARKDOWN,
            YAML_NAME
        ]
    );
    assert!(formats.get(FormatName::JSON).is_none());
    let error = View::requested(Some(FormatName::JSON), ColorMode::Never)
        .resolve(&formats, true)
        .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    assert!(error.to_string().contains("json"), "{error}");
}

#[test]
fn each_default_follows_the_terminal_check() {
    let formats = Formats::builtin()
        .add(&YAML)
        .terminal_default(FormatName::PRETTY)
        .piped_default(YAML_NAME);
    let unset = View::requested(None, ColorMode::Never);
    assert_eq!(
        unset.resolve(&formats, true).unwrap().format(),
        FormatName::PRETTY
    );
    assert_eq!(unset.resolve(&formats, false).unwrap().format(), YAML_NAME);
    let missing = Formats::builtin().piped_default(YAML_NAME);
    assert_eq!(
        unset.resolve(&missing, false).unwrap_err().kind(),
        io::ErrorKind::InvalidInput
    );
}

#[test]
fn quiet_hides_pretty_and_a_default_but_not_a_named_machine_format() {
    let formats = Formats::builtin().add(&YAML).piped_default(YAML_NAME);
    let capture = |view: View| view.quiet(true).capture(&status()).unwrap().stream();
    let requested = |name| View::requested(name, ColorMode::Never);

    assert_eq!(
        capture(View::new(&YAML, ColorMode::Never)),
        Destination::Stdout
    );
    assert_eq!(
        capture(requested(Some(YAML_NAME)).resolve(&formats, false).unwrap()),
        Destination::Stdout
    );
    assert_eq!(
        capture(requested(None).resolve(&formats, false).unwrap()),
        Destination::None
    );
    assert_eq!(
        capture(
            requested(Some(FormatName::PRETTY))
                .resolve(&formats, false)
                .unwrap()
        ),
        Destination::None
    );
    assert_eq!(
        capture(View::new(&PRETTY, ColorMode::Never)),
        Destination::None
    );
    assert_eq!(
        capture(View::new(&JSON, ColorMode::Never)),
        Destination::Stdout
    );
}

#[test]
fn a_machine_format_never_writes_an_escape_whatever_the_colour_mode_says() {
    let machine = View::new(&MACHINE_LAYOUT, ColorMode::Always)
        .capture(&status())
        .unwrap();
    assert!(!text(&machine).contains('\u{1b}'), "{:?}", text(&machine));
    assert!(text(&machine).contains("pending"));
    let human = View::new(&HUMAN_LAYOUT, ColorMode::Always)
        .capture(&status())
        .unwrap();
    assert!(text(&human).contains('\u{1b}'), "{:?}", text(&human));
    let machine: [&'static dyn Format; 4] = [&JSON, &TSV, &CSV, &MARKDOWN];
    for built_in in machine {
        let captured = View::new(built_in, ColorMode::Always)
            .capture(&status())
            .unwrap();
        assert!(
            !text(&captured).contains('\u{1b}'),
            "{}: {:?}",
            built_in.name(),
            text(&captured)
        );
    }
}

#[derive(Parser)]
#[command(name = "toy", version, about = "toy", env_prefix = "FMTTOY")]
struct Toy {
    #[command(flatten)]
    output: OutputArgs,
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Show status.
    Status,
}

fn formats() -> Formats {
    Formats::builtin().add(&YAML)
}

fn run(args: &[&str]) -> ExitCode {
    run_with(formats(), |toy| toy.output.view(), args)
}

fn run_with(formats: Formats, select: fn(&Toy) -> View, args: &[&str]) -> ExitCode {
    App::<Toy>::new("toy")
        .formats(formats)
        .view(select)
        .run_from(iter::once("toy").chain(args.iter().copied()), |_| {
            Ok(status())
        })
}

const CHILD: &str = "CTL_CORE_FORMATS_CHILD";

#[test]
fn a_shorthand_selects_its_format() {
    let Some(output) = child::in_child(CHILD, "a_shorthand_selects_its_format", &[]) else {
        assert_eq!(
            child::case(|| run(&["status", "--yaml"])),
            ExitCode::SUCCESS
        );
        return;
    };
    assert_eq!(child::case_stdout(&output), "yaml:{\"pending\":2}\n");
}

#[test]
fn a_shorthand_reads_its_variable() {
    let Some(output) = child::in_child(
        CHILD,
        "a_shorthand_reads_its_variable",
        &[("FMTTOY_YAML", "true")],
    ) else {
        assert_eq!(child::case(|| run(&["status"])), ExitCode::SUCCESS);
        return;
    };
    assert_eq!(child::case_stdout(&output), "yaml:{\"pending\":2}\n");
}

#[test]
fn the_format_variable_names_a_custom_format() {
    let Some(output) = child::in_child(
        CHILD,
        "the_format_variable_names_a_custom_format",
        &[("FMTTOY_FORMAT", "yaml")],
    ) else {
        assert_eq!(
            child::case(|| run(&["status", "--quiet"])),
            ExitCode::SUCCESS
        );
        return;
    };
    assert_eq!(child::case_stdout(&output), "yaml:{\"pending\":2}\n");
}

#[test]
fn the_command_line_beats_the_variable_of_another_shorthand() {
    let Some(output) = child::in_child(
        CHILD,
        "the_command_line_beats_the_variable_of_another_shorthand",
        &[("FMTTOY_JSON", "true")],
    ) else {
        assert_eq!(
            child::case(|| run(&["status", "--yaml"])),
            ExitCode::SUCCESS
        );
        return;
    };
    assert_eq!(child::case_stdout(&output), "yaml:{\"pending\":2}\n");
}

#[test]
fn two_shorthands_naming_different_formats_are_a_usage_error() {
    let Some(output) = child::in_child(
        CHILD,
        "two_shorthands_naming_different_formats_are_a_usage_error",
        &[],
    ) else {
        assert_eq!(
            child::case(|| run(&["status", "--yaml", "--json"])),
            ExitCode::from(2)
        );
        return;
    };
    assert_eq!(child::case_stdout(&output), "");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--yaml"), "{stderr}");
    assert!(stderr.contains("--json"), "{stderr}");
}

#[test]
fn the_same_format_twice_is_fine() {
    let Some(output) = child::in_child(CHILD, "the_same_format_twice_is_fine", &[]) else {
        assert_eq!(
            child::case(|| run(&["status", "--yaml", "--format", "yaml"])),
            ExitCode::SUCCESS
        );
        return;
    };
    assert_eq!(child::case_stdout(&output), "yaml:{\"pending\":2}\n");
}

#[test]
fn an_unregistered_name_is_a_usage_error_that_lists_the_registered_names() {
    let Some(output) = child::in_child(
        CHILD,
        "an_unregistered_name_is_a_usage_error_that_lists_the_registered_names",
        &[],
    ) else {
        assert_eq!(
            child::case(|| run(&["status", "--format", "msgpack"])),
            ExitCode::from(2)
        );
        return;
    };
    assert_eq!(child::case_stdout(&output), "");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(
            "unknown format `msgpack`; use one of pretty, json, tsv, csv, markdown, yaml"
        ),
        "{stderr}"
    );
}

#[test]
fn help_lists_the_registered_formats_and_the_shorthand_variable() {
    let Some(output) = child::in_child(
        CHILD,
        "help_lists_the_registered_formats_and_the_shorthand_variable",
        &[("COLUMNS", "200")],
    ) else {
        assert_eq!(child::case(|| run(&["--help"])), ExitCode::SUCCESS);
        return;
    };
    let stdout = child::case_stdout(&output);
    assert!(
        stdout.contains("[pretty|json|tsv|csv|markdown|yaml]"),
        "{stdout}"
    );
    assert!(stdout.contains("--yaml"), "{stdout}");
    assert!(stdout.contains("Shorthand for `--format yaml`"), "{stdout}");
    assert!(stdout.contains("FMTTOY_YAML"), "{stdout}");
}

#[test]
fn a_parse_error_after_a_shorthand_uses_that_formats_error_form() {
    let Some(output) = child::in_child(
        CHILD,
        "a_parse_error_after_a_shorthand_uses_that_formats_error_form",
        &[("CLICOLOR_FORCE", "1")],
    ) else {
        assert_eq!(
            child::case(|| run(&["status", "--bogus", "--yaml"])),
            ExitCode::from(2)
        );
        return;
    };
    assert_eq!(child::case_stdout(&output), "");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.starts_with("toy: "), "{stderr}");
    assert!(!stderr.contains('\u{1b}'), "{stderr}");
}

#[cfg(feature = "completion")]
#[test]
fn completion_offers_the_registered_names() {
    use ctl_core::completion::VAR;

    let Some(output) = child::in_child(
        CHILD,
        "completion_offers_the_registered_names",
        &[(VAR, "fish")],
    ) else {
        let code = child::case(|| {
            App::<Toy>::new("toy")
                .formats(formats())
                .completion()
                .run_from(["toy", "--", "toy", "status", "--format", "y"], |_| {
                    Ok(status())
                })
        });
        assert_eq!(code, ExitCode::SUCCESS);
        return;
    };
    let stdout = child::case_stdout(&output);
    assert!(stdout.contains("yaml\tIndented key-value text"), "{stdout}");
}

#[cfg(all(feature = "usage", feature = "surface-serde"))]
#[test]
fn the_usage_spec_and_the_surface_list_the_registered_names() {
    use ctl_core::surface::Surface;
    use ctl_core::usage;

    let command = formats().augment(Toy::command());
    let spec = usage::spec(command.clone(), "toy");
    assert!(
        spec.contains("choice yaml help=\"Indented key-value text\""),
        "{spec}"
    );
    assert!(spec.contains("flag --yaml"), "{spec}");
    let surface = serde_json::to_string(&Surface::from_command(command, "toy")).unwrap();
    assert!(surface.contains("yaml"), "{surface}");
}

#[test]
fn a_command_with_no_format_argument_is_returned_as_it_is() {
    let command = clap::Command::new("bare").arg(clap::Arg::new("name"));
    let augmented = formats().augment(command.clone());
    assert_eq!(
        augmented.get_arguments().count(),
        command.get_arguments().count()
    );
}

#[test]
fn removing_json_hides_its_flag() {
    let command = Formats::builtin()
        .remove(FormatName::JSON)
        .augment(Toy::command());
    let json = command
        .get_arguments()
        .find(|arg| arg.get_id() == "json")
        .unwrap();
    assert!(json.is_hide_set());
}

#[test]
fn the_json_flag_selects_a_format_that_replaces_json_without_a_shorthand() {
    let Some(output) = child::in_child(
        CHILD,
        "the_json_flag_selects_a_format_that_replaces_json_without_a_shorthand",
        &[],
    ) else {
        let code = child::case(|| {
            run_with(
                Formats::builtin().add(&REPLACEMENT),
                |toy| toy.output.view(),
                &["status", "--json"],
            )
        });
        assert_eq!(code, ExitCode::SUCCESS);
        return;
    };
    assert_eq!(child::case_stdout(&output), "replaced\n");
}

#[test]
fn the_json_flag_is_a_usage_error_once_json_is_removed() {
    let Some(output) = child::in_child(
        CHILD,
        "the_json_flag_is_a_usage_error_once_json_is_removed",
        &[],
    ) else {
        let code = child::case(|| {
            run_with(
                Formats::builtin().add(&YAML).remove(FormatName::JSON),
                |toy| toy.output.view(),
                &["status", "--json"],
            )
        });
        assert_eq!(code, ExitCode::from(2));
        return;
    };
    assert_eq!(child::case_stdout(&output), "");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unknown format `json`; use one of pretty, tsv, csv, markdown, yaml"),
        "{stderr}"
    );
}

#[test]
fn the_json_variable_is_a_usage_error_once_json_is_removed() {
    let Some(output) = child::in_child(
        CHILD,
        "the_json_variable_is_a_usage_error_once_json_is_removed",
        &[("FMTTOY_JSON", "true")],
    ) else {
        let code = child::case(|| {
            run_with(
                Formats::builtin().add(&YAML).remove(FormatName::JSON),
                |toy| toy.output.view(),
                &["status"],
            )
        });
        assert_eq!(code, ExitCode::from(2));
        return;
    };
    assert_eq!(child::case_stdout(&output), "");
    let stderr = String::from_utf8_lossy(&output.stderr)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        stderr.contains(
            "unknown format `json` from the variable `FMTTOY_JSON`; use one of pretty, tsv, csv, markdown, yaml"
        ),
        "{stderr}"
    );
}

#[test]
fn a_flag_a_subcommand_declares_keeps_a_shorthand_off_the_root() {
    let command = clap::Command::new("toy")
        .arg(
            clap::Arg::new("format")
                .long("format")
                .global(true)
                .env("FMTTOY_FORMAT"),
        )
        .subcommand(
            clap::Command::new("status").arg(
                clap::Arg::new("legacy")
                    .long("yaml")
                    .action(clap::ArgAction::SetTrue),
            ),
        );
    let augmented = formats().augment(command);
    augmented.clone().debug_assert();
    let matches = augmented
        .try_get_matches_from(["toy", "status", "--yaml"])
        .unwrap();
    assert!(
        matches
            .subcommand_matches("status")
            .is_some_and(|status| status.get_flag("legacy"))
    );
}

#[test]
fn a_view_that_requested_a_format_keeps_it_when_no_flag_names_one() {
    let Some(output) = child::in_child(
        CHILD,
        "a_view_that_requested_a_format_keeps_it_when_no_flag_names_one",
        &[],
    ) else {
        let code = child::case(|| {
            run_with(
                formats(),
                |_| View::requested(Some(FormatName::JSON), ColorMode::Never),
                &["status"],
            )
        });
        assert_eq!(code, ExitCode::SUCCESS);
        return;
    };
    assert_eq!(child::case_stdout(&output), "{\n  \"pending\": 2\n}\n");
}

#[test]
fn a_parse_error_after_a_shorthand_variable_uses_that_formats_error_form() {
    let Some(output) = child::in_child(
        CHILD,
        "a_parse_error_after_a_shorthand_variable_uses_that_formats_error_form",
        &[("CLICOLOR_FORCE", "1"), ("FMTTOY_YAML", "true")],
    ) else {
        assert_eq!(
            child::case(|| run(&["status", "--bogus"])),
            ExitCode::from(2)
        );
        return;
    };
    assert_eq!(child::case_stdout(&output), "");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.starts_with("toy: "), "{stderr}");
    assert!(!stderr.contains('\u{1b}'), "{stderr}");
}

#[test]
fn an_unregistered_format_variable_is_a_usage_error_that_names_the_variable() {
    let Some(output) = child::in_child(
        CHILD,
        "an_unregistered_format_variable_is_a_usage_error_that_names_the_variable",
        &[("FMTTOY_FORMAT", "nope")],
    ) else {
        assert_eq!(child::case(|| run(&["status"])), ExitCode::from(2));
        return;
    };
    assert_eq!(child::case_stdout(&output), "");
    let stderr = String::from_utf8_lossy(&output.stderr)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        stderr.contains(
            "unknown format `nope` from the variable `FMTTOY_FORMAT`; use one of pretty, json, tsv, csv, markdown, yaml"
        ),
        "{stderr}"
    );
}
