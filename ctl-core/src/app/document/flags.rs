//! The two flags every command takes, and the clash with a CLI's own.

use std::path::PathBuf;

use clap::builder::PossibleValuesParser;
use clap::{Arg, Command, ValueHint, value_parser};

use crate::flags::env_name;
use crate::format::declares;
use crate::input;

/// The id of `--input`.
pub(super) const INPUT: &str = "input";
/// The id of `--input-format`.
pub(super) const INPUT_FORMAT: &str = "input_format";

/// The help heading both flags sit under.
const HEADING: &str = "Input";

/// Whether `command` or a subcommand at any depth declares `--input` or
/// `--input-format` already, by id, long name or long alias.
/// A second flag of either name would make Clap's own checks fail.
pub(in crate::app) fn clashes(command: &Command) -> bool {
    [INPUT, "input-format", INPUT_FORMAT]
        .iter()
        .any(|name| declares(command, name))
}

/// `command` with `--input <FILE>` and `--input-format <NAME>`, both global
/// so every subcommand takes them. They read `<PREFIX>_INPUT` and
/// `<PREFIX>_INPUT_FORMAT` when `prefix` is known.
///
/// `--input-format` takes the names of the input formats `Input::open`
/// reads through, so a CLI that installed its own set offers those.
pub(in crate::app) fn add(command: Command, prefix: Option<&str>) -> Command {
    let formats = input::default_formats();
    let mut file = Arg::new(INPUT)
        .long("input")
        .value_name("FILE")
        .value_parser(value_parser!(PathBuf))
        .value_hint(ValueHint::FilePath)
        .global(true)
        .help_heading(HEADING)
        .help("Fill this command's arguments from a document; `-` reads stdin");
    let mut format = Arg::new(INPUT_FORMAT)
        .long("input-format")
        .value_name("NAME")
        .value_parser(PossibleValuesParser::new(
            formats.names().collect::<Vec<_>>(),
        ))
        .global(true)
        .help_heading(HEADING)
        .help("The document's format, which stdin needs; a file's extension names it otherwise");
    if let Some(prefix) = prefix {
        file = file.env(env_name(prefix, "input"));
        format = format.env(env_name(prefix, "input-format"));
    }
    command.arg(file).arg(format)
}
