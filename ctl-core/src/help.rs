//! Styled help extracted from Clap and rendered as a semantic document.
//!
//! [`App`] prints help for you. These functions serve a CLI that
//! drives its own lifecycle. [`document`] turns a Clap command into a
//! [`Document`], and the `try_emit` functions print it when the arguments
//! ask for help. Help uses the same renderer, theme, and width policy as
//! command output.
//!
//! ```
//! use clap::Parser;
//! use ctl_core::help::document;
//!
//! #[derive(Parser)]
//! #[command(name = "toy", about = "A toy")]
//! struct Cli {}
//!
//! let help = document(<Cli as clap::CommandFactory>::command());
//! assert!(!help.is_empty());
//! ```
#![cfg_attr(feature = "app", doc = "", doc = "[`App`]: crate::App")]
#![cfg_attr(not(feature = "app"), doc = "", doc = "[`App`]: crate#cargo-features")]

use std::ffi::OsString;
use std::{env, io};

use clap::{Command, CommandFactory};
use ctl_term::Stream;

use crate::document::{Document, Role, Section, Table, Text};
use crate::parser;
use crate::render::RenderOptions;

const NARROW_HELP_WIDTH: u16 = 64;

/// Print styled help to stdout when the process argv asks for it, and return
/// `true`. Return `false` and print nothing otherwise.
///
/// Colour comes from the `--color` and `--no-color` flags in argv. Use
/// [`try_emit_with_options`] to control the width and style policy.
///
/// # Errors
///
/// Returns the error from writing help to stdout.
///
/// # Panics
///
/// Panics when the command disables `-h/--help` or `-V/--version`.
pub fn try_emit<C: CommandFactory>() -> io::Result<bool> {
    let raw = env::args_os().collect::<Vec<_>>();
    let color = parser::parsed_output(C::command(), &raw).color;
    try_emit_with_options::<C>(RenderOptions::new(color))
}

/// Like [`try_emit`], rendering with `options` in place of the defaults.
///
/// # Errors
///
/// Returns the error from writing help to stdout.
///
/// # Panics
///
/// Panics when the command disables `-h/--help` or `-V/--version`.
pub fn try_emit_with_options<C: CommandFactory>(options: RenderOptions) -> io::Result<bool> {
    let args = env::args_os()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    try_emit_from_with_options::<C>(&args, options)
}

/// Like [`try_emit`], reading `args` in place of the process argv. The first
/// item is the binary name.
///
/// # Errors
///
/// Returns the error from writing help to stdout.
///
/// # Panics
///
/// Panics when the command disables `-h/--help` or `-V/--version`.
pub fn try_emit_from<C: CommandFactory>(args: &[String]) -> io::Result<bool> {
    let raw = args.iter().map(OsString::from).collect::<Vec<_>>();
    let color = parser::parsed_output(C::command(), &raw).color;
    try_emit_from_with_options::<C>(args, RenderOptions::new(color))
}

/// Like [`try_emit`], reading `args` and rendering with `options`.
///
/// # Errors
///
/// Returns the error from writing help to stdout.
///
/// # Panics
///
/// Panics when the command disables `-h/--help` or `-V/--version`.
pub fn try_emit_from_with_options<C: CommandFactory>(
    args: &[String],
    options: RenderOptions,
) -> io::Result<bool> {
    try_emit_grouped(&C::command(), args, options, &[])
}

/// Print the root command's help to stderr, as shown for a bare invocation of
/// a command that requires input.
///
/// Colour and width are decided for stderr, not stdout. Help printed after a
/// usage error then fits the terminal stderr writes to, even when stdout is a
/// pipe.
///
/// # Errors
///
/// Returns the error from writing help to stderr.
pub fn emit_bare_with_options<C: CommandFactory>(options: RenderOptions) -> io::Result<()> {
    emit_bare_grouped(C::command(), options, &[])
}

/// [`emit_bare_with_options`] for `command`, with the root's commands under
/// `groups`. `App` passes the command it augmented with the CLI's formats.
pub(crate) fn emit_bare_grouped(
    command: Command,
    options: RenderOptions,
    groups: &[CommandGroup],
) -> io::Result<()> {
    options.print_for(Stream::Stderr, &render(command, Detail::Short, groups))
}

