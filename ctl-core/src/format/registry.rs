#[cfg(feature = "app")]
use std::iter;
use std::{fmt, io};

#[cfg(feature = "cli")]
use clap::{Arg, ArgAction, Command};

#[cfg(feature = "cli")]
use crate::flags::env_name;
#[cfg(feature = "cli")]
use crate::format::FormatParser;
use crate::format::{CSV, Format, FormatName, JSON, MARKDOWN, PRETTY, TSV};

/// The formats a CLI offers, and which one is the default.
///
/// [`Self::builtin`] holds `pretty`, `json`, `tsv`, `csv`, and `markdown`. A
/// CLI adds a format, replaces a built-in of the same name, or removes one,
/// then installs the set with `App::formats`. Help, completion, the Usage spec,
/// and parse errors list exactly the names registered here. Every builder takes
/// and returns `self`, so a later call replaces an earlier one.
///
/// The set is not `Copy`: it owns the list. A `View` holds one registered
/// format, never the set.
///
/// ```
/// use ctl_core::format::{FormatName, Formats};
///
/// let formats = Formats::builtin().remove(FormatName::JSON);
/// assert_eq!(
///     formats.names().collect::<Vec<_>>(),
///     [
///         FormatName::PRETTY,
///         FormatName::TSV,
///         FormatName::CSV,
///         FormatName::MARKDOWN
///     ]
/// );
/// ```
#[derive(Clone)]
pub struct Formats {
    registered: Vec<&'static dyn Format>,
    terminal: FormatName,
    piped: FormatName,
    stream: FormatName,
}

impl Formats {
    /// `pretty`, `json`, `tsv`, `csv`, and `markdown`. `pretty` is the default
    /// on a terminal and `tsv` is the default when stdout is not one. A CLI
    /// that keeps `pretty` when piped calls
    /// `piped_default(FormatName::PRETTY)`. `json` is the default under
    /// `--stream`, which [`Self::stream_default`] changes.
    #[must_use]
    pub fn builtin() -> Self {
        Self {
            registered: vec![&PRETTY, &JSON, &TSV, &CSV, &MARKDOWN],
            terminal: FormatName::PRETTY,
            piped: FormatName::TSV,
            stream: FormatName::JSON,
        }
    }

