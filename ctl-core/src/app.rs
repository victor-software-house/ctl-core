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
//! A failure goes to stderr in each format's own error form, so stdout
//! holds nothing after a failed run. That covers a handler error, a parse
//! error, and a result whose [`Present::outcome`](view::Present::outcome) is an
//! error.
//!
//! A handler returns a [`Present`](view::Present) value, or a
//! [`Reply`](view::Reply) that is a value or a stream of
//! [`Records`](view::Records), and prints nothing. Help, errors, and results
//! share one renderer, one theme, and one width policy. A stream of records
//! is written as it is produced under `--stream`; see
//! [`Records`](view::Records).
//!
//! # Exit codes
//!
//! - A result exits with its [`Present::exit_code`](view::Present::exit_code),
//!   0 by default. Records exit 0.
//! - A handler error, or an error from the iterator of a stream of records,
//!   exits 1, or with the code of the [`CommandError`] in its chain. A JSON
//!   envelope carries that error's `kind` value.
//! - A parse error exits with Clap's code, 2 for a usage error, and its JSON
//!   envelope sets `kind` to `usage`.
//! - A bare invocation of a command that needs a subcommand or an argument
//!   prints the root help to stderr and exits 2.
//! - A reader that closed the output stream changes the code only under
//!   [`BrokenPipe::Fail`].

use std::ffi::OsString;
use std::io::{self, IsTerminal as _, Write as _};
use std::marker::PhantomData;
use std::process::ExitCode;
use std::{env, fmt};

use anyhow::Result;
use clap::error::ErrorKind;
use clap::parser::ValueSource;
use clap::{ArgMatches, Command, CommandFactory, Parser};

use crate::color::ColorMode;
#[cfg(feature = "completion")]
use crate::completion;
use crate::format::{FormatName, Formats, PRETTY, unknown_format, unknown_stream_default};
use crate::help::CommandGroup;
#[cfg(any(feature = "input-yaml", feature = "input-toml"))]
use crate::input::{self, Limits};
use crate::model::ErrorBody;
use crate::render::{self, RenderOptions};
use crate::theme::Theme;
#[cfg(feature = "usage")]
use crate::usage;
use crate::view::{self, JsonLayout, Respond, View};
use crate::{flags, help, parser};

mod command_error;

pub use command_error::CommandError;

/// The `kind` value of a parse error's JSON envelope when Clap reports a
/// usage error.
const USAGE_KIND: &str = "usage";