/// [`try_emit_from_with_options`] for `command`, with the root's commands
/// under `groups`.
pub(crate) fn try_emit_grouped(
    command: &Command,
    args: &[String],
    options: RenderOptions,
    groups: &[CommandGroup],
) -> io::Result<bool> {
    let raw = args.iter().map(OsString::from).collect::<Vec<_>>();
    if !parser::wants_help(command, &raw) {
        return Ok(false);
    }
    emit_selected(command, args, options, groups)?;
    Ok(true)
}

/// Help for the command `args` select, to stdout, as a help request prints it:
/// the summary for `-h`, and the long form for `--help` and the `help`
/// subcommand.
pub(crate) fn emit_selected(
    declared: &Command,
    args: &[String],
    options: RenderOptions,
    groups: &[CommandGroup],
) -> io::Result<()> {
    let (command, is_root) = help_command(declared, args);
    let groups = if is_root { groups } else { &[] };
    options.print_for(
        Stream::Stdout,
        &render(command, detail(declared, args), groups),
    )
}

/// The summary help for the command `args` select, to stderr. Clap asks for
/// it when a command that requires an argument or a subcommand gets neither.
#[cfg(feature = "app")]
pub(crate) fn emit_selected_to_stderr(
    declared: &Command,
    args: &[String],
    options: RenderOptions,
    groups: &[CommandGroup],
) -> io::Result<()> {
    let (command, is_root) = help_command(declared, args);
    let groups = if is_root { groups } else { &[] };
    options.print_for(Stream::Stderr, &render(command, Detail::Short, groups))
}

/// A heading over some of the root's commands in help, in the order the
/// heading names them. `App::command_group` builds one.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CommandGroup {
    pub(crate) heading: String,
    pub(crate) commands: Vec<String>,
}

/// How much help a request asks for, as Clap tells `-h` from `--help`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Detail {
    /// `-h`, a bare invocation, and a missing argument: `about`, each
    /// argument's `help`, and `after_help`.
    Short,
    /// `--help` and the `help` subcommand: `long_about`, each argument's
    /// `long_help`, and `after_long_help`, each falling back to its short
    /// form.
    Long,
}

/// The detail a help request in `args` asks for, as Clap reads the flag
/// that stopped it: the long form for `--help` and the `help` subcommand,
/// and the summary for `-h`.
fn detail(declared: &Command, args: &[String]) -> Detail {
    let raw = args.iter().map(OsString::from).collect::<Vec<_>>();
    if parser::wants_long_help(declared, &raw) {
        Detail::Long
    } else {
        Detail::Short
    }
}

/// The command `args` select, and whether it is the root.
fn help_command(declared: &Command, args: &[String]) -> (Command, bool) {
    let mut root = parser::with_help(declared.clone());
    root.build();
    select_command(root, declared.clone(), args.get(1..).unwrap_or(&[]))
}

fn select_command(mut command: Command, mut declared: Command, args: &[String]) -> (Command, bool) {
    let mut is_root = true;
    for value in args {
        if value == "-h" || value == "--help" {
            break;
        }
        if value.starts_with('-') {
            continue;
        }
        let declared_next = declared
            .get_subcommands()
            .find(|subcommand| subcommand.get_name() == value)
            .cloned();
        if value == "help" && declared_next.is_none() {
            continue;
        }
        let Some(next) = command
            .get_subcommands()
            .find(|subcommand| subcommand.get_name() == value)
            .cloned()
        else {
            continue;
        };
        command = next;
        is_root = false;
        if let Some(next) = declared_next {
            declared = next;
        }
    }
    (command, is_root)
}

/// Build the help document of a Clap command, as `-h` shows it.
///
/// The document holds the usage line, the about text, a `Commands` section,
/// an `Arguments` section, one section per option help heading (`Options`
/// when an option names none), and an `Examples` section from the command's
/// `after_help`. Each command shows its visible aliases beside its name.
/// `--help` shows `long_about`, each argument's `long_help`, and
/// `after_long_help` in their place when the command sets them.
///
/// ctl-core's `-h/--help` flag and `help` subcommand are added, and the
/// command is built first, so derived values such as the version flag are
/// present. Hidden subcommands, arguments, and options are left out.
///
/// # Panics
///
/// In a debug build, panics when Clap's `debug_assert` finds an invalid
/// command definition, as [`parser::verify`] does.
#[must_use]
pub fn document(command: Command) -> Document {
    render(command, Detail::Short, &[])
}

