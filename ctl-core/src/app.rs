//! The typed CLI lifecycle: [`App`] parses argv, runs a handler, and writes the
//! result through a [`View`].
//!
//! [`App::run`] reads the process argv and returns the [`ExitCode`] for `main`.
//! [`App::run_from`] takes explicit argv, which tests use. In order, a run:
//!
//! 1. answers a `--usage-spec` request from mise, then every
//!    [`App::before_parse`] hook;
//! 2. warns on stderr about an unusable style or layout variable;
//! 3. prints help, or the bare-command help, when the arguments ask for it;
//! 4. parses the arguments, with environment variables yielding to flags;
//! 5. calls the handler and writes its result, or its error, through the view.
//!
//! A handler returns a [`Present`] value and prints nothing. Help, errors, and
//! results share one renderer, one theme, and one width policy.
//!
//! # Exit codes
//!
//! - A result exits with its [`Present::exit_code`], 0 by default.
//! - A handler error exits 1.
//! - A parse error exits with Clap's code, 2 for a usage error.
//! - A bare invocation of a command that needs a subcommand or an argument
//!   prints the root help to stderr and exits 2.
//! - A reader that closed the output stream changes the code only under
//!   [`BrokenPipe::Fail`].

use std::ffi::OsString;
use std::fmt;
use std::io::{self, Write as _};
use std::marker::PhantomData;
use std::process::ExitCode;

use anyhow::Result;
#[cfg(feature = "usage")]
use clap::Command;
use clap::{CommandFactory, Parser};

use crate::color::ColorMode;
use crate::format::OutputFormat;
use crate::render::RenderOptions;
use crate::theme::Theme;
use crate::view::{JsonLayout, Present, View};

type BeforeParse = Box<dyn Fn(&[OsString]) -> Option<ExitCode>>;
type SelectView<C> = Box<dyn Fn(&C) -> View>;
#[cfg(feature = "usage")]
type RenderUsage = Box<dyn Fn(Command, &str) -> String>;

/// A fallback width the CLI chose, including `None` to disable it.
#[derive(Clone, Copy)]
struct Fallback(Option<u16>);

/// How [`App`] exits when the reader closes stdout or stderr before the
/// output is written, as `head -1` does.
///
/// A closed pipe never turns a failure into success, and never hides a more
/// specific code. A command that failed, or a parse error, keeps its own exit
/// code under either policy. A script never reads a failed run as a passing
/// one because its reader went away. A check such as `$? -eq 2` behaves the
/// same under both.
///
/// A CLI built on ctl-core ignores SIGPIPE, as every Rust program does by
/// default. A closed pipe then reaches it as a failed write, not as a signal.
///
/// The set is closed, and the enum is not `#[non_exhaustive]`: a run that
/// failed keeps its code under either policy, so a policy decides only
/// whether a successful run whose reader left still exits 0.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum BrokenPipe {
    /// Exit with the code the command would have exited with had every byte
    /// been read, and write nothing to stderr. A command that succeeded
    /// exits 0: the reader had what it wanted, and a script under
    /// `set -o pipefail` keeps going.
    #[default]
    Ignore,
    /// Exit 1 when the command succeeded, for a CLI that treats a reader that
    /// stopped early as an error. A command that
    /// already failed keeps its own code.
    Fail,
}

/// One ctl process: short-circuits, help, parsing, execution, and presentation.
///
/// `C` is the CLI's Clap parser. Build an `App` with [`Self::new`],
/// configure it with the builder methods, and finish with [`Self::run`].
/// Every builder takes `self`, so a setting set later replaces an earlier one.
/// The one exception: [`Self::theme`] wins over the theme in [`Self::styles`],
/// whichever is called first.
pub struct App<C> {
    bin: String,
    before_parse: Vec<BeforeParse>,
    select_view: SelectView<C>,
    automatic_width_buffer: Option<u16>,
    automatic_width_buffer_envs: Option<&'static [&'static str]>,
    minimum_automatic_width: Option<u16>,
    fallback_width: Option<Fallback>,
    styles: Option<RenderOptions>,
    json_layout: Option<JsonLayout>,
    json_layout_envs: Option<&'static [&'static str]>,
    theme: Option<Theme>,
    broken_pipe: BrokenPipe,
    #[cfg(feature = "usage")]
    mounted_as: Option<String>,
    #[cfg(feature = "usage")]
    render_usage: RenderUsage,
    marker: PhantomData<C>,
}