    /// Register `format`, or replace the registered format of the same name.
    /// A replacement keeps its place in the list.
    ///
    /// A replacement of `json` that gives its format no stream form of its own
    /// (`Format::stream`) writes each record under `--stream` as `write` does,
    /// not as one line per record.
    #[must_use]
    #[expect(
        clippy::should_implement_trait,
        reason = "a builder named for what it does, as `remove` is; `Add` has no meaning here"
    )]
    pub fn add(mut self, format: &'static dyn Format) -> Self {
        let name = format.name();
        match self.registered.iter_mut().find(|held| held.name() == name) {
            Some(held) => *held = format,
            None => self.registered.push(format),
        }
        self
    }

    /// Take the format called `name` out. A default that names it then fails
    /// when a run resolves it, so change the default too.
    #[must_use]
    pub fn remove(mut self, name: FormatName) -> Self {
        self.registered.retain(|held| held.name() != name);
        self
    }

    /// The format used when stdout is a terminal and none was named.
    #[must_use]
    pub fn terminal_default(mut self, name: FormatName) -> Self {
        self.terminal = name;
        self
    }

    /// The format used when stdout is not a terminal and none was named.
    /// Setting it to the terminal default turns the piped switch off.
    #[must_use]
    pub fn piped_default(mut self, name: FormatName) -> Self {
        self.piped = name;
        self
    }

    /// The format used under `--stream` when none was named, whether stdout
    /// is a terminal or not. Setting it to a name that is not registered makes
    /// `--stream` with no format a usage error.
    ///
    /// The default is `json`, which writes one line per record. A CLI that
    /// replaces `json` with a format that has no stream form of its own gets
    /// each record written by that format's `write`, not one line per record.
    #[must_use]
    pub fn stream_default(mut self, name: FormatName) -> Self {
        self.stream = name;
        self
    }

    /// The registered format called `name`.
    #[must_use]
    pub fn get(&self, name: FormatName) -> Option<&'static dyn Format> {
        self.registered
            .iter()
            .copied()
            .find(|format| format.name() == name)
    }

    /// The registered names, in the order help lists them.
    pub fn names(&self) -> impl Iterator<Item = FormatName> + '_ {
        self.registered.iter().map(|format| format.name())
    }

    /// Each long flag that names a format, with the name: `json` first, then
    /// the shorthand of every other registered format.
    ///
    /// The flag structs declare `--json` and the matches record it as `json`,
    /// so it names `json` whatever the registry holds. A format that replaces
    /// `json` is chosen by that flag, and one that removes it makes the flag
    /// a usage error.
    #[cfg(feature = "app")]
    pub(crate) fn shorthands(&self) -> impl Iterator<Item = (&'static str, FormatName)> + '_ {
        let json = FormatName::JSON.as_str();
        iter::once((json, FormatName::JSON)).chain(self.registered.iter().filter_map(
            move |format| {
                format
                    .shorthand()
                    .filter(|long| *long != json)
                    .map(|long| (long, format.name()))
            },
        ))
    }

    /// The default format for a run whose stdout is, or is not, a terminal.
    ///
    /// # Errors
    ///
    /// An [`io::ErrorKind::InvalidInput`] error when the default names a format
    /// that is not registered.
    pub(crate) fn default_format(&self, terminal: bool) -> io::Result<&'static dyn Format> {
        let name = if terminal { self.terminal } else { self.piped };
        self.get(name)
            .ok_or_else(|| unregistered("the default format", name))
    }

    /// The default format for a run under `--stream`.
    ///
    /// # Errors
    ///
    /// An [`io::ErrorKind::InvalidInput`] error when the stream default names
    /// a format that is not registered.
    pub(crate) fn stream_format(&self) -> io::Result<&'static dyn Format> {
        self.get(self.stream)
            .ok_or_else(|| unregistered("the stream default", self.stream))
    }

    /// The name of the stream default, registered or not.
    #[cfg(feature = "app")]
    pub(crate) const fn stream_name(&self) -> FormatName {
        self.stream
    }

    /// `command` with this registry's formats: the names `--format` accepts,
    /// and one global flag for each shorthand the command lacks.
    ///
    /// A command with no `format` argument is returned as it is. A shorthand
    /// that the command or any subcommand already declares, as a long flag, a
    /// long alias, or an argument id, gets no flag anywhere in the CLI, since
    /// a global flag reaches every subcommand; `--format <name>` still selects
    /// that format. Call it once every subcommand is attached, as `App` does.
    /// Each shorthand flag reads `<PREFIX>_<LONG>`, with the prefix taken from
    /// the variable `--format` reads: `TOY_FORMAT` gives `TOY`. A `--format`
    /// with no variable binds none. The `--json` flag the flag structs add is
    /// hidden when `json` is not registered, and its variable stays bound, so
    /// setting it is a usage error that lists the registered names.
    ///
    /// Without `App`, call it on the command a CLI parses so a custom set
    /// applies there too.
    #[cfg(feature = "cli")]
    #[must_use]
    pub fn augment(&self, command: Command) -> Command {
        let Some(format) = command.get_arguments().find(|arg| arg.get_id() == "format") else {
            return command;
        };
        let prefix = format
            .get_env()
            .and_then(|name| name.to_str())
            .and_then(|name| name.strip_suffix("_FORMAT"))
            .map(str::to_owned);
        let parser = FormatParser::new(
            self.registered
                .iter()
                .map(|format| (format.name(), format.about()))
                .collect(),
        );
        let hide_json = self.get(FormatName::JSON).is_none();
        let mut command = command.mut_args(|arg| {
            if arg.get_id() == "format" {
                arg.value_parser(parser.clone())
            } else if hide_json && arg.get_id() == "json" {
                arg.hide(true)
            } else {
                arg
            }
        });
        for format in &self.registered {
            let Some(long) = format.shorthand() else {
                continue;
            };
            if declares(&command, long) {
                continue;
            }
            let mut flag = Arg::new(long)
                .long(long)
                .action(ArgAction::SetTrue)
                .global(true)
                .help(format!("Shorthand for `--format {}`", format.name()))
                .help_heading("Output");
            if let Some(prefix) = &prefix {
                flag = flag.env(env_name(prefix, long));
            }
            command = command.arg(flag);
        }
        command
    }
}

/// Whether `command` or a subcommand at any depth declares `name` as a long
/// flag, a long alias, or an argument id, so a second flag of that name would
/// make Clap's own checks fail.
#[cfg(feature = "cli")]
fn declares(command: &Command, name: &str) -> bool {
    command.get_arguments().any(|arg| {
        arg.get_long() == Some(name)
            || arg.get_id() == name
            || arg
                .get_all_aliases()
                .is_some_and(|aliases| aliases.contains(&name))
    }) || command
        .get_subcommands()
        .any(|subcommand| declares(subcommand, name))
}

/// The error for a name that is not registered.
pub(crate) fn unregistered(what: &str, name: FormatName) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("{what} `{name}` is not a registered format"),
    )
}

/// Names the registered formats and both defaults: a format has no `Debug`.
impl fmt::Debug for Formats {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Formats")
            .field(
                "names",
                &self
                    .registered
                    .iter()
                    .map(|format| format.name())
                    .collect::<Vec<_>>(),
            )
            .field("terminal", &self.terminal)
            .field("piped", &self.piped)
            .field("stream", &self.stream)
            .finish()
    }
}
