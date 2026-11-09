//! Shell completion: one `completion <SHELL>` verb for a CLI.
//!
//! The verb prints a short registration script. The script does not hold the
//! command grammar. At each TAB the shell runs the CLI again with [`VAR`] set
//! to the shell's name, and [`App::completion`] prints the candidates before
//! Clap parses anything. Candidates therefore come from the binary that is
//! installed, including values a CLI computes at run time through
//! `clap_complete`'s `ArgValueCompleter`.
//!
//! A CLI flattens [`CompletionCommand`] into its subcommands, turns the hook
//! on, routes the verb to [`shell::view`], and returns the script. The example
//! needs `clap` with its `derive` feature, as the
//! [crate quick start](crate#quick-start) shows:
//!
//! ```no_run
//! use ctl_core::completion::CompletionCommand;
//! use ctl_core::prelude::*;
//! use ctl_core::shell;
//!
//! #[derive(Parser)]
//! struct Cli {
//!     #[command(flatten)]
//!     output: OutputArgs,
//!     #[command(subcommand)]
//!     command: Command,
//! }
//!
//! #[derive(Subcommand)]
//! enum Command {
//!     #[command(flatten)]
//!     Completion(CompletionCommand),
//! }
//!
//! fn main() -> ExitCode {
//!     App::<Cli>::new("toy")
//!         .completion()
//!         .view(|cli| match cli.command {
//!             Command::Completion(_) => shell::view(cli.output.format),
//!         })
//!         .run(|cli| match cli.command {
//!             Command::Completion(command) => Ok(command.script("toy")?),
//!         })
//! }
//! ```
//!
//! A user loads the script at shell start, so it always matches the
//! installed binary:
//!
//! | Shell | Startup file | Line |
//! |:--|:--|:--|
//! | bash | `~/.bashrc` | `source <(toy completion bash)` |
//! | zsh | `~/.zshrc`, after `compinit` | `source <(toy completion zsh)` |
//! | fish | `~/.config/fish/config.fish` | `toy completion fish \| source` |
//!
//! A Homebrew formula installs the same script at every install and upgrade
//! with `generate_completions_from_executable(bin/"toy", "completion")`, or
//! with `shell_parameter_format: :clap`, which sets [`VAR`] instead.
//!
//! [`App::completion`]: crate::app::App::completion
//! [`shell::view`]: crate::shell::view

use std::ffi::{OsStr, OsString};
use std::io::{self, Write as _};
use std::path::Path;
use std::process::ExitCode;

use clap::{CommandFactory, ValueEnum};
use clap_complete::CompleteEnv;
use clap_complete::env::{Bash, EnvCompleter, Fish, Shells, Zsh};

use crate::formatdoc;
use crate::shell::{Script, Shell};

/// The environment variable that turns a run into a completion request.
///
/// It is `clap_complete`'s own default, which Homebrew's `:clap` format
/// sets, so `COMPLETE=zsh toy` prints the same script as `toy completion
/// zsh`. An empty value or `0` leaves the run alone. `clap_complete` removes
/// the variable before it calls a value completer, so a child process never
/// answers a request meant for its parent.
pub const VAR: &str = "COMPLETE";

/// The shells a TAB-time request may name: those [`Shell`] holds.
const SHELLS: Shells<'static> = Shells(&[&Bash, &Zsh, &Fish]);

impl Shell {
    /// The registration script for `bin`, which the shell finds on `PATH`
    /// at each TAB.
    ///
    /// The script calls `bin` by name, never by the path that produced it.
    /// A script Homebrew writes from a versioned directory then keeps working
    /// after the next upgrade.
    ///
    /// # Errors
    ///
    /// Returns an error when the script `clap_complete` writes is not UTF-8,
    /// which a `bin` that is a `&str` never causes.
    pub fn script(self, bin: &str) -> io::Result<Script> {
        let mut buffer = Vec::new();
        self.completer()
            .write_registration(VAR, bin, bin, bin, &mut buffer)?;
        let mut text = String::from_utf8(buffer)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        match self {
            Self::Bash | Self::Fish => {}
            Self::Zsh => text.push_str(&zsh_autoload(bin)),
        }
        Ok(Script { shell: self, text })
    }