/// Names the binary only: the hooks are closures, which have no `Debug`.
impl<C> fmt::Debug for App<C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("App")
            .field("bin", &self.bin)
            .finish_non_exhaustive()
    }
}

impl<C> App<C> {
    /// Build a CLI named `bin` with pretty output and automatic colour.
    ///
    /// `bin` prefixes error lines (`bin: message`) and names the binary in
    /// JSON error envelopes. [`Self::view`] replaces the default view.
    #[must_use]
    pub fn new(bin: impl Into<String>) -> Self {
        Self {
            bin: bin.into(),
            before_parse: Vec::new(),
            select_view: Box::new(|_| View::new(OutputFormat::Pretty, ColorMode::Auto)),
            automatic_width_buffer: None,
            automatic_width_buffer_envs: None,
            minimum_automatic_width: None,
            fallback_width: None,
            styles: None,
            json_layout: None,
            json_layout_envs: None,
            theme: None,
            broken_pipe: BrokenPipe::Ignore,
            #[cfg(feature = "usage")]
            mounted_as: None,
            #[cfg(feature = "usage")]
            render_usage: Box::new(crate::usage::spec),
            marker: PhantomData,
        }
    }

    /// Pick the [`View`] for a run from the parsed CLI, usually from
    /// [`OutputArgs`](crate::OutputArgs).
    ///
    /// The `select` closure runs after a successful parse. Help and parse
    /// errors come before it, so they read the format and colour from the raw
    /// arguments instead. The width, style, and layout settings of this `App`
    /// are applied on top of the view `select` returns.
    #[must_use]
    pub fn view(mut self, select: impl Fn(&C) -> View + 'static) -> Self {
        self.select_view = Box::new(select);
        self
    }

    /// Override the number of cells subtracted from an automatically
    /// detected width, for help, errors, and command output. Zero disables the
    /// subtraction. An explicit view width stays exact.
    #[must_use]
    pub fn automatic_width_buffer(mut self, cells: u16) -> Self {
        self.automatic_width_buffer = Some(cells);
        self
    }

