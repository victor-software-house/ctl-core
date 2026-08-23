//! Fluent typed CLI lifecycle.

use std::ffi::OsString;
use std::marker::PhantomData;
use std::process::ExitCode;

use anyhow::Result;
use clap::{CommandFactory, Parser};

use crate::color::ColorMode;
use crate::document::{Document, Notice, NoticeLevel, Text};
use crate::format::OutputFormat;
use crate::render::RenderOptions;
use crate::view::{Present, View};

type BeforeParse = Box<dyn Fn(&[OsString]) -> Option<ExitCode>>;
type SelectView<C> = Box<dyn Fn(&C) -> View>;

/// One ctl process: short-circuits, help, parsing, execution, and presentation.
pub struct App<C> {
    bin: String,
    before_parse: Vec<BeforeParse>,
    select_view: SelectView<C>,
    #[cfg(feature = "usage")]
    mounted_as: Option<String>,
    marker: PhantomData<C>,
}

impl<C> App<C> {
    /// Build a CLI with pretty automatic-color output.
    #[must_use]
    pub fn new(bin: impl Into<String>) -> Self {
        Self {
            bin: bin.into(),
            before_parse: Vec::new(),
            select_view: Box::new(|_| View::new(OutputFormat::Pretty, ColorMode::Auto)),
            #[cfg(feature = "usage")]
            mounted_as: None,
            marker: PhantomData,
        }
    }

    /// Select output, color, and quiet policy from the parsed CLI.
    #[must_use]
    pub fn view(mut self, select: impl Fn(&C) -> View + 'static) -> Self {
        self.select_view = Box::new(select);
        self
    }

    /// Add an ordered pre-parse short-circuit, such as dynamic completion.
    #[must_use]
    pub fn before_parse(
        mut self,
        hook: impl Fn(&[OsString]) -> Option<ExitCode> + 'static,
    ) -> Self {
        self.before_parse.push(Box::new(hook));
        self
    }

    /// Expose a mise Usage spec under a mounted task name.
    #[cfg(feature = "usage")]
    #[must_use]
    pub fn mounted_as(mut self, task: impl Into<String>) -> Self {
        self.mounted_as = Some(task.into());
        self
    }
}

impl<C> App<C>
where
    C: Parser + CommandFactory,
{
    /// Run against process argv.
    #[must_use]
    pub fn run<T>(self, execute: impl FnOnce(C) -> Result<T>) -> ExitCode
    where
        T: Present,
    {
        self.run_from(std::env::args_os(), execute)
    }

    /// Run against explicit argv. The first item is the binary name.
    #[must_use]
    pub fn run_from<T>(
        self,
        args: impl IntoIterator<Item = impl Into<OsString>>,
        execute: impl FnOnce(C) -> Result<T>,
    ) -> ExitCode
    where
        T: Present,
    {
        let raw = args.into_iter().map(Into::into).collect::<Vec<_>>();
        let words = raw
            .iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>();

        #[cfg(feature = "usage")]
        if let Some(task) = &self.mounted_as
            && let Some(spec_bin) = crate::usage::spec_bin(words.iter().skip(1), task)
        {
            let spec = crate::usage::spec(C::command(), &spec_bin);
            return crate::view::write_stdout(spec.as_bytes(), ColorMode::Never)
                .map_or(ExitCode::FAILURE, |()| ExitCode::SUCCESS);
        }

        for hook in &self.before_parse {
            if let Some(code) = hook(&raw) {
                return code;
            }
        }

        match crate::help::try_emit_from::<C>(&words) {
            Ok(true) => return ExitCode::SUCCESS,
            Ok(false) => {}
            Err(_) => return ExitCode::FAILURE,
        }

        let warnings = crate::flags::chassis_warnings(words.iter().skip(1).map(String::as_str));
        if self.emit_warnings(&warnings, &words).is_err() {
            return ExitCode::FAILURE;
        }
        let raw_view = raw_view(&words);
        let mut command = crate::parser::apply_defaults(C::command());
        if raw_view.format.is_json() {
            command = command.color(clap::ColorChoice::Never);
        }
        let matches = match command.try_get_matches_from(&raw) {
            Ok(matches) => matches,
            Err(error) => return self.clap_error(&error, raw_view),
        };
        let cli = match C::from_arg_matches(&matches) {
            Ok(cli) => cli,
            Err(error) => return self.clap_error(&error, raw_view),
        };
        let view = (self.select_view)(&cli);
        match execute(cli) {
            Ok(value) => view.show(&value).unwrap_or(ExitCode::FAILURE),
            Err(error) => view
                .emit_err(&self.bin, &format!("{error:#}"))
                .unwrap_or(ExitCode::FAILURE),
        }
    }

    fn clap_error(&self, error: &clap::Error, view: View) -> ExitCode {
        let code = exit_code(error.exit_code());
        if is_clap_display(error.kind()) {
            let _ = error.print();
            return code;
        }
        view.emit_err(&self.bin, error.to_string().trim())
            .map_or(ExitCode::FAILURE, |_| code)
    }

    fn emit_warnings(
        &self,
        warnings: &[crate::flags::FlagWarning],
        args: &[String],
    ) -> std::io::Result<()> {
        if warnings.is_empty() {
            return Ok(());
        }
        let document = warnings.iter().fold(Document::new(), |document, warning| {
            document.notice(Notice::new(
                NoticeLevel::Warning,
                Text::new()
                    .token(self.bin.clone())
                    .then(": ")
                    .then(warning.to_string()),
            ))
        });
        let color = ColorMode::from_args(args.iter().map(String::as_str));
        crate::view::write_stderr(document.render(RenderOptions::new(color)).as_bytes(), color)
    }
}

