//! The Usage KDL that `--usage-spec` prints, for a fixed corpus of commands.
//!
//! Five kinds of command, each a reviewed snapshot in `tests/snapshots`:
//! one builder command that sets every attribute the spec reads, a toy
//! `App` mounted as the mise task `toy`, one command per hostile string,
//! once as help text and once as a default value, one command per word the
//! KDL v2 identifier grammar treats specially, as a default and a possible
//! value, and one command per value name that reads as usage syntax. The
//! snapshots hold the bytes the writer must keep, so a change to the spec shows
//! as a diff.
//!
//! Everything but the toy runs with the `usage` feature alone, so the plain
//! tier is held to the same bytes. The toy needs `app`.
#![allow(
    missing_docs,
    reason = "an integration test crate has no public API to document"
)]
#![cfg(feature = "usage")]

use std::fmt::Write as _;

use clap::builder::PossibleValue;
use clap::{Arg, ArgAction, ArgGroup, Command, ValueHint};
use ctl_core::usage;
use kdl::{KdlDocument, KdlNode, KdlValue};

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
    (
        "osc-link",
        "\u{1b}]8;;https://example.com\u{1b}\\link\u{1b}]8;;\u{1b}\\",
    ),
    ("csi-newline", "a\u{1b}[1\nb"),
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

/// The help text clap's `Display` gives for each hostile string that holds a
/// terminal control sequence or DEL, worked out by hand from the anstyle-parse
/// 1.0.0 state table. Every other string is its own help text.
const STRIPPED_HELP: &[(&str, &str)] = &[
    ("esc", "ab"),
    ("osc-link", "link"),
    ("csi-newline", "a\nb"),
    ("del", "ab"),
];

/// Words the KDL v2 identifier grammar treats specially, each used once as a
/// default and as the only possible value. Each must read back as a string with
/// the same text: number-like words, the reserved keywords bare and with `#`,
/// and words holding a character no identifier may hold.
const IDENTIFIER: &[&str] = &[
    "1", "-1", "+1", ".5", "-.5", "+.5", "1e3", "0x1F", "1_000", "true", "false", "null", "inf",
    "-inf", "nan", "#true", "#false", "#null", "#inf", "#-inf", "#nan", "", " ", "a b", "=", "a=b",
    "(", ")", "{", "}", "[", "]", "/", "\\", ";", "\"", "'",
];

/// Value names that read as usage syntax, each with its snapshot label: a
/// leading `--` and a space, two placeholders, brackets, two bracketed words,
/// and a trailing `...` or `…`.
/// clap means each literally, as the name of one value.
const VALUE_NAMES: &[(&str, &str)] = &[
    ("plain", "X"),
    ("double-dash", "-- X"),
    ("placeholders", "<A> <B>"),
    ("brackets", "[x]"),
    ("bracket-pair", "[A] [B]"),
    ("dots", "x..."),
    ("ellipsis", "x…"),
];

/// Write each control, bidi, line or paragraph separator, and byte order mark
/// character as `<U+XXXX>`, so the snapshot file holds no invisible
/// character. Newlines stay. The spec itself already writes some of them as
/// KDL's `\u{..}`, so the marker differs from it. No spec contains `<U+`, so
/// the view is lossless.
#[expect(
    clippy::expect_used,
    reason = "writing to a String cannot fail, and a test helper may panic"
)]
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
            write!(out, "<U+{:04X}>", u32::from(c)).expect("writing to a String cannot fail");
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

/// A toy [`App`](ctl_core::App) mounted as the mise task `toy`.
#[cfg(feature = "app")]
mod toy {
    use std::cell::RefCell;
    use std::rc::Rc;

    use ctl_core::prelude::*;
    use ctl_core::usage;
    use serde::Serialize;

    use super::visible;

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