    /// Replace the ordered environment names that set the buffer, for every
    /// automatic-width render. An empty slice disables environment lookup.
    #[must_use]
    pub fn automatic_width_buffer_envs(mut self, names: &'static [&'static str]) -> Self {
        self.automatic_width_buffer_envs = Some(names);
        self
    }

    /// Set the floor, in cells, for an automatically detected width.
    #[must_use]
    pub fn minimum_automatic_width(mut self, cells: u16) -> Self {
        self.minimum_automatic_width = Some(cells);
        self
    }

    /// Lay out to `width` cells when no width is detected. `None` turns the
    /// fallback off, so tables render at their natural width.
    #[must_use]
    pub fn fallback_width(mut self, width: Option<u16>) -> Self {
        self.fallback_width = Some(Fallback(width));
        self
    }

    /// Set the process-wide default input limits as soon as it is called, not
    /// when the `App` runs. The limits of a single call win over them, and so
    /// does a repository's `.ctl/input.yaml` with `input-yaml`.
    #[cfg(any(feature = "input-yaml", feature = "input-toml"))]
    #[must_use]
    pub fn input_limits(self, limits: crate::input::Limits) -> Self {
        crate::input::set_default_limits(limits);
        self
    }

    /// Take the theme, record style, table frame, row separation, and the
    /// variables they read from `styles`, for help, errors, and command
    /// output. Its colour and width settings are ignored.
    #[must_use]
    pub fn styles(mut self, styles: RenderOptions) -> Self {
        self.styles = Some(styles);
        self
    }

    /// Lay out JSON output as `layout`. This beats the environment.
    #[must_use]
    pub fn json_layout(mut self, layout: JsonLayout) -> Self {
        self.json_layout = Some(layout);
        self
    }

    /// Replace the ordered environment names read for the JSON layout.
    /// An empty slice disables lookup.
    #[must_use]
    pub fn json_layout_envs(mut self, names: &'static [&'static str]) -> Self {
        self.json_layout_envs = Some(names);
        self
    }

    /// Draw help, errors, and command output in `theme`, in place of
    /// [`Theme::default`]. It also replaces the theme a [`Self::styles`]
    /// value carries. Colour is still the colour mode's choice:
    /// `--color never` writes no style.
    #[must_use]
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Exit as `policy` says when the reader closes stdout or stderr before
    /// the output is written. The default, [`BrokenPipe::Ignore`], exits
    /// with the command's own code and writes nothing to stderr.
    #[must_use]
    pub fn broken_pipe(mut self, policy: BrokenPipe) -> Self {
        self.broken_pipe = policy;
        self
    }

    /// The exit code once output meant to end with `code` was written.
    /// `written` is the result of the write.
    ///
    /// `code` is the answer when the write went through, and a failure when it
    /// did not. When the reader closed the stream, [`BrokenPipe::Ignore`]
    /// keeps `code`. [`BrokenPipe::Fail`] keeps a failing `code` and turns
    /// success into a failure. Nothing is reported on stderr, which may be the
    /// stream that closed.
    fn written(&self, written: io::Result<()>, code: ExitCode) -> ExitCode {
        match written {
            Ok(()) => code,
            Err(error) if error.kind() == io::ErrorKind::BrokenPipe => match self.broken_pipe {
                BrokenPipe::Fail if code == ExitCode::SUCCESS => ExitCode::FAILURE,
                BrokenPipe::Ignore | BrokenPipe::Fail => code,
            },
            Err(_) => ExitCode::FAILURE,
        }
    }

    fn configured_view(&self, mut view: View) -> View {
        if let Some(buffer) = self.automatic_width_buffer {
            view = view.automatic_width_buffer(buffer);
        }
        if let Some(names) = self.automatic_width_buffer_envs {
            view = view.automatic_width_buffer_envs(names);
        }
        if let Some(minimum) = self.minimum_automatic_width {
            view = view.minimum_automatic_width(minimum);
        }
        if let Some(Fallback(width)) = self.fallback_width {
            view = view.fallback_width(width);
        }
        if let Some(styles) = self.styles {
            view = view.styles(styles);
        }
        if let Some(theme) = self.theme {
            view = view.theme(theme);
        }
        if let Some(layout) = self.json_layout {
            view = view.json_layout(layout);
        }
        if let Some(names) = self.json_layout_envs {
            view = view.json_layout_envs(names);
        }
        view
    }

    /// Add a hook that runs before parsing and may end the run.
    ///
    /// A hook gets the raw argv, program name first. Returning `Some(code)`
    /// stops the run with that code. Hooks run in the order added, after a
    /// `--usage-spec` request and before help and parsing.
    #[must_use]
    pub fn before_parse(
        mut self,
        hook: impl Fn(&[OsString]) -> Option<ExitCode> + 'static,
    ) -> Self {
        self.before_parse.push(Box::new(hook));
        self
    }

    /// Answer `COMPLETE=<shell>` before parsing, so the script the
    /// `completion` verb prints can ask this binary for candidates.
    ///
    /// A run without the variable, or with it empty or `0`, parses as usual.
    /// See [the `completion` module](crate::completion).
    #[cfg(feature = "completion")]
    #[must_use]
    pub fn completion(self) -> Self
    where
        C: CommandFactory + 'static,
    {
        let bin = self.bin.clone();
        self.before_parse(move |args| crate::completion::complete::<C>(&bin, args))
    }

    /// Answer a `--usage-spec` request from mise for the task named `task`.
    ///
    /// When argv matches, the run prints the Usage spec to stdout and exits 0
    /// without parsing. See [the `usage` module](crate::usage) for the argv
    /// shape.
    #[cfg(feature = "usage")]
    #[must_use]
    pub fn mounted_as(mut self, task: impl Into<String>) -> Self {
        self.mounted_as = Some(task.into());
        self
    }

    /// Replace how the Usage document is rendered, keeping the
    /// short-circuit and the stdout write. The `render` callback receives the
    /// declared Clap command and the requested task name, and
    /// returns the document text.
    #[cfg(feature = "usage")]
    #[must_use]
    pub fn usage_spec(mut self, render: impl Fn(Command, &str) -> String + 'static) -> Self {
        self.render_usage = Box::new(render);
        self
    }
}

