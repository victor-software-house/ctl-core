//! Fluent typed CLI lifecycle.

use std::ffi::OsString;
use std::fmt;
use std::marker::PhantomData;
use std::process::ExitCode;

use anyhow::Result;
#[cfg(feature = "usage")]
use clap::Command;
use clap::{CommandFactory, Parser};

use crate::color::ColorMode;
use crate::format::OutputFormat;
use crate::render::RenderOptions;
use crate::view::{JsonLayout, Present, View};

type BeforeParse = Box<dyn Fn(&[OsString]) -> Option<ExitCode>>;
type SelectView<C> = Box<dyn Fn(&C) -> View>;
#[cfg(feature = "usage")]
type RenderUsage = Box<dyn Fn(Command, &str) -> String>;

/// A fallback width the owner chose, including `None` to disable it.
#[derive(Clone, Copy)]
struct Fallback(Option<u16>);

/// One ctl process: short-circuits, help, parsing, execution, and presentation.
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
    /// Build a CLI with pretty automatic-color output.
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
            #[cfg(feature = "usage")]
            mounted_as: None,
            #[cfg(feature = "usage")]
            render_usage: Box::new(crate::usage::spec),
            marker: PhantomData,
        }
    }

    /// Select output, color, and quiet policy from the parsed CLI.
    #[must_use]
    pub fn view(mut self, select: impl Fn(&C) -> View + 'static) -> Self {
        self.select_view = Box::new(select);
        self
    }

    /// Override the automatic-width buffer for help, errors, and command
    /// output. Zero disables buffering; explicit view widths remain exact.
    #[must_use]
    pub fn automatic_width_buffer(mut self, columns: u16) -> Self {
        self.automatic_width_buffer = Some(columns);
        self
    }

    /// Replace the ordered environment names for every automatic-width render.
    /// An empty slice disables environment lookup.
    #[must_use]
    pub fn automatic_width_buffer_envs(mut self, names: &'static [&'static str]) -> Self {
        self.automatic_width_buffer_envs = Some(names);
        self
    }

    /// Set the floor for automatically detected effective widths.
    #[must_use]
    pub fn minimum_automatic_width(mut self, columns: u16) -> Self {
        self.minimum_automatic_width = Some(columns);
        self
    }

    /// Lay out to `width` when no width is detected; `None` disables it.
    #[must_use]
    pub fn fallback_width(mut self, width: Option<u16>) -> Self {
        self.fallback_width = Some(Fallback(width));
        self
    }

    /// Set the process-wide input limits when called. A repository's
    /// `.ctl/input.yaml` and a single call's limits both beat them.
    #[cfg(feature = "input")]
    #[must_use]
    pub fn input_limits(self, limits: crate::input::Limits) -> Self {
        crate::input::set_default_limits(limits);
        self
    }

    /// Take record style, list style, and row separation from `styles` for
    /// help, errors, and command output.
    #[must_use]
    pub fn styles(mut self, styles: RenderOptions) -> Self {
        self.styles = Some(styles);
        self
    }

    /// Lay out JSON output as `layout`.
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
        if let Some(layout) = self.json_layout {
            view = view.json_layout(layout);
        }
        if let Some(names) = self.json_layout_envs {
            view = view.json_layout_envs(names);
        }
        view
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

    /// Customize the mounted Usage document while keeping App's short-circuit
    /// and stream ownership. The callback receives the declared Clap graph and
    /// the requested mounted binary name.
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
    /// Run against process argv.
    #[must_use = "return it from main, or the command exits 0"]
    pub fn run<T>(self, execute: impl FnOnce(C) -> Result<T>) -> ExitCode
    where
        T: Present,
    {
        self.run_from(std::env::args_os(), execute)
    }

    /// Run against explicit argv. The first item is the binary name.
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
            return crate::view::write_stdout(spec.as_bytes(), ColorMode::Never)
                .map_or(ExitCode::FAILURE, |()| ExitCode::SUCCESS);
        }

        for hook in &self.before_parse {
            if let Some(code) = hook(&raw) {
                return code;
            }
        }

        let raw_view = self.configured_view(raw_view::<C>(&raw));
        for warning in raw_view.env_warnings(crate::render::process_env) {
            eprintln!("{}: warning: {warning}", self.bin);
        }
        if words.len() == 1 && crate::parser::requires_input::<C>() {
            return crate::help::emit_bare_with_options::<C>(raw_view.render_options())
                .map_or(ExitCode::FAILURE, |()| ExitCode::from(2));
        }
        match crate::help::try_emit_from_with_options::<C>(&words, raw_view.render_options()) {
            Ok(true) => return ExitCode::SUCCESS,
            Ok(false) => {}
            Err(_) => return ExitCode::FAILURE,
        }

        let mut command = crate::parser::apply_defaults(C::command());
        if raw_view.format.is_json() {
            command = command.color(clap::ColorChoice::Never);
        }
        let matches = match command.try_get_matches_from(&raw) {
            Ok(matches) => matches,
            Err(error) => return self.clap_error(&error, parse_error_view(&raw, raw_view)),
        };
        let cli = match C::from_arg_matches(&matches) {
            Ok(cli) => cli,
            Err(error) => return self.clap_error(&error, parse_error_view(&raw, raw_view)),
        };
        let view = self.configured_view((self.select_view)(&cli));
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
}