    fn completer(self) -> &'static dyn EnvCompleter {
        match self {
            Self::Bash => &Bash,
            Self::Zsh => &Zsh,
            Self::Fish => &Fish,
        }
    }
}

/// Lines that make the zsh script complete on the first TAB when zsh
/// autoloads it from `fpath`, where Homebrew installs it as `_<bin>`.
///
/// `clap_complete` 4.6.11 ends its zsh script with `compdef`, which binds the
/// completer for the next TAB and lists nothing for this one. Autoloaded,
/// the file runs as the function `_<bin>`, so these lines also complete now.
/// Sourced, `funcstack[1]` is the file, and they do nothing. Drop them when
/// a pinned `clap_complete` writes its own; the script test shows when.
fn zsh_autoload(bin: &str) -> String {
    formatdoc! {r#"
        if [[ ${{funcstack[1]}} == _{bin} ]]; then
            _clap_dynamic_completer_{name} "$@"
        fi
        "#,
        name = bin.replace('-', "_"),
    }
}

/// The `completion` verb, flattened into a CLI's subcommands with
/// `#[command(flatten)]`, so its name, help, and argument read the same in
/// every CLI built on ctl-core.
#[derive(Debug, Clone, PartialEq, Eq, clap::Subcommand)]
pub enum CompletionCommand {
    /// Print the script that loads shell completion for this command.
    Completion(CompletionArgs),
}

impl CompletionCommand {
    /// The registration script the user asked for, for `bin`.
    ///
    /// # Errors
    ///
    /// Returns the error [`Shell::script`] returns.
    pub fn script(&self, bin: &str) -> io::Result<Script> {
        match self {
            Self::Completion(args) => args.shell.script(bin),
        }
    }
}

/// The arguments of the `completion` verb.
#[derive(Debug, Clone, PartialEq, Eq, clap::Args)]
pub struct CompletionArgs {
    /// Shell to load completion into: bash, zsh, or fish.
    #[arg(value_enum)]
    pub shell: Shell,
}

/// Answer a completion request for `C` when [`VAR`] is set, or return
/// `None` for an ordinary run.
///
/// With no word after `--` the request is for the registration, which this
/// prints from [`Shell::script`] so it matches the verb byte for byte. With
/// words, `clap_complete` prints the candidates.
pub(crate) fn complete<C: CommandFactory>(bin: &str, args: &[OsString]) -> Option<ExitCode> {
    let requested = std::env::var_os(VAR).filter(|value| !value.is_empty() && value != "0")?;
    let words = args
        .iter()
        .skip(1)
        .skip_while(|arg| arg.as_os_str() != "--")
        .nth(1)
        .is_some();
    Some(if words {
        candidates::<C>(bin, args)
    } else {
        registration(bin, &requested)
    })
}

fn registration(bin: &str, requested: &OsStr) -> ExitCode {
    let name = Path::new(requested)
        .file_stem()
        .unwrap_or(requested)
        .to_string_lossy();
    let Ok(shell) = <Shell as ValueEnum>::from_str(&name, false) else {
        let expected = Shell::value_variants()
            .iter()
            .filter_map(ValueEnum::to_possible_value)
            .map(|value| value.get_name().to_owned())
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(
            io::stderr().lock(),
            "{bin}: error: unknown shell `{name}` in {VAR}, expected one of {expected}"
        );
        return ExitCode::from(2);
    };
    shell
        .script(bin)
        .and_then(|script| crate::view::write_stdout(script.text.as_bytes()))
        .map_or(ExitCode::FAILURE, |()| ExitCode::SUCCESS)
}

fn candidates<C: CommandFactory>(bin: &str, args: &[OsString]) -> ExitCode {
    let current_dir = std::env::current_dir().ok();
    match CompleteEnv::with_factory(|| crate::parser::with_help(C::command()))
        .var(VAR)
        .bin(bin)
        .completer(bin)
        .shells(SHELLS)
        .try_complete(args.iter().cloned(), current_dir.as_deref())
    {
        Ok(_) => ExitCode::SUCCESS,
        Err(error) => {
            let _ = error.print();
            u8::try_from(error.exit_code()).map_or(ExitCode::FAILURE, ExitCode::from)
        }
    }
}
