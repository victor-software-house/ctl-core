//! The Usage KDL that `--usage-spec` prints, for a fixed corpus of commands.
//!
//! Three kinds of command, each a reviewed snapshot in `tests/snapshots`:
//! one builder command that sets every attribute the spec reads, a toy
//! [`App`] mounted as the mise task `toy`, and one command per hostile string,
//! once as help text and once as a default value. The snapshots hold the
//! bytes the writer must keep, so a change to the spec shows as a diff.
#![allow(
    missing_docs,
    reason = "an integration test crate has no public API to document"
)]
#![cfg(all(feature = "usage", feature = "app"))]

use std::cell::RefCell;
use std::rc::Rc;

use clap::builder::PossibleValue;
use clap::{Arg, ArgAction, ArgGroup, Command, ValueHint};
use ctl_core::prelude::*;
use ctl_core::usage;
use serde::Serialize;

/// Strings a spec writer must quote or escape, each with its snapshot label.
const HOSTILE: &[(&str, &str)] = &[
    ("quote", "say \"hi\""),
    ("backslash", "a\\b\\\\c"),
    ("newline", "one\ntwo"),
    ("tab", "a\tb"),
    ("tab-multi", "a\tb\nc"),
    ("unicode", "café \u{65e5}\u{672c} e\u{301}"),
    ("triple-hash", "x\n\"\"\"#\ny"),
    ("quad-hash", "x\n\"\"\"\"#\ny"),
    ("triple-two-hash", "x\n\"\"\"##\ny"),
    ("ws-only-line", "a\n   \nb"),
    ("trailing-nl", "a\n"),
    ("leading-nl", "\na"),
    ("line-sep", "a\u{2028}b"),
    ("line-sep-multi", "a\u{2028}b\nc"),
    ("para-sep", "a\u{2029}b"),
    ("bidi", "a\u{200e}b\u{202e}c\u{2066}d"),
    ("bidi-multi", "a\u{202e}b\nc"),
    ("bom", "a\u{feff}b"),
    ("esc", "a\u{1b}[1mb"),
    ("cr", "a\rb"),
    ("del", "a\u{7f}b"),
    ("nel", "a\u{85}b"),
    ("dash", "-x"),
    ("dashdash", "--x"),
    ("true", "true"),
    ("null", "null"),
    ("inf", "inf"),
    ("digit", "1abc"),
    ("negnum", "-1"),
    ("empty", ""),
    ("space", "a b"),
    ("equals", "a=b"),
    ("braces", "{a}"),
    ("hash", "#a"),
    ("slash", "a//b"),
];

/// Write each control, bidi, line or paragraph separator, and byte order mark
/// character as `<U+XXXX>`, so the snapshot file holds no invisible
/// character. Newlines stay. The spec itself already writes some of them as
/// KDL's `\u{..}`, so the marker differs from it. No spec contains `<U+`, so
/// the view is lossless.
fn visible(spec: &str) -> String {
    assert!(
        !spec.contains("<U+"),
        "the marker would be ambiguous: {spec}"
    );
    spec.chars().fold(String::new(), |mut out, c| {
        let hidden = (c.is_control() && c != '\n')
            || matches!(
                c,
                '\u{200b}'..='\u{200f}'
                    | '\u{2028}'..='\u{202e}'
                    | '\u{2060}'..='\u{2069}'
                    | '\u{feff}'
            );
        if hidden {
            out.push_str(&format!("<U+{:04X}>", u32::from(c)));
        } else {
            out.push(c);
        }
        out
    })
}

fn every() -> Command {
    let command = Command::new("every")
        .version("1.2.3")
        .long_version("1.2.3 (abc)")
        .about("Every attribute.")
        .long_about("Every attribute.\n\nA second paragraph.")
        .before_help("before")
        .before_long_help("before long")
        .after_help("after")
        .after_long_help("after long")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand_value_name("VERB")
        .subcommand_help_heading("Verbs")
        .next_line_help(true)
        .flatten_help(true)
        .disable_help_subcommand(true)
        .args_conflicts_with_subcommands(true)
        .subcommand_negates_reqs(true)
        .group(
            ArgGroup::new("mode")
                .args(["fast", "slow"])
                .required(false)
                .multiple(false),
        );
    let command = every_flags(command);
    let command = every_options(command);
    every_subcommands(command)
}

fn every_flags(command: Command) -> Command {
    command
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .action(ArgAction::Count)
                .global(true)
                .help("More output")
                .long_help("More output.\nRepeat it."),
        )
        .arg(
            Arg::new("fast")
                .long("fast")
                .action(ArgAction::SetTrue)
                .conflicts_with("slow")
                .help_heading("Speed")
                .display_order(3),
        )
        .arg(
            Arg::new("slow")
                .long("slow")
                .action(ArgAction::SetTrue)
                .hide(true),
        )
        .arg(
            Arg::new("no-pager")
                .long("no-pager")
                .action(ArgAction::SetFalse)
                .visible_alias("nopager")
                .alias("np"),
        )
        .arg(
            Arg::new("short-only")
                .long("short-only")
                .hide_short_help(true)
                .exclusive(true)
                .action(ArgAction::SetTrue),
        )
}