type BeforeParse = Box<dyn Fn(&[OsString], &Command) -> Option<ExitCode>>;
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
/// The set is closed, and the enum is not `#[non_exhaustive]`. A run that
/// failed keeps its code under either policy. So a policy decides only
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
    formats: Formats,
    automatic_width_buffer: Option<u16>,
    automatic_width_buffer_envs: Option<&'static [&'static str]>,
    minimum_automatic_width: Option<u16>,
    fallback_width: Option<Fallback>,
    styles: Option<RenderOptions>,
    json_layout: Option<JsonLayout>,
    json_layout_envs: Option<&'static [&'static str]>,
    theme: Option<Theme>,
    broken_pipe: BrokenPipe,
    command_groups: Vec<CommandGroup>,
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
    /// Build a CLI named `bin` with the built-in formats and automatic colour.
    ///
    /// `bin` prefixes error lines (`bin: message`) and names the binary in
    /// JSON error envelopes. The default view requests no format, so a run
    /// writes the format `--format`, a shorthand such as `--json`, or its
    /// variable names, and otherwise the default of [`Self::formats`].
    /// [`Self::view`] replaces the default view.
    #[must_use]
    pub fn new(bin: impl Into<String>) -> Self {
        Self {
            bin: bin.into(),
            before_parse: Vec::new(),
            select_view: Box::new(|_| View::requested(None, ColorMode::Auto)),
            formats: Formats::builtin(),
            automatic_width_buffer: None,
            automatic_width_buffer_envs: None,
            minimum_automatic_width: None,
            fallback_width: None,
            styles: None,
            json_layout: None,
            json_layout_envs: None,
            theme: None,
            broken_pipe: BrokenPipe::Ignore,
            command_groups: Vec::new(),
            #[cfg(feature = "usage")]
            mounted_as: None,
            #[cfg(feature = "usage")]
            render_usage: Box::new(usage::spec),
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
    ///
    /// A view that only requests a format, as
    /// [`OutputArgs::view`](crate::OutputArgs::view) returns, takes the format
    /// `App` reads from `--format`, every registered shorthand, and their
    /// variables, and then the default for the terminal check. A view that
    /// names a registered format, as [`View::new`] builds, keeps it.
    #[must_use]
    pub fn view(mut self, select: impl Fn(&C) -> View + 'static) -> Self {
        self.select_view = Box::new(select);
        self
    }

    /// Install the formats this CLI offers, in place of
    /// [`Formats::builtin`]. `--format`'s help, completion, the Usage spec, and
    /// parse errors list exactly the registered names, each shorthand becomes
    /// a flag, and the registry's defaults apply when no format is named.
    #[must_use]
    pub fn formats(mut self, formats: Formats) -> Self {
        self.formats = formats;
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
    pub fn input_limits(self, limits: Limits) -> Self {
        input::set_default_limits(limits);
        self
    }

    /// Set the process-wide input formats as soon as it is called, not when
    /// the `App` runs, so `Input::open` in a handler reads the extensions
    /// `formats` names and no others. A CLI adds its own format with
    /// `Formats::builtin().add(&FORMAT)`.
    #[cfg(any(feature = "input-yaml", feature = "input-toml"))]
    #[must_use]
    pub fn input_formats(self, formats: input::Formats) -> Self {
        input::set_default_formats(formats);
        self
    }

    /// Take the theme, record style, table frame, row separation, column
    /// floor, table fit, and the variables they read from `styles`, for help,
    /// errors, and command output. Its colour and width settings are ignored.
    #[must_use]
    #[expect(
        clippy::large_types_passed_by_value,
        reason = "public signature that takes the options by value, as every RenderOptions builder does; a reference would break every caller"
    )]
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

    /// List the root's commands `commands` under `heading` in help, in that
    /// order, as Clap has no heading for a subcommand.
    ///
    /// Each call adds one group, and the groups show in the order added. The
    /// root's other visible commands follow under `Commands`, and a name that
    /// matches no visible command is skipped. Help for a subcommand shows its
    /// own commands under `Commands`.
    #[must_use]
    pub fn command_group(
        mut self,
        heading: impl Into<String>,
        commands: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.command_groups.push(CommandGroup {
            heading: heading.into(),
            commands: commands.into_iter().map(Into::into).collect(),
        });
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
        self.before_parse.push(Box::new(move |args, _| hook(args)));
        self
    }

    /// Answer `COMPLETE=<shell>` before parsing, so the script the
    /// `completion` verb prints can ask this binary for candidates.
    ///
    /// A run without the variable, or with it empty or `0`, parses as usual.
    /// See [the `completion` module](crate::completion).
    #[cfg(feature = "completion")]
    #[must_use]
    pub fn completion(mut self) -> Self
    where
        C: CommandFactory + 'static,
    {
        let bin = self.bin.clone();
        self.before_parse.push(Box::new(move |args, command| {
            completion::complete(&bin, args, command)
        }));
        self
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
    /// to stderr as an error (`bin: message`, or a JSON envelope). Its
    /// message includes the whole `anyhow` chain. It exits 1, or with the
    /// exit code of the first [`CommandError`] in the chain, whose `kind`
    /// value the envelope carries.
    ///
    /// # Panics
    ///
    /// Panics when the command disables `-h/--help` or `-V/--version`.
    #[must_use = "return it from main, or the command exits 0"]
    pub fn run<T>(self, execute: impl FnOnce(C) -> Result<T>) -> ExitCode
    where
        T: Respond,
    {
        self.run_from(env::args_os(), execute)
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
        T: Respond,
    {
        let raw = args.into_iter().map(Into::into).collect::<Vec<_>>();
        let words = raw
            .iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        let command = self.formats.augment(C::command());

        #[cfg(feature = "usage")]
        if let Some(task) = &self.mounted_as
            && let Some(spec_bin) = usage::spec_bin(words.iter().skip(1), task)
        {
            let mut command = command.clone();
            command.set_bin_name(&spec_bin);
            let spec = (self.render_usage)(command, &spec_bin);
            return self.written(view::write_stdout(spec.as_bytes()), ExitCode::SUCCESS);
        }

        for hook in &self.before_parse {
            if let Some(code) = hook(&raw, &command) {
                return code;
            }
        }

        let parsed = parser::parsed_output(command.clone(), &raw);
        let named = parsed.matches.as_ref().and_then(|matches| {
            requested_format(&command, matches, &self.formats).unwrap_or(Some(FormatName::JSON))
        });
        let streamed = parsed
            .matches
            .as_ref()
            .is_some_and(|matches| matches!(matches.try_get_one::<bool>("stream"), Ok(Some(true))));
        let raw_request = View::requested(named, parsed.color).stream(streamed);
        let raw_view = self.configured_view(
            self.resolve(&raw_request)
                .unwrap_or_else(|_| View::new(&PRETTY, parsed.color)),
        );
        for warning in raw_view.env_warnings(render::process_env) {
            let _ = writeln!(io::stderr().lock(), "{}: warning: {warning}", self.bin);
        }
        if words.len() == 1 && parser::requires_input(&command) {
            return self.written(
                help::emit_bare_grouped(command, &raw_view.render_options(), &self.command_groups),
                ExitCode::from(2),
            );
        }
        match help::try_emit_grouped(
            &command,
            &words,
            &raw_view.render_options(),
            &self.command_groups,
        ) {
            Ok(true) => return ExitCode::SUCCESS,
            Ok(false) => {}
            Err(error) => return self.written(Err(error), ExitCode::SUCCESS),
        }

        let error_view = self.parse_error_view(&raw, &raw_request, parsed.color);
        let parse = match flags::env_under_flags(parser::apply_defaults(command.clone()), &raw) {
            Ok(parse) => parse,
            Err(error) => return self.clap_error(&error, &error_view, &words, &command),
        };
        let matches = match parse.try_get_matches_from(&raw) {
            Ok(matches) => matches,
            Err(error) => return self.clap_error(&error, &error_view, &words, &command),
        };
        let requested = match requested_format(&command, &matches, &self.formats) {
            Ok(requested) => requested,
            Err(error) => return self.clap_error(&error, &error_view, &words, &command),
        };
        let cli = match C::from_arg_matches(&matches) {
            Ok(cli) => cli,
            Err(error) => return self.clap_error(&error, &error_view, &words, &command),
        };
        let view = self.configured_view((self.select_view)(&cli).with_request(requested));
        if view.is_stream()
            && view.names_no_format()
            && self.formats.get(self.formats.stream_name()).is_none()
        {
            let error =
                unknown_stream_default(self.formats.stream_name(), &mut self.formats.names())
                    .format(&mut command.clone());
            return self.clap_error(&error, &error_view, &words, &command);
        }
        let view = match self.resolve(&view) {
            Ok(view) => view,
            Err(error) => {
                let body = ErrorBody::new(&self.bin, error.to_string());
                return self.written(raw_view.write_error(&body), ExitCode::FAILURE);
            }
        };
        match execute(cli) {
            Ok(value) => {
                let code = ExitCode::from(value.reply_exit_code());
                match value.deliver(&view) {
                    Ok(None) => self.written(Ok(()), code),
                    Ok(Some(error)) => self.failed(&view, &error),
                    Err(error) => self.written(Err(error), code),
                }
            }
            Err(error) => self.failed(&view, &error),
        }
    }

    /// Write `error` to stderr in `view`'s error form and return its exit
    /// code: 1, or the code of the first [`CommandError`] in its chain, whose
    /// `kind` value the JSON envelope carries. A handler error and the error
    /// of a stream of records both end here.
    fn failed(&self, view: &View, error: &anyhow::Error) -> ExitCode {
        let mut body = ErrorBody::new(&self.bin, format!("{error:#}"));
        let mut code = ExitCode::FAILURE;
        if let Some(failure) = CommandError::find(error) {
            code = ExitCode::from(failure.code().get());
            if let Some(kind) = failure.kind_name() {
                body = body.kind(kind);
            }
        }
        self.written(view.write_error(&body), code)
    }

    /// `view` with its format settled against this CLI's registry and the
    /// terminal check on stdout.
    fn resolve(&self, view: &View) -> io::Result<View> {
        (*view).resolve(&self.formats, io::stdout().is_terminal())
    }

    /// Report a parse error, or print the help or version Clap stopped for.
    /// Help always comes from ctl-core's renderer, so it needs none of Clap's
    /// `help` or `color` features. `command` is the one `App` augmented with
    /// the registered formats.
    fn clap_error(
        &self,
        error: &clap::Error,
        view: &View,
        args: &[String],
        command: &Command,
    ) -> ExitCode {
        let code = exit_code(error.exit_code());
        let kind = error.kind();
        if !is_clap_display(kind) {
            let rendered = error.to_string();
            let mut body = ErrorBody::new(&self.bin, parse_error_message(&rendered));
            if error.exit_code() == 2 {
                body = body.kind(USAGE_KIND);
            }
            return self.written(view.write_error(&body), code);
        }
        let options = view.render_options();
        let written = if kind == ErrorKind::DisplayVersion {
            view::write_stdout(error.to_string().as_bytes())
        } else if kind == ErrorKind::DisplayHelp {
            help::emit_selected(command, args, &options, &self.command_groups)
        } else {
            help::emit_selected_to_stderr(command, args, &options, &self.command_groups)
        };
        self.written(written, code)
    }

    /// The view for a parse error.
    ///
    /// Clap's lenient read stops at the bad argument. A `--format json` after
    /// it goes unseen. The error would then print as text, coloured when
    /// `CLICOLOR_FORCE` or `FORCE_COLOR` is set, to a caller that asked for a
    /// machine format and pipes it to `jq`.
    ///
    /// So the shorthand of a registered format, `--json`, or a long
    /// `--format <name>` or `--format=<name>` that names a registered format,
    /// before `--` picks that format's error form, and a machine format never
    /// carries ANSI. Among `--format` values the last one Clap accepts wins in
    /// either direction, as it does in a full parse, so a later
    /// `--format pretty` undoes an earlier `--format json`. A shorthand asks
    /// for its format wherever it stands.
    ///
    /// The short `-f` is left out, because a CLI may bind it to another
    /// flag. The scan is lexical, so a domain option whose value happens to be
    /// `--format=json` also gets a JSON error. That only changes how a failed
    /// parse is reported.
    ///
    /// A `--stream` after the bad argument goes unseen the same way, and would
    /// leave the error in the piped default. So the scan finds it too, and when
    /// no format is named, the view is `request` resolved under `--stream`, in
    /// the stream default's error form. `request` is the view the lenient read
    /// built, before it settled a format. When the scan finds nothing,
    /// `request` resolves as it did.
    fn parse_error_view(&self, raw: &[OsString], request: &View, color: ColorMode) -> View {
        let mut args = raw.iter().skip(1).map(|arg| arg.to_str());
        let mut last = None;
        let mut streamed = request.is_stream();
        while let Some(arg) = args.next() {
            let Some(arg) = arg else { continue };
            if arg == "--" {
                break;
            }
            streamed |= arg == "--stream";
            let shorthand = arg.strip_prefix("--").and_then(|long| {
                self.formats
                    .shorthands()
                    .find_map(|(flag, name)| (flag == long).then_some(name))
            });
            if shorthand.is_some() {
                last = shorthand;
                break;
            }
            let value = if arg == "--format" {
                args.next().flatten()
            } else {
                arg.strip_prefix("--format=")
            };
            let named =
                value.and_then(|value| self.formats.names().find(|name| name.as_str() == value));
            if named.is_some() {
                last = named;
            }
        }
        let Some(format) = last.and_then(|name| self.formats.get(name)) else {
            return self.configured_view(
                self.resolve(&(*request).stream(streamed))
                    .unwrap_or_else(|_| View::new(&PRETTY, color)),
            );
        };
        let color = if format.machine() {
            ColorMode::Never
        } else {
            color
        };
        self.configured_view(View::new(format, color))
    }
}

/// The format the command line, or else the variables, name, among `--format`,
/// `--json`, and the shorthand of every registered format.
///
/// A value from the command line outranks one from a variable. Two flags, or
/// two variables, that name different formats are a usage error that names
/// both, because neither can be told to win. The derived flag structs
/// reconcile `--format` with `--json` the same way for a CLI without `App`,
/// but only `App` knows the shorthands and variables of a custom registry.
///
/// `--json` names `json` whatever the registry holds, so a CLI that replaces
/// `json` with a format that has no shorthand still selects it with `--json`.
///
/// # Errors
///
/// An `ArgumentConflict` error for two flags or two variables that name
/// different formats, and an `InvalidValue` error, exit code 2, for a name
/// the registry lacks, such as `--json` once `json` is removed. It names the
/// value, the variable it came from, and the registered names.
fn requested_format(
    command: &Command,
    matches: &ArgMatches,
    formats: &Formats,
) -> Result<Option<FormatName>, clap::Error> {
    struct Given {
        flag: String,
        env: String,
        name: FormatName,
        from_line: bool,
    }
    let env_of = |id: &str| {
        command
            .get_arguments()
            .find(|arg| arg.get_id() == id)
            .and_then(|arg| arg.get_env())
            .map_or_else(|| id.to_owned(), |name| name.to_string_lossy().into_owned())
    };
    let from = |id: &str| match matches.value_source(id) {
        Some(ValueSource::CommandLine) => Some(true),
        Some(ValueSource::EnvVariable) => Some(false),
        _ => None,
    };
    let mut given = Vec::new();
    if let Ok(Some(name)) = matches.try_get_one::<FormatName>("format")
        && let Some(from_line) = from("format")
    {
        given.push(Given {
            flag: format!("--format {name}"),
            env: env_of("format"),
            name: *name,
            from_line,
        });
    }
    for (long, name) in formats.shorthands() {
        if matches!(matches.try_get_one::<bool>(long), Ok(Some(true)))
            && let Some(from_line) = from(long)
        {
            given.push(Given {
                flag: format!("--{long}"),
                env: env_of(long),
                name,
                from_line,
            });
        }
    }
    for from_line in [true, false] {
        let mut set = given.iter().filter(|given| given.from_line == from_line);
        let Some(first) = set.next() else {
            continue;
        };
        if let Some(other) = set.find(|other| other.name != first.name) {
            let message = if from_line {
                format!(
                    "the arguments `{}` and `{}` name different formats\n",
                    first.flag, other.flag
                )
            } else {
                format!(
                    "the variables `{}` and `{}` name different formats\n",
                    first.env, other.env
                )
            };
            return Err(
                clap::Error::raw(ErrorKind::ArgumentConflict, message).format(&mut command.clone())
            );
        }
        if formats.get(first.name).is_none() {
            let variable = (!from_line).then_some(first.env.as_str());
            return Err(
                unknown_format(first.name.as_str(), variable, &mut formats.names())
                    .format(&mut command.clone()),
            );
        }
        return Ok(Some(first.name));
    }
    Ok(None)
}

/// Clap's rendered error without its leading `error: `, so a parse error reads
/// `bin: message` like every other error.
fn parse_error_message(rendered: &str) -> &str {
    rendered.trim().trim_prefix("error: ")
}

fn is_clap_display(kind: ErrorKind) -> bool {
    matches!(
        kind,
        ErrorKind::DisplayVersion
            | ErrorKind::DisplayHelp
            | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
    )
}

fn exit_code(code: i32) -> ExitCode {
    u8::try_from(code).map_or(ExitCode::FAILURE, ExitCode::from)
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::ffi::OsString;
    use std::io;
    use std::process::ExitCode;
    use std::rc::Rc;

    use clap::{CommandFactory, Parser, Subcommand};
    use ctl_text::anstyle::Effects;
    use serde::Serialize;

    use super::{App, BrokenPipe, is_clap_display, parse_error_message, requested_format};
    use crate::document::{Document, Fields};
    use crate::format::{FormatName, Formats, JSON, PRETTY};
    use crate::render::{RecordStyle, RenderOptions};
    use crate::view::{JsonLayout, Present, View};
    use crate::{ColorMode, OutputArgs, Role, Theme, parser};

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
                View::requested(cli.output.format, cli.output.color())
                    .quiet(cli.output.quiet)
                    .width(80)
            })
            .run_from(["toy", "status", "--quiet"], move |_| {
                observed.set(true);
                Ok(Status { pending: 0 })
            });
        assert_eq!(code, ExitCode::SUCCESS);
        assert!(ran.get());
    }

    /// A bad argument before `--format json` still gets a JSON error, so
    /// machine output stays plain whatever the colour environment says.
    #[test]
    fn a_parse_error_honours_a_later_json_format() {
        let app = App::<Cli>::new("toy");
        let pretty = View::new(&PRETTY, ColorMode::Always);
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
            (&["toy", "--json", "status", "--bogus"][..], true),
            (&["toy", "status", "--bogus", "--json"][..], true),
            (
                &["toy", "--json", "--bogus", "--format", "pretty"][..],
                true,
            ),
            (&["toy", "status", "--bogus", "--", "--json"][..], false),
            (&["toy", "status", "--bogus", "--format", "yaml"][..], false),
        ];
        for (argv, json) in cases {
            let raw: Vec<OsString> = argv.iter().map(OsString::from).collect();
            let view = app.parse_error_view(&raw, &pretty, ColorMode::Always);
            assert_eq!(view.format() == FormatName::JSON, json, "{argv:?}");
            assert_eq!(view.is_machine(), json, "{argv:?}");
            if json {
                assert_eq!(
                    view.render_options(),
                    RenderOptions::new(ColorMode::Never),
                    "{argv:?}"
                );
            }
        }
        let json_first = View::new(&JSON, ColorMode::Never);
        let raw: Vec<OsString> = ["toy", "--format", "json", "--bogus", "--format", "pretty"]
            .iter()
            .map(OsString::from)
            .collect();
        assert_eq!(
            app.parse_error_view(&raw, &json_first, ColorMode::Always)
                .format(),
            FormatName::PRETTY,
            "a later --format pretty undoes the pre-parse's JSON"
        );
    }

    #[test]
    fn a_parse_error_drops_claps_error_prefix() {
        assert_eq!(
            parse_error_message("error: unexpected argument found\n"),
            "unexpected argument found"
        );
        assert_eq!(parse_error_message("no prefix\n"), "no prefix");
    }

    #[test]
    fn pre_parse_hook_runs_before_clap() {
        let code = App::<Cli>::new("toy")
            .before_parse(|args| {
                args.iter()
                    .any(|arg| arg == "--complete")
                    .then_some(ExitCode::SUCCESS)
            })
            .run_from(["toy", "--complete"], |_| Ok(Status { pending: 0 }));
        assert_eq!(code, ExitCode::SUCCESS);
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
        assert_eq!(code, ExitCode::SUCCESS);
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
        let view = app.configured_view(View::new(&PRETTY, ColorMode::Never));
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
            .configured_view(View::new(&JSON, ColorMode::Never));
        assert_eq!(view.json_layout_with(compact), JsonLayout::Compact);
        let disabled = App::<Cli>::new("toy")
            .json_layout_envs(&[])
            .configured_view(View::new(&JSON, ColorMode::Never));
        assert_eq!(disabled.json_layout_with(compact), JsonLayout::Pretty);
    }

    #[test]
    fn app_styles_and_fallback_reach_every_view() {
        let app = App::<Cli>::new("toy")
            .fallback_width(None)
            .styles(RenderOptions::new(ColorMode::Never).record_style(RecordStyle::Boxed))
            .json_layout(JsonLayout::Compact);
        let options = app
            .configured_view(View::new(&PRETTY, ColorMode::Never))
            .render_options();
        assert_eq!(
            options,
            RenderOptions::new(ColorMode::Never)
                .fallback_width(None)
                .record_style(RecordStyle::Boxed)
        );
        assert_eq!(options.resolved_record_style(), RecordStyle::Boxed);
        let json = app
            .configured_view(View::new(&JSON, ColorMode::Never))
            .capture(&Status { pending: 1 })
            .unwrap();
        assert_eq!(json.text(), "{\"pending\":1}\n");
    }

    #[test]
    fn app_theme_reaches_every_view_over_the_styles_theme() {
        let underline = ctl_text::Style::new().effects(Effects::UNDERLINE);
        let theme = Theme::new().style(Role::Heading, underline);
        let app = App::<Cli>::new("toy")
            .theme(theme)
            .styles(RenderOptions::new(ColorMode::Never).record_style(RecordStyle::Boxed));
        let options = app
            .configured_view(View::new(&PRETTY, ColorMode::Always))
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
        let closed = || Err(io::Error::from(io::ErrorKind::BrokenPipe));
        let other = || Err(io::Error::other("disk full"));
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
        let app = App::<Cli>::new("toy")
            .view(|_| View::new(&PRETTY, ColorMode::Auto).quiet(true).width(80));
        let code = app.run_from(["toy", "status"], |_| Ok(Status { pending: 0 }));
        assert_eq!(code, ExitCode::SUCCESS);
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
        assert_eq!(code, ExitCode::from(2));
    }

    #[test]
    fn bare_invocation_is_usage_error() {
        let code = App::<Cli>::new("toy").run_from(["toy"], |_| Ok(Status { pending: 0 }));
        assert_eq!(code, ExitCode::from(2));
    }

    #[test]
    fn bare_invocation_executes_when_the_root_accepts_it() {
        let ran = Rc::new(Cell::new(false));
        let observed = Rc::clone(&ran);
        let code = App::<OptionalCli>::new("toy").run_from(["toy"], move |_| {
            observed.set(true);
            Ok(Status { pending: 0 })
        });
        assert_eq!(code, ExitCode::SUCCESS);
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
        assert_eq!(code, ExitCode::SUCCESS);
        assert!(ran.get());
    }

    #[test]
    fn parsed_output_uses_clap_to_separate_globals_from_domain_values() {
        let domain_value = ["toy", "status", "-m", "--format=json", "--bogus"].map(OsString::from);
        let formats = Formats::builtin();
        let named = |parsed: &parser::ParsedOutput| {
            let matches = parsed.matches.as_ref().unwrap();
            requested_format(&Cli::command(), matches, &formats).unwrap()
        };
        let parsed = parser::parsed_output(Cli::command(), &domain_value);
        assert_eq!(named(&parsed), None);
        assert_eq!(parsed.color, ColorMode::Auto);

        let global_after_subcommand =
            ["toy", "status", "-fjson", "-cnever", "--bogus"].map(OsString::from);
        let parsed = parser::parsed_output(Cli::command(), &global_after_subcommand);
        assert_eq!(named(&parsed), Some(FormatName::JSON));
        assert_eq!(parsed.color, ColorMode::Never);
    }
}