    /// The spec `App` prints when mise asks the task `toy` for it. The
    /// renderer records what `usage::spec` returns for the command `App` hands
    /// it, and returns it unchanged, so `App` prints it too.
    pub(super) fn mounted_toy() -> String {
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

    #[test]
    fn a_mounted_toy() {
        insta::assert_snapshot!("toy", visible(&mounted_toy()));
    }
}

/// One command whose only flag is a switch with `text` as its help.
fn help_command(id: &'static str, text: &'static str) -> Command {
    Command::new("h").arg(Arg::new(id).long(id).help(text).action(ArgAction::SetTrue))
}

/// One command whose only flag takes a value, with `text` as its default.
fn default_command(id: &'static str, text: &'static str) -> Command {
    Command::new("h").arg(Arg::new(id).long(id).value_name("V").default_value(text))
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
fn hostile_help_text() {
    let corpus = HOSTILE.iter().fold(String::new(), |mut out, (id, text)| {
        entry(&mut out, id, &usage::spec(help_command(id, text), "h"));
        out
    });
    insta::assert_snapshot!("hostile_help", visible(&corpus));
}

#[test]
fn hostile_default_values() {
    let corpus = HOSTILE.iter().filter(|(_, text)| !text.is_empty()).fold(
        String::new(),
        |mut out, (id, text)| {
            entry(&mut out, id, &usage::spec(default_command(id, text), "h"));
            out
        },
    );
    insta::assert_snapshot!("hostile_default", visible(&corpus));
}

#[test]
fn identifier_grammar() {
    let corpus = IDENTIFIER.iter().fold(String::new(), |mut out, word| {
        entry(&mut out, &format!("{word:?}"), &identifier_spec(word));
        out
    });
    insta::assert_snapshot!("identifier", visible(&corpus));
}

/// One command with a flag, a required positional, and an optional one, that
/// each take one value named `name`. clap borrows a string only as
/// `&'static str`.
fn value_name_spec(name: &'static str) -> String {
    let command = Command::new("h")
        .arg(Arg::new("f").long("f").value_name(name))
        .arg(Arg::new("v").value_name(name).required(true))
        .arg(Arg::new("o").value_name(name));
    usage::spec(command, "h")
}

#[test]
fn value_names_that_read_as_usage_syntax() {
    let corpus = VALUE_NAMES
        .iter()
        .fold(String::new(), |mut out, (id, name)| {
            entry(&mut out, id, &value_name_spec(name));
            out
        });
    insta::assert_snapshot!("value_names", visible(&corpus));
}

/// A character KDL v2 forbids literally anywhere in a spec, as the Newline
/// and Disallowed Literal Code Points tables of
/// [KDL 2.0.0](https://github.com/kdl-org/kdl/blob/2.0.0/SPEC.md) list them:
/// every Newline but LF, which only breaks lines and sits in raw multi-line
/// strings, and every disallowed literal code point but the surrogates, which
/// no `char` holds. kdl 6.7.1 misses U+007F, so a parse alone cannot catch it.
fn is_forbidden_literal(c: char) -> bool {
    matches!(
        c,
        '\r' | '\u{85}'
            | '\u{b}'
            | '\u{c}'
            | '\u{2028}'
            | '\u{2029}'
            | '\0'..='\u{8}'
            | '\u{e}'..='\u{1f}'
            | '\u{7f}'
            | '\u{200e}'
            | '\u{200f}'
            | '\u{202a}'..='\u{202e}'
            | '\u{2066}'..='\u{2069}'
            | '\u{feff}'
    )
}

/// Every spec the corpus renders, with its label.
fn corpus() -> Vec<(String, String)> {
    let mut specs = vec![("every".to_owned(), usage::spec(every(), "every"))];
    #[cfg(feature = "app")]
    specs.push(("toy".to_owned(), toy::mounted_toy()));
    for (id, text) in HOSTILE {
        let help = usage::spec(help_command(id, text), "h");
        specs.push((format!("help {id}"), help));
        let default = usage::spec(default_command(id, text), "h");
        specs.push((format!("default {id}"), default));
    }
    for word in IDENTIFIER {
        specs.push((format!("identifier {word:?}"), identifier_spec(word)));
    }
    for (id, name) in VALUE_NAMES {
        specs.push((format!("value name {id}"), value_name_spec(name)));
    }
    specs
}

/// One command whose only flag takes `word` as its default and its one
/// possible value. clap borrows a string only as `&'static str`.
fn identifier_spec(word: &'static str) -> String {
    let command = Command::new("h").arg(
        Arg::new("v")
            .long("v")
            .value_parser([PossibleValue::new(word)])
            .default_value(word),
    );
    usage::spec(command, "h")
}

/// The first `flag` node of a parsed spec.
fn flag(document: &KdlDocument) -> Option<&KdlNode> {
    document.get("flag")
}

#[test]
fn every_spec_parses_with_no_forbidden_literal() {
    for (label, spec) in corpus() {
        if let Err(error) = KdlDocument::parse_v2(&spec) {
            panic!("{label}: the spec does not parse as KDL v2: {error:?}\n{spec}");
        }
        assert!(
            !spec.chars().any(is_forbidden_literal),
            "{label}: the spec holds a literal KDL v2 forbids: {}",
            visible(&spec)
        );
    }
}

/// The help text clap's `Display` gives for the hostile string `id`.
fn stripped_help(id: &str, text: &'static str) -> &'static str {
    STRIPPED_HELP
        .iter()
        .find(|(stripped, _)| *stripped == id)
        .map_or(text, |&(_, help)| help)
}

#[test]
fn help_reads_back_as_clap_renders_it() {
    for (id, text) in HOSTILE {
        let spec = usage::spec(help_command(id, text), "h");
        let document = KdlDocument::parse_v2(&spec).expect("the spec parses");
        let help = flag(&document)
            .and_then(|node| node.get("help"))
            .and_then(KdlValue::as_string);
        let expected = stripped_help(id, text);
        assert_eq!(help, Some(expected), "help {id}: {}", visible(&spec));
    }
}

#[test]
fn defaults_read_back_unchanged() {
    for (id, text) in HOSTILE {
        let spec = usage::spec(default_command(id, text), "h");
        let document = KdlDocument::parse_v2(&spec).expect("the spec parses");
        let default = flag(&document)
            .and_then(|node| node.get("default"))
            .and_then(KdlValue::as_string);
        assert_eq!(default, Some(*text), "default {id}: {}", visible(&spec));
    }
}

#[test]
fn identifier_words_read_back_as_strings() {
    for word in IDENTIFIER {
        let spec = identifier_spec(word);
        let document = KdlDocument::parse_v2(&spec).expect("the spec parses");
        let node = flag(&document).expect("the spec has a flag");
        let default = node.get("default").and_then(KdlValue::as_string);
        assert_eq!(default, Some(*word), "default {word:?}: {spec}");
        let choice = node
            .children()
            .and_then(|children| children.get("arg"))
            .and_then(KdlNode::children)
            .and_then(|children| children.get("choices"))
            .and_then(|choices| choices.entries().first())
            .and_then(|entry| entry.value().as_string());
        assert_eq!(choice, Some(*word), "choice {word:?}: {spec}");
    }
}