fn raw_view(words: &[String]) -> View {
    let args = words.iter().skip(1).map(String::as_str);
    View::new(
        OutputFormat::from_args(args.clone()),
        ColorMode::from_args(args),
    )
}

fn is_clap_display(kind: clap::error::ErrorKind) -> bool {
    matches!(
        kind,
        clap::error::ErrorKind::DisplayVersion
            | clap::error::ErrorKind::DisplayHelp
            | clap::error::ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
    )
}

fn exit_code(code: i32) -> ExitCode {
    u8::try_from(code).map_or(ExitCode::FAILURE, ExitCode::from)
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use clap::{Parser, Subcommand};
    use serde::Serialize;

    use super::{App, is_clap_display, raw_view};
    use crate::document::{Document, Fields};
    use crate::view::{Present, View};
    use crate::{ColorMode, OutputArgs, OutputFormat};

    #[derive(Parser)]
    #[command(version, about = "toy")]
    struct Cli {
        #[command(flatten)]
        output: OutputArgs,
        #[command(subcommand)]
        command: Command,
    }

    #[derive(Subcommand)]
    enum Command {
        /// Show status.
        Status,
    }

    #[derive(Parser)]
    #[command(version, about = "nested toy")]
    struct NestedCli {
        #[command(subcommand)]
        command: NestedCommand,
    }

    #[derive(Subcommand)]
    enum NestedCommand {
        /// Group commands.
        Group {
            #[command(subcommand)]
            command: GroupCommand,
        },
    }

    #[derive(Subcommand)]
    enum GroupCommand {
        /// Show status.
        Status,
    }

    #[derive(Serialize)]
    struct Status {
        pending: usize,
    }

    impl Present for Status {
        fn present(&self) -> Document {
            Document::new().fields(Fields::new().row("pending", self.pending.to_string()))
        }
    }

    #[test]
    fn parses_executes_and_suppresses_quiet_pretty() {
        let ran = Rc::new(Cell::new(false));
        let observed = Rc::clone(&ran);
        let code = App::<Cli>::new("toy")
            .view(|cli| {
                View::new(cli.output.format, cli.output.color())
                    .quiet(cli.output.quiet)
                    .width(80)
            })
            .run_from(["toy", "status", "--quiet"], move |_| {
                observed.set(true);
                Ok(Status { pending: 0 })
            });
        assert_eq!(code, std::process::ExitCode::SUCCESS);
        assert!(ran.get());
    }

    #[test]
    fn pre_parse_hook_runs_before_clap() {
        let code = App::<Cli>::new("toy")
            .before_parse(|args| {
                args.iter()
                    .any(|arg| arg == "--complete")
                    .then_some(std::process::ExitCode::SUCCESS)
            })
            .run_from(["toy", "--complete"], |_| Ok(Status { pending: 0 }));
        assert_eq!(code, std::process::ExitCode::SUCCESS);
    }

    #[test]
    fn defaults_are_pretty_auto() {
        let app = App::<Cli>::new("toy").view(|_| {
            View::new(OutputFormat::Pretty, ColorMode::Auto)
                .quiet(true)
                .width(80)
        });
        let code = app.run_from(["toy", "status"], |_| Ok(Status { pending: 0 }));
        assert_eq!(code, std::process::ExitCode::SUCCESS);
    }

    #[test]
    fn clap_help_kinds_stay_on_claps_display_path() {
        use clap::error::ErrorKind;

        assert!(is_clap_display(ErrorKind::DisplayVersion));
        assert!(is_clap_display(ErrorKind::DisplayHelp));
        assert!(is_clap_display(
            ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
        ));
        assert!(!is_clap_display(ErrorKind::UnknownArgument));
    }

    #[test]
    fn missing_nested_subcommand_uses_claps_help_exit() {
        let code =
            App::<NestedCli>::new("toy").run_from(["toy", "group"], |_| Ok(Status { pending: 0 }));
        assert_eq!(code, std::process::ExitCode::from(2));
    }

    #[test]
    fn raw_view_skips_binary_and_stops_at_separator() {
        let words = ["--format=json", "status", "--", "--color=always"].map(String::from);
        let view = raw_view(&words);
        assert_eq!(view.format, OutputFormat::Pretty);
        assert_eq!(view.color, ColorMode::Auto);
    }
}
