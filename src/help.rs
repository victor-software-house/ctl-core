//! Styled help extracted from Clap and rendered as a semantic document.

use std::io::{self, Write};

use clap::{Command, CommandFactory};

use crate::color::ColorMode;
use crate::document::{Document, Section, Table, Text};
use crate::render::RenderOptions;

const NARROW_HELP_WIDTH: u16 = 64;

/// Styled `-h` / `--help`. Returns `true` when help ran.
pub fn try_emit<C: CommandFactory>() -> io::Result<bool> {
    let raw = std::env::args_os().collect::<Vec<_>>();
    let args = raw
        .iter()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let color = crate::parser::parsed_output::<C>(&raw).color;
    try_emit_from_with_color::<C>(&args, color)
}

/// Same as [`try_emit`] with explicit argv.
pub fn try_emit_from<C: CommandFactory>(args: &[String]) -> io::Result<bool> {
    let raw = args
        .iter()
        .map(std::ffi::OsString::from)
        .collect::<Vec<_>>();
    let color = crate::parser::parsed_output::<C>(&raw).color;
    try_emit_from_with_color::<C>(args, color)
}

/// Explicit-argv help with a policy already recovered by Clap.
pub(crate) fn try_emit_from_with_color<C: CommandFactory>(
    args: &[String],
    color: ColorMode,
) -> io::Result<bool> {
    let raw = args
        .iter()
        .map(std::ffi::OsString::from)
        .collect::<Vec<_>>();
    if !crate::parser::wants_help::<C>(&raw) {
        return Ok(false);
    }
    let command = help_command::<C>(args);
    let output = document(command).render(RenderOptions::new(color));
    let mut stream = anstream::AutoStream::new(io::stdout().lock(), color.choice());
    stream.write_all(output.as_bytes())?;
    stream.flush()?;
    Ok(true)
}

/// Render root help to stderr for a bare invocation that requires input.
pub(crate) fn emit_bare<C: CommandFactory>(color: ColorMode) -> io::Result<()> {
    let output = document(C::command()).render(RenderOptions::new(color));
    let mut stream = anstream::AutoStream::new(io::stderr().lock(), color.choice());
    stream.write_all(output.as_bytes())?;
    stream.flush()
}

fn help_command<C: CommandFactory>(args: &[String]) -> Command {
    let declared = C::command();
    let mut root = declared.clone();
    root.build();
    select_command(root, declared, args.get(1..).unwrap_or(&[]))
}

fn select_command(mut command: Command, mut declared: Command, args: &[String]) -> Command {
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
        if let Some(next) = declared_next {
            declared = next;
        }
    }
    command
}

/// Extract one semantic help document from a Clap command.
#[must_use]
pub fn document(mut command: Command) -> Document {
    command.build();
    let usage = command.render_usage().to_string();
    let mut output = Document::new().heading(usage.trim().to_owned());
    if let Some(about) = command.get_about() {
        output = output.paragraph(about.to_string());
    }

    let commands = command
        .get_subcommands()
        .filter(|subcommand| !subcommand.is_hide_set())
        .fold(Table::plain().token_column(0), |table, subcommand| {
            table.row([
                Text::plain(subcommand.get_name()),
                Text::plain(
                    subcommand
                        .get_about()
                        .map_or_else(String::new, ToString::to_string),
                ),
            ])
        });
    if !commands.is_empty() {
        output = output.section(Section::new(
            "Commands",
            Document::new().table(commands.stacked_below(NARROW_HELP_WIDTH, 1)),
        ));
    }

    let positionals = command
        .get_positionals()
        .filter(|arg| !arg.is_hide_set())
        .fold(Table::plain(), |table, arg| {
            table.row([
                Text::new(),
                Text::new().value(value_label(arg)),
                Text::new(),
                description(arg),
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
                    description(arg),
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
    let names = arg
        .get_value_names()
        .map(|names| {
            names
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default();
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

fn description(arg: &clap::Arg) -> Text {
    let description = arg.get_help().map_or_else(String::new, ToString::to_string);
    let defaults = arg
        .get_default_values()
        .iter()
        .map(|value| value.to_string_lossy())
        .collect::<Vec<_>>();
    if defaults.is_empty()
        || matches!(
            arg.get_action(),
            clap::ArgAction::SetTrue | clap::ArgAction::SetFalse
        )
    {
        return Text::plain(description);
    }
    let separator = if description.is_empty() { "" } else { " " };
    Text::plain(description)
        .then(separator)
        .muted(format!("[default: {}]", defaults.join(", ")))
}

#[cfg(test)]
mod tests {
    use clap::{CommandFactory, Parser};

    use super::{document, help_command, try_emit_from};
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
    fn help_subcommand_selects_the_requested_command() {
        let args = ["toy", "help", "status"].map(String::from);
        assert_eq!(help_command::<Toy>(&args).get_name(), "status");
    }

    #[test]
    fn subcommand_help_keeps_parent_usage_and_globals() {
        let command = help_command::<Toy>(&["toy", "status", "--help"].map(String::from));
        assert_eq!(command.get_bin_name(), Some("ctl-core status"));
        assert!(command.get_arguments().any(|arg| arg.get_id() == "format"));
    }

    #[test]
    fn nested_help_subcommand_selects_the_requested_command() {
        let args = ["toy", "group", "help", "show"].map(String::from);
        assert_eq!(help_command::<Toy>(&args).get_name(), "show");
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