/// The help document at `detail`, with the commands under `groups` first.
fn render(command: Command, detail: Detail, groups: &[CommandGroup]) -> Document {
    let mut command = parser::with_help(command);
    command.build();
    let usage = command.render_usage().to_string();
    let mut output =
        Document::new().paragraph(Text::new().span(Role::Heading, usage.trim().to_owned()));
    let about = match detail {
        Detail::Short => command.get_about(),
        Detail::Long => command.get_long_about().or_else(|| command.get_about()),
    };
    if let Some(about) = about {
        output = output.paragraph(about.to_string());
    }
    output = command_sections(output, &command, groups);
    output = argument_sections(output, &command, detail);
    let examples = match detail {
        Detail::Short => command.get_after_help(),
        Detail::Long => command
            .get_after_long_help()
            .or_else(|| command.get_after_help()),
    };
    if let Some(examples) = examples {
        let text = examples.to_string();
        output = output.section(Section::new(
            "Examples",
            Document::new().verbatim(text.trim_start_matches('\n')),
        ));
    }
    output
}

/// One section per group, in the order the groups come, then `Commands` for
/// the visible commands no group names. A name that matches no visible
/// command is skipped, so a hidden command stays hidden.
fn command_sections(mut output: Document, command: &Command, groups: &[CommandGroup]) -> Document {
    let visible = command
        .get_subcommands()
        .filter(|subcommand| !subcommand.is_hide_set())
        .collect::<Vec<_>>();
    for group in groups {
        let members = group.commands.iter().filter_map(|name| {
            visible
                .iter()
                .copied()
                .find(|subcommand| subcommand.get_name() == name)
        });
        output = commands_section(output, group.heading.clone(), members);
    }
    let rest = visible.iter().copied().filter(|subcommand| {
        !groups.iter().any(|group| {
            group
                .commands
                .iter()
                .any(|name| name == subcommand.get_name())
        })
    });
    commands_section(output, "Commands".to_owned(), rest)
}

/// `output` with a section of `commands` under `heading`, or unchanged when
/// there are none.
fn commands_section<'a>(
    output: Document,
    heading: String,
    commands: impl Iterator<Item = &'a Command>,
) -> Document {
    let table = commands.fold(Table::plain().token_column(0), |table, subcommand| {
        table.row([
            Text::plain(command_name(subcommand)),
            Text::plain(
                subcommand
                    .get_about()
                    .map_or_else(String::new, ToString::to_string),
            ),
        ])
    });
    if table.is_empty() {
        return output;
    }
    output.section(Section::new(
        heading,
        Document::new().table(table.stacked_below(NARROW_HELP_WIDTH, 1)),
    ))
}

/// The command's name, then its visible aliases: `ls, list`.
fn command_name(command: &Command) -> String {
    let mut name = command.get_name().to_owned();
    for alias in command.get_visible_aliases() {
        name.push_str(", ");
        name.push_str(alias);
    }
    name
}

/// The `Arguments` section, then one section per option help heading, with
/// `Help` last.
fn argument_sections(mut output: Document, command: &Command, detail: Detail) -> Document {
    let positionals = command
        .get_positionals()
        .filter(|arg| !arg.is_hide_set())
        .fold(Table::plain(), |table, arg| {
            table.row([
                Text::new(),
                Text::new().value(value_label(arg)),
                Text::new(),
                description(arg, detail),
            ])
        });
    if !positionals.is_empty() {
        output = output.section(Section::new(
            "Arguments",
            Document::new().table(positionals.stacked_below(NARROW_HELP_WIDTH, 3)),
        ));
    }

    let mut headings = Vec::<String>::new();
    for arg in command
        .get_arguments()
        .filter(|arg| !arg.is_positional() && !arg.is_hide_set() && arg.get_id().as_str() != "help")
    {
        let heading = arg
            .get_help_heading()
            .map_or_else(|| "Options".to_owned(), ToString::to_string);
        if !headings.contains(&heading) {
            headings.push(heading);
        }
    }
    if command
        .get_arguments()
        .any(|arg| arg.get_id().as_str() == "help")
    {
        headings.push("Help".into());
    }
    for heading in headings {
        let rows = command
            .get_arguments()
            .filter(|arg| {
                if heading == "Help" {
                    return arg.get_id().as_str() == "help";
                }
                !arg.is_positional()
                    && !arg.is_hide_set()
                    && arg.get_id().as_str() != "help"
                    && arg.get_help_heading().map_or("Options", |value| value) == heading
            })
            .fold(Table::plain(), |table, arg| {
                table.row([
                    arg.get_short()
                        .map_or_else(Text::new, |value| Text::new().token(format!("-{value}"))),
                    arg.get_long()
                        .map_or_else(Text::new, |value| Text::new().token(format!("--{value}"))),
                    Text::new().value(value_label(arg)),
                    description(arg, detail),
                ])
            });
        if !rows.is_empty() {
            output = output.section(Section::new(
                heading,
                Document::new().table(rows.stacked_below(NARROW_HELP_WIDTH, 3)),
            ));
        }
    }
    output
}