fn every_options(command: Command) -> Command {
    command
        .arg(
            Arg::new("color")
                .long("color")
                .short('c')
                .value_name("WHEN")
                .require_equals(true)
                .num_args(0..=1)
                .value_parser([
                    PossibleValue::new("auto").help("Detect"),
                    PossibleValue::new("always").alias("yes"),
                    PossibleValue::new("never").hide(true),
                ])
                .default_value("auto")
                .env("EVERY_COLOR")
                .ignore_case(true),
        )
        .arg(
            Arg::new("tag")
                .long("tag")
                .short('t')
                .action(ArgAction::Append)
                .value_delimiter(',')
                .visible_short_alias('T'),
        )
        .arg(
            Arg::new("path")
                .long("path")
                .value_hint(ValueHint::DirPath)
                .hide_default_value(true)
                .default_value(".")
                .hide_env(true)
                .env("EVERY_PATH"),
        )
        .arg(
            Arg::new("secret")
                .long("secret")
                .env("EVERY_SECRET")
                .hide_env_values(true)
                .hide_long_help(true),
        )
        .arg(
            Arg::new("pair")
                .long("pair")
                .num_args(2)
                .value_names(["KEY", "VALUE"]),
        )
        .arg(
            Arg::new("neg")
                .long("neg")
                .allow_negative_numbers(true)
                .allow_hyphen_values(true),
        )
        .arg(
            Arg::new("target")
                .required(true)
                .help("The target")
                .value_hint(ValueHint::FilePath),
        )
        .arg(
            Arg::new("rest")
                .num_args(1..)
                .last(true)
                .value_terminator(";"),
        )
}

fn every_subcommands(command: Command) -> Command {
    command
        .subcommand(
            Command::new("status")
                .about("Show status")
                .visible_alias("st")
                .alias("stat")
                .display_order(1)
                .arg(Arg::new("name").help("Name to show"))
                .arg(
                    Arg::new("all")
                        .long("all")
                        .short('a')
                        .action(ArgAction::SetTrue),
                ),
        )
        .subcommand(Command::new("hidden").hide(true))
        .subcommand(
            Command::new("run").arg(
                Arg::new("cmd")
                    .num_args(1..)
                    .trailing_var_arg(true)
                    .allow_hyphen_values(true),
            ),
        )
        .subcommand(
            Command::new("ext")
                .allow_external_subcommands(true)
                .dont_delimit_trailing_values(true)
                .allow_missing_positional(true)
                .subcommand_precedence_over_arg(true)
                .args_override_self(true)
                .disable_help_flag(true)
                .disable_version_flag(true),
        )
        .subcommand(
            Command::new("multi")
                .multicall(false)
                .arg(Arg::new("x").long("x").required(true)),
        )
}

/// A toy CLI as a consumer writes it, with the shared flags.
#[derive(Parser)]
#[expect(dead_code, reason = "only the spec is read, never a parsed value")]
#[command(name = "toy", version = "1.0.0", about = "A toy CLI")]
struct Toy {
    #[command(flatten)]
    output: OutputArgs,
    #[command(subcommand)]
    command: ToyCommand,
}

#[derive(Subcommand)]
#[expect(dead_code, reason = "only the spec is read, never a parsed value")]
enum ToyCommand {
    /// Show the queue.
    Status,
    /// Check one file.
    Check {
        /// The file to check.
        path: std::path::PathBuf,
        /// Stop at the first problem.
        #[arg(long)]
        strict: bool,
    },
}

#[derive(Serialize)]
struct Done;

impl Present for Done {
    fn present(&self) -> Document {
        Document::new()
    }
}

/// The spec `App` prints when mise asks the task `toy` for it. The renderer
/// records what `usage::spec` returns for the command `App` hands it, and
/// returns it unchanged, so `App` prints it too.
fn mounted_toy() -> String {
    let printed = Rc::new(RefCell::new(String::new()));
    let recorded = Rc::clone(&printed);
    let code = App::<Toy>::new("toy")
        .mounted_as("toy")
        .usage_spec(move |command, bin| {
            let spec = usage::spec(command, bin);
            recorded.replace(spec.clone());
            spec
        })
        .run_from(["toy", "--usage-spec=toy"], |_| Ok(Done));
    assert_eq!(code, ExitCode::SUCCESS);
    printed.take()
}

/// Append one corpus entry: a `// id` comment line, then the spec and a
/// blank line.
fn entry(out: &mut String, id: &str, spec: &str) {
    for line in [format!("// {id}").as_str(), spec] {
        out.push_str(line);
        out.push('\n');
    }
}

#[test]
fn every_attribute() {
    insta::assert_snapshot!("every", visible(&usage::spec(every(), "every")));
}

#[test]
fn a_mounted_toy() {
    insta::assert_snapshot!("toy", visible(&mounted_toy()));
}

#[test]
fn hostile_help_text() {
    let corpus = HOSTILE.iter().fold(String::new(), |mut out, (id, text)| {
        let command = Command::new("h").arg(
            Arg::new(*id)
                .long(*id)
                .help(*text)
                .action(ArgAction::SetTrue),
        );
        entry(&mut out, id, &usage::spec(command, "h"));
        out
    });
    insta::assert_snapshot!("hostile_help", visible(&corpus));
}

#[test]
fn hostile_default_values() {
    let corpus = HOSTILE.iter().filter(|(_, text)| !text.is_empty()).fold(
        String::new(),
        |mut out, (id, text)| {
            let command =
                Command::new("h").arg(Arg::new(*id).long(*id).value_name("V").default_value(*text));
            entry(&mut out, id, &usage::spec(command, "h"));
            out
        },
    );
    insta::assert_snapshot!("hostile_default", visible(&corpus));
}