impl<C> App<C>
where
    C: Parser + CommandFactory,
{
    /// Run against the process argv and return the exit code for `main`.
    ///
    /// The handler `execute` gets the parsed CLI. An `Err` from it is written
    /// as an error (`bin: message`, or a JSON envelope) with exit code 1. Its
    /// message includes the whole `anyhow` chain.
    ///
    /// # Panics
    ///
    /// Panics when the command disables `-h/--help` or `-V/--version`.
    #[must_use = "return it from main, or the command exits 0"]
    pub fn run<T>(self, execute: impl FnOnce(C) -> Result<T>) -> ExitCode
    where
        T: Present,
    {
        self.run_from(std::env::args_os(), execute)
    }

    /// Run against explicit argv. The first item is the binary name.
    ///
    /// The result is the same as [`Self::run`]. Use it in tests with a fixed
    /// argv.
    ///
    /// # Panics
    ///
    /// Panics when the command disables `-h/--help` or `-V/--version`.
    #[must_use = "return it from main, or the command exits 0"]
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
            let mut command = C::command();
            command.set_bin_name(&spec_bin);
            let spec = (self.render_usage)(command, &spec_bin);
            return self.written(
                crate::view::write_stdout(spec.as_bytes()),
                ExitCode::SUCCESS,
            );
        }

        for hook in &self.before_parse {
            if let Some(code) = hook(&raw) {
                return code;
            }
        }

        let raw_view = self.configured_view(raw_view::<C>(&raw));
        for warning in raw_view.env_warnings(crate::render::process_env) {
            let _ = writeln!(std::io::stderr().lock(), "{}: warning: {warning}", self.bin);
        }
        if words.len() == 1 && crate::parser::requires_input::<C>() {
            return self.written(
                crate::help::emit_bare_with_options::<C>(raw_view.render_options()),
                ExitCode::from(2),
            );
        }
        match crate::help::try_emit_from_with_options::<C>(&words, raw_view.render_options()) {
            Ok(true) => return ExitCode::SUCCESS,
            Ok(false) => {}
            Err(error) => return self.written(Err(error), ExitCode::SUCCESS),
        }

        let command = crate::parser::apply_defaults(C::command());
        let command = match crate::flags::env_under_flags(command, &raw) {
            Ok(command) => command,
            Err(error) => {
                return self.clap_error(&error, &parse_error_view(&raw, raw_view), &words);
            }
        };
        let matches = match command.try_get_matches_from(&raw) {
            Ok(matches) => matches,
            Err(error) => {
                return self.clap_error(&error, &parse_error_view(&raw, raw_view), &words);
            }
        };
        let cli = match C::from_arg_matches(&matches) {
            Ok(cli) => cli,
            Err(error) => {
                return self.clap_error(&error, &parse_error_view(&raw, raw_view), &words);
            }
        };
        let view = self.configured_view((self.select_view)(&cli));
        match execute(cli) {
            Ok(value) => self.written(
                view.show(&value).map(|_| ()),
                ExitCode::from(value.exit_code()),
            ),
            Err(error) => self.written(
                view.emit_err(&self.bin, &format!("{error:#}")).map(|_| ()),
                ExitCode::FAILURE,
            ),
        }
    }

    /// Report a parse error, or print the help or version Clap stopped for.
    /// Help always comes from ctl-core's renderer, so it needs none of Clap's
    /// `help` or `color` features.
    fn clap_error(&self, error: &clap::Error, view: &View, args: &[String]) -> ExitCode {
        use clap::error::ErrorKind;

        let code = exit_code(error.exit_code());
        let kind = error.kind();
        if !is_clap_display(kind) {
            return self.written(
                view.emit_err(&self.bin, error.to_string().trim())
                    .map(|_| ()),
                code,
            );
        }
        let options = view.render_options();
        let written = if kind == ErrorKind::DisplayVersion {
            crate::view::write_stdout(error.to_string().as_bytes())
        } else if kind == ErrorKind::DisplayHelp {
            crate::help::emit_selected::<C>(args, options)
        } else {
            crate::help::emit_selected_to_stderr::<C>(args, options)
        };
        self.written(written, code)
    }
}