fn value_label(arg: &clap::Arg) -> String {
    if matches!(
        arg.get_action(),
        clap::ArgAction::SetTrue
            | clap::ArgAction::SetFalse
            | clap::ArgAction::Help
            | clap::ArgAction::Version
    ) {
        return String::new();
    }
    let names = arg.get_value_names().map_or_default(|names| {
        names
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(" ")
    });
    let choices = arg
        .get_possible_values()
        .into_iter()
        .filter(|value| !value.is_hide_set())
        .map(|value| value.get_name().to_owned())
        .collect::<Vec<_>>();
    if choices.is_empty() {
        names
    } else {
        format!("[{}]", choices.join("|"))
    }
}

/// The help text, `long_help` first at [`Detail::Long`], then `[default: …]`
/// and `[env: …]` notes in muted style. An option marked `hide_env` keeps its
/// variable out of help, as in clap.
fn description(arg: &clap::Arg, detail: Detail) -> Text {
    let help = match detail {
        Detail::Short => arg.get_help(),
        Detail::Long => arg.get_long_help().or_else(|| arg.get_help()),
    };
    let description = help.map_or_else(String::new, ToString::to_string);
    let defaults = arg
        .get_default_values()
        .iter()
        .map(|value| value.to_string_lossy())
        .collect::<Vec<_>>();
    let mut notes = Vec::new();
    if !defaults.is_empty()
        && !matches!(
            arg.get_action(),
            clap::ArgAction::SetTrue | clap::ArgAction::SetFalse
        )
    {
        notes.push(format!("[default: {}]", defaults.join(", ")));
    }
    if let Some(name) = arg.get_env().filter(|_| !arg.is_hide_env_set()) {
        notes.push(format!("[env: {}]", name.to_string_lossy()));
    }
    let mut separator = if description.is_empty() { "" } else { " " };
    let mut text = Text::plain(description);
    for note in notes {
        text = text.then(separator).muted(note);
        separator = " ";
    }
    text
}

#[cfg(test)]
mod tests {
    use clap::{CommandFactory, Parser};

    use super::{Detail, detail, document, help_command, try_emit_from};
    use crate::color::ColorMode;
    use crate::flags::{DryRunArgs, OutputArgs};
    use crate::render::RenderOptions;

    #[derive(Parser)]
    #[command(version, about = "toy ctl", arg_required_else_help = true)]
    struct Toy {
        #[command(flatten)]
        output: OutputArgs,
        #[command(flatten)]
        dry: DryRunArgs,
        #[command(subcommand)]
        command: ToyCmd,
    }

    #[derive(clap::Subcommand)]
    enum ToyCmd {
        /// Show status.
        Status(StatusArgs),
        /// Group commands.
        Group {
            #[command(subcommand)]
            command: GroupCmd,
        },
    }

    #[derive(clap::Subcommand)]
    enum GroupCmd {
        /// Show a nested item.
        Show,
    }

    #[derive(clap::Args)]
    struct StatusArgs {
        /// Domain text that may begin with a hyphen.
        #[arg(short = 'm', long, allow_hyphen_values = true)]
        message: Option<String>,
    }

    #[test]
    fn document_lists_commands_and_flags() {
        let text = document(Toy::command()).render(RenderOptions::new(ColorMode::Never).width(80));
        assert!(text.contains("Commands"));
        assert!(text.contains("status"));
        assert!(text.contains("--dry-run"));
        assert!(text.contains("--format"));
        assert!(text.contains("--no-color"));
    }