fn raw_view<C: CommandFactory>(raw: &[OsString]) -> View {
    let parsed = crate::parser::parsed_output::<C>(raw);
    View::new(parsed.format, parsed.color)
}

/// The view for a parse error. Clap's lenient read stops at the bad argument,
/// so a `--format json` after it goes unseen, and the error would print as
/// text, coloured when `CLICOLOR_FORCE` or `FORCE_COLOR` is set, to a caller
/// that asked for JSON and pipes it to `jq`. A long `--format json` or
/// `--format=json` before `--` makes the error JSON, which never carries
/// ANSI; the last `--format` wins, as it does in a full parse. The short `-f`
/// is left out: a consumer may bind it to another flag. The scan is lexical,
/// so a domain option whose value happens to be `--format=json` also gets a
/// JSON error; that only changes how a failed parse is reported.
fn parse_error_view(raw: &[OsString], mut view: View) -> View {
    if view.format.is_json() {
        return view;
    }
    let mut args = raw.iter().skip(1).map(|arg| arg.to_str());
    let mut last = None;
    while let Some(arg) = args.next() {
        let value = match arg {
            Some("--") => break,
            Some("--format") => args.next().flatten(),
            Some(arg) => arg.strip_prefix("--format="),
            None => None,
        };
        if value.is_some() {
            last = value;
        }
    }
    if last == Some("json") {
        view.format = OutputFormat::Json;
        view.color = ColorMode::Never;
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

    use super::{App, is_clap_display, parse_error_view, raw_view};
    use crate::document::{Document, Fields};
    use crate::render::{RecordStyle, RenderOptions};
    use crate::view::{JsonLayout, Present, View};
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
            assert_eq!(view.format.is_json(), json, "{argv:?}");
            if json {
                assert_eq!(view.color, ColorMode::Never, "{argv:?}");
            }
        }
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
        assert_eq!(view.explicit_automatic_width_buffer(), Some(0));
        assert_eq!(
            view.automatic_width_buffer_env_names(),
            ["TOY_BUFFER", "LEGACY_BUFFER"]
        );
        assert_eq!(view.automatic_width_minimum(), 8);
    }

    #[test]
    fn app_json_layout_envs_reach_every_view() {
        let compact = |name: &str| (name == "TOY_JSON").then(|| "compact".to_owned());
        let view = App::<Cli>::new("toy")
            .json_layout_envs(&["TOY_JSON"])
            .configured_view(View::new(OutputFormat::Json, ColorMode::Never));
        assert_eq!(view.layout_with(compact), JsonLayout::Compact);
        let disabled = App::<Cli>::new("toy")
            .json_layout_envs(&[])
            .configured_view(View::new(OutputFormat::Json, ColorMode::Never));
        assert_eq!(disabled.layout_with(compact), JsonLayout::Pretty);
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
        assert_eq!(options.fallback(), None);
        assert_eq!(options.record(), RecordStyle::Boxed);
        let json = app
            .configured_view(View::new(OutputFormat::Json, ColorMode::Never))
            .capture(&Status { pending: 1 })
            .unwrap();
        assert_eq!(json.text(), "{\"pending\":1}\n");
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
        assert_eq!(view.format, OutputFormat::Pretty);
        assert_eq!(view.color, ColorMode::Auto);

        let global_after_subcommand =
            ["toy", "status", "-fjson", "-cnever", "--bogus"].map(OsString::from);
        let view = raw_view::<Cli>(&global_after_subcommand);
        assert_eq!(view.format, OutputFormat::Json);
        assert_eq!(view.color, ColorMode::Never);
    }
}