fn raw_view<C: CommandFactory>(raw: &[OsString]) -> View {
    let parsed = crate::parser::parsed_output::<C>(raw);
    View::new(parsed.format, parsed.color)
}

/// The view for a parse error.
///
/// Clap's lenient read stops at the bad argument. A `--format json` after it
/// goes unseen. The error would then print as text, coloured when
/// `CLICOLOR_FORCE` or `FORCE_COLOR` is set, to a caller that asked for JSON
/// and pipes it to `jq`.
///
/// So a long `--format json` or `--format=json` before `--` makes the error
/// JSON, which never carries ANSI. The last `--format` value Clap accepts
/// wins in either direction, as it does in a full parse, so a later
/// `--format pretty` undoes an earlier JSON choice.
///
/// The short `-f` is left out, because a CLI may bind it to another
/// flag. The scan is lexical, so a domain option whose value happens to be
/// `--format=json` also gets a JSON error. That only changes how a failed
/// parse is reported.
fn parse_error_view(raw: &[OsString], mut view: View) -> View {
    let mut args = raw.iter().skip(1).map(|arg| arg.to_str());
    let mut last = None;
    while let Some(arg) = args.next() {
        let value = match arg {
            Some("--") => break,
            Some("--format") => args.next().flatten(),
            Some(arg) => arg.strip_prefix("--format="),
            None => None,
        };
        if let Some(format) =
            value.and_then(|value| <OutputFormat as clap::ValueEnum>::from_str(value, false).ok())
        {
            last = Some(format);
        }
    }
    if let Some(format) = last {
        view = view.in_format(format);
        if format.is_json() {
            view = view.color(ColorMode::Never);
        }
    }
    view
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
    use std::ffi::OsString;
    use std::rc::Rc;

    use clap::{Parser, Subcommand};
    use serde::Serialize;

    use super::{App, BrokenPipe, is_clap_display, parse_error_view, raw_view};
    use crate::document::{Document, Fields};
    use crate::render::{RecordStyle, RenderOptions};
    use crate::view::{JsonLayout, Present, View};
    use crate::{ColorMode, OutputArgs, OutputFormat, Role, Theme};

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
        Status(StatusArgs),
    }

    #[derive(clap::Args)]
    struct StatusArgs {
        /// Domain text that may begin with a hyphen.
        #[arg(short = 'm', long, allow_hyphen_values = true)]
        message: Option<String>,
    }

    #[derive(Parser)]
    #[command(version, about = "optional toy")]
    struct OptionalCli {}

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

    /// A bad argument before `--format json` still gets a JSON error, so
    /// machine output stays plain whatever the colour environment says.
    #[test]
    fn a_parse_error_honours_a_later_json_format() {
        let pretty = View::new(OutputFormat::Pretty, ColorMode::Always);
        let cases = [
            (&["toy", "status", "--bogus", "--format", "json"][..], true),
            (&["toy", "status", "--bogus", "--format=json"][..], true),
            (&["toy", "status", "--bogus"][..], false),
            (
                &["toy", "status", "--bogus", "--", "--format", "json"][..],
                false,
            ),
            (
                &["toy", "status", "--bogus", "--format", "pretty"][..],
                false,
            ),
            (
                &["toy", "--bogus", "--format", "json", "--format", "pretty"][..],
                false,
            ),
            (
                &["toy", "--bogus", "--format=pretty", "--format", "json"][..],
                true,
            ),
        ];
        for (argv, json) in cases {
            let raw: Vec<OsString> = argv.iter().map(OsString::from).collect();
            let view = parse_error_view(&raw, pretty);
            assert_eq!(view.format().is_json(), json, "{argv:?}");
            if json {
                assert_eq!(
                    view.render_options(),
                    RenderOptions::new(ColorMode::Never),
                    "{argv:?}"
                );
            }
        }
        let json_first = View::new(OutputFormat::Json, ColorMode::Never);
        let raw: Vec<OsString> = ["toy", "--format", "json", "--bogus", "--format", "pretty"]
            .iter()
            .map(OsString::from)
            .collect();
        assert!(
            !parse_error_view(&raw, json_first).format().is_json(),
            "a later --format pretty undoes the pre-parse's JSON"
        );
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

    #[cfg(feature = "usage")]
    #[test]
    fn mounted_usage_accepts_a_custom_renderer() {
        use std::cell::RefCell;

        let observed = Rc::new(RefCell::new(None));
        let callback_observed = Rc::clone(&observed);
        let code = App::<Cli>::new("toy")
            .mounted_as("q")
            .usage_spec(move |command, bin| {
                *callback_observed.borrow_mut() =
                    Some((bin.to_owned(), command.get_bin_name().map(str::to_owned)));
                format!("custom {bin}\n")
            })
            .run_from(["toy", "--usage-spec=mounted"], |_| {
                Ok(Status { pending: 0 })
            });
        assert_eq!(code, std::process::ExitCode::SUCCESS);
        assert_eq!(
            observed.borrow().as_ref(),
            Some(&("mounted".to_owned(), Some("mounted".to_owned())))
        );
    }

    #[test]
    fn app_width_policy_configures_every_view() {
        let app = App::<Cli>::new("toy")
            .automatic_width_buffer(0)
            .automatic_width_buffer_envs(&["TOY_BUFFER", "LEGACY_BUFFER"])
            .minimum_automatic_width(8);
        let view = app.configured_view(View::new(OutputFormat::Pretty, ColorMode::Never));
        assert_eq!(
            view.render_options(),
            RenderOptions::new(ColorMode::Never)
                .automatic_width_buffer(0)
                .automatic_width_buffer_envs(&["TOY_BUFFER", "LEGACY_BUFFER"])
                .minimum_automatic_width(8)
        );
    }

    #[test]
    fn app_json_layout_envs_reach_every_view() {
        let compact = |name: &str| (name == "TOY_JSON").then(|| "compact".to_owned());
        let view = App::<Cli>::new("toy")
            .json_layout_envs(&["TOY_JSON"])
            .configured_view(View::new(OutputFormat::Json, ColorMode::Never));
        assert_eq!(view.json_layout_with(compact), JsonLayout::Compact);
        let disabled = App::<Cli>::new("toy")
            .json_layout_envs(&[])
            .configured_view(View::new(OutputFormat::Json, ColorMode::Never));
        assert_eq!(disabled.json_layout_with(compact), JsonLayout::Pretty);
    }

    #[test]
    fn app_styles_and_fallback_reach_every_view() {
        let app = App::<Cli>::new("toy")
            .fallback_width(None)
            .styles(RenderOptions::new(ColorMode::Never).record_style(RecordStyle::Boxed))
            .json_layout(JsonLayout::Compact);
        let options = app
            .configured_view(View::new(OutputFormat::Pretty, ColorMode::Never))
            .render_options();
        assert_eq!(
            options,
            RenderOptions::new(ColorMode::Never)
                .fallback_width(None)
                .record_style(RecordStyle::Boxed)
        );
        assert_eq!(options.resolved_record_style(), RecordStyle::Boxed);
        let json = app
            .configured_view(View::new(OutputFormat::Json, ColorMode::Never))
            .capture(&Status { pending: 1 })
            .unwrap();
        assert_eq!(json.text(), "{\"pending\":1}\n");
    }

    #[test]
    fn app_theme_reaches_every_view_over_the_styles_theme() {
        let underline = ctl_text::Style::new().effects(ctl_text::anstyle::Effects::UNDERLINE);
        let theme = Theme::new().style(Role::Heading, underline);
        let app = App::<Cli>::new("toy")
            .theme(theme)
            .styles(RenderOptions::new(ColorMode::Never).record_style(RecordStyle::Boxed));
        let options = app
            .configured_view(View::new(OutputFormat::Pretty, ColorMode::Always))
            .render_options();
        assert_eq!(
            options,
            RenderOptions::new(ColorMode::Always)
                .theme(theme)
                .record_style(RecordStyle::Boxed)
        );
        assert_eq!(options.resolved_record_style(), RecordStyle::Boxed);
    }

    #[test]
    fn a_closed_pipe_exits_as_the_policy_says() {
        use std::process::ExitCode;

        let closed = || Err(std::io::Error::from(std::io::ErrorKind::BrokenPipe));
        let other = || Err(std::io::Error::other("disk full"));
        let ignore = App::<Cli>::new("toy");
        assert_eq!(
            ignore.written(closed(), ExitCode::SUCCESS),
            ExitCode::SUCCESS
        );
        assert_eq!(
            ignore.written(closed(), ExitCode::from(2)),
            ExitCode::from(2)
        );
        assert_eq!(
            ignore.written(closed(), ExitCode::FAILURE),
            ExitCode::FAILURE
        );
        assert_eq!(
            ignore.written(other(), ExitCode::SUCCESS),
            ExitCode::FAILURE
        );
        assert_eq!(ignore.written(Ok(()), ExitCode::from(2)), ExitCode::from(2));
        let fail = App::<Cli>::new("toy").broken_pipe(BrokenPipe::Fail);
        assert_eq!(fail.written(closed(), ExitCode::SUCCESS), ExitCode::FAILURE);
        assert_eq!(fail.written(closed(), ExitCode::from(2)), ExitCode::from(2));
        assert_eq!(fail.written(closed(), ExitCode::FAILURE), ExitCode::FAILURE);
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
    fn bare_invocation_is_usage_error() {
        let code = App::<Cli>::new("toy").run_from(["toy"], |_| Ok(Status { pending: 0 }));
        assert_eq!(code, std::process::ExitCode::from(2));
    }

    #[test]
    fn bare_invocation_executes_when_the_root_accepts_it() {
        let ran = Rc::new(Cell::new(false));
        let observed = Rc::clone(&ran);
        let code = App::<OptionalCli>::new("toy").run_from(["toy"], move |_| {
            observed.set(true);
            Ok(Status { pending: 0 })
        });
        assert_eq!(code, std::process::ExitCode::SUCCESS);
        assert!(ran.get());
    }

    #[test]
    fn explicit_help_used_as_a_domain_value_reaches_execution() {
        let ran = Rc::new(Cell::new(false));
        let observed = Rc::clone(&ran);
        let code = App::<Cli>::new("toy").run_from(["toy", "status", "-m", "--help"], move |_| {
            observed.set(true);
            Ok(Status { pending: 0 })
        });
        assert_eq!(code, std::process::ExitCode::SUCCESS);
        assert!(ran.get());
    }

    #[test]
    fn raw_view_uses_clap_to_separate_globals_from_domain_values() {
        let domain_value = ["toy", "status", "-m", "--format=json", "--bogus"].map(OsString::from);
        let view = raw_view::<Cli>(&domain_value);
        assert_eq!(view.format(), OutputFormat::Pretty);
        assert_eq!(view.render_options(), RenderOptions::new(ColorMode::Auto));

        let global_after_subcommand =
            ["toy", "status", "-fjson", "-cnever", "--bogus"].map(OsString::from);
        let view = raw_view::<Cli>(&global_after_subcommand);
        assert_eq!(view.format(), OutputFormat::Json);
        assert_eq!(view.render_options(), RenderOptions::new(ColorMode::Never));
    }
}