    #[test]
    fn colorless_help_has_no_ansi() {
        let text = document(Toy::command()).render(RenderOptions::new(ColorMode::Never).width(80));
        assert!(!text.contains('\u{1b}'));
    }

    #[test]
    fn long_usage_wraps_to_the_render_width() {
        let command = clap::Command::new("refresh")
            .bin_name("forkctl patch refresh")
            .arg(
                clap::Arg::new("rewrite")
                    .long("rewrite-below")
                    .action(clap::ArgAction::SetTrue),
            )
            .arg(clap::Arg::new("name").value_name("NAME"));
        let text = document(command).render(RenderOptions::new(ColorMode::Never).width(40));
        assert!(text.lines().all(|line| line.chars().count() <= 40));
        assert_eq!(
            text.lines().take(2).collect::<Vec<_>>(),
            ["Usage: forkctl patch refresh [OPTIONS]", "[NAME]"]
        );
    }

    #[test]
    fn help_names_a_variable_unless_it_is_hidden() {
        let command = clap::Command::new("toy")
            .arg(clap::Arg::new("config").long("config").env("TOY_CONFIG"))
            .arg(
                clap::Arg::new("token")
                    .long("token")
                    .env("TOY_TOKEN")
                    .hide_env(true),
            );
        let text = document(command).render(RenderOptions::new(ColorMode::Never).width(80));
        assert!(text.contains("[env: TOY_CONFIG]"), "{text}");
        assert!(!text.contains("TOY_TOKEN"), "{text}");
    }

    #[test]
    fn the_detail_follows_the_help_flag_clap_reads() {
        let cases = [
            (&["toy", "-h"][..], Detail::Short),
            (&["toy", "--help"][..], Detail::Long),
            (&["toy", "-h", "--help"][..], Detail::Short),
            (&["toy", "--help", "-h"][..], Detail::Long),
            (&["toy", "status", "-qh"][..], Detail::Short),
            (&["toy", "status", "-m", "--help", "-h"][..], Detail::Short),
            (&["toy", "help", "status"][..], Detail::Long),
        ];
        for (args, expected) in cases {
            let args = args.iter().copied().map(String::from).collect::<Vec<_>>();
            assert_eq!(detail(&Toy::command(), &args), expected, "{args:?}");
        }
    }

    #[test]
    fn help_subcommand_selects_the_requested_command() {
        let args = ["toy", "help", "status"].map(String::from);
        assert_eq!(help_command(&Toy::command(), &args).0.get_name(), "status");
    }

    #[test]
    fn subcommand_help_keeps_parent_usage_and_globals() {
        let (command, is_root) = help_command(
            &Toy::command(),
            &["toy", "status", "--help"].map(String::from),
        );
        assert_eq!(command.get_bin_name(), Some("ctl-core status"));
        assert!(!is_root);
        assert!(command.get_arguments().any(|arg| arg.get_id() == "format"));
    }

    #[test]
    fn nested_help_subcommand_selects_the_requested_command() {
        let args = ["toy", "group", "help", "show"].map(String::from);
        assert_eq!(help_command(&Toy::command(), &args).0.get_name(), "show");
    }

    #[test]
    fn try_emit_skips_without_help() {
        let args = ["toy", "status"]
            .into_iter()
            .map(String::from)
            .collect::<Vec<_>>();
        assert!(!try_emit_from::<Toy>(&args).unwrap());
    }

    #[test]
    fn try_emit_ignores_help_used_as_a_domain_value() {
        let args = ["toy", "status", "-m", "--help"]
            .into_iter()
            .map(String::from)
            .collect::<Vec<_>>();
        assert!(!try_emit_from::<Toy>(&args).unwrap());
    }

    #[test]
    fn try_emit_ignores_help_after_separator() {
        let args = ["toy", "status", "--", "--help"]
            .into_iter()
            .map(String::from)
            .collect::<Vec<_>>();
        assert!(!try_emit_from::<Toy>(&args).unwrap());
    }

    #[test]
    fn try_emit_does_not_claim_bare_invocation() {
        let args = ["toy"].map(String::from);
        assert!(!try_emit_from::<Toy>(&args).unwrap());
    }
}
