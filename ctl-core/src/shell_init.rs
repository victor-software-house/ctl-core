//! Shell integration: one `init <SHELL>` verb that hooks a CLI into the
//! user's shell.
//!
//! The verb prints a script that runs one command of the CLI in the
//! foreground, before the shell draws its first prompt. It never runs again in
//! that shell. The CLI declares the command once as a [`Hook`], and ctl-core
//! writes every line of shell syntax from it.
//!
//! The hook runs only when its variable is set to a non-empty value and the
//! binary is on `PATH`. A startup file that loads the script therefore stays
//! safe on a machine without the CLI, or outside the context the variable
//! marks.
//! Loading the script twice installs the hook once.
//!
//! A CLI flattens [`InitCommand`] into its subcommands, routes the verb to
//! [`shell::view`], and returns the script. The example needs `clap` with its
//! `derive` feature, as the [crate quick start](crate#quick-start) shows:
//!
//! ```no_run
//! use ctl_core::prelude::*;
//! use ctl_core::shell;
//! use ctl_core::shell_init::{Hook, InitCommand};
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
//!     Init(InitCommand),
//! }
//!
//! fn main() -> ExitCode {
//!     App::<Cli>::new("toy")
//!         .view(|cli| match cli.command {
//!             Command::Init(_) => shell::view(cli.output.format),
//!         })
//!         .run(|cli| match cli.command {
//!             Command::Init(command) => {
//!                 let hook = Hook::new("toy", "pick", "TOY_BLOCK")?;
//!                 Ok(command.script(&hook))
//!             }
//!         })
//! }
//! ```
//!
//! A user loads the script from a startup file. Each script names its
//! line in its first comment:
//!
//! | Shell | Startup file | Line |
//! |:--|:--|:--|
//! | bash | `~/.bashrc` | `eval "$(toy init bash)"` |
//! | zsh | `~/.zshrc` | `eval "$(toy init zsh)"` |
//! | fish | `~/.config/fish/config.fish` | `toy init fish \| source` |
//!
//! The script calls the binary by name and carries no path or setting read
//! at `init` time, so a saved copy keeps working across upgrades.
//!
//! [`shell::view`]: crate::shell::view

use std::error::Error;
use std::fmt;

use crate::formatdoc;
use crate::shell::{Script, Shell};

/// A command of the CLI that the shell runs once, in the foreground, before
/// its first prompt.
///
/// It runs as `<bin> <subcommand>`, and only when the environment variable
/// `variable` is set to a non-empty value and `bin` is on `PATH`. Each shell
/// removes the hook before running the command, so a later prompt never runs
/// it again.
///
/// In bash, another tool, such as bash-preexec, can rewrite `PROMPT_COMMAND`
/// so that the removal misses. So bash's hook also sets a variable it checks
/// first.
///
/// Every part is one word the shell reads without quoting, which
/// [`Hook::new`] checks once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hook {
    bin: String,
    subcommand: String,
    variable: String,
}

impl Hook {
    /// Declare the hook that runs `bin subcommand` when `variable` is set.
    ///
    /// # Errors
    ///
    /// Returns a [`HookError`] naming the first part that is not a plain word.
    ///
    /// - `bin` and `subcommand` take letters, digits, `-`, and `_`, and start
    ///   with a letter or digit.
    /// - `variable` takes letters, digits, and `_`, and does not start with a
    ///   digit.
    pub fn new(bin: &str, subcommand: &str, variable: &str) -> Result<Self, HookError> {
        if !is_command_word(bin) {
            return Err(HookError::Bin(bin.to_owned()));
        }
        if !is_command_word(subcommand) {
            return Err(HookError::Subcommand(subcommand.to_owned()));
        }
        if !is_variable_name(variable) {
            return Err(HookError::Variable(variable.to_owned()));
        }
        Ok(Self {
            bin: bin.to_owned(),
            subcommand: subcommand.to_owned(),
            variable: variable.to_owned(),
        })
    }

    /// The script that installs this hook in `shell`.
    ///
    /// bash runs it from `PROMPT_COMMAND`, as a string or, from bash 5.1, an
    /// array. zsh runs it from `precmd_functions`, and fish on the
    /// `fish_prompt` event. The bash hook returns the exit status it found, so
    /// a prompt command after it still reads the status of the user's last
    /// command.
    #[must_use]
    pub fn script(&self, shell: Shell) -> Script {
        let text = match shell {
            Shell::Bash => self.bash(),
            Shell::Zsh => self.zsh(),
            Shell::Fish => self.fish(),
        };
        Script { shell, text }
    }

    /// The shell function that runs the hook: the binary's name with `-`
    /// written `_`, so it is also a valid name for a bash variable.
    fn function(&self) -> String {
        format!("__{}_first_prompt", self.bin.replace('-', "_"))
    }

    fn bash(&self) -> String {
        let Self {
            bin,
            subcommand,
            variable,
        } = self;
        let function = self.function();
        formatdoc! {r#"
            # Load in ~/.bashrc: eval "$({bin} init bash)"
            {function}() {{
                local status=$? declaration
                if [[ -n ${{{function}_ran-}} ]]; then
                    return "${{status}}"
                fi
                {function}_ran=1
                declaration=$(declare -p PROMPT_COMMAND 2>/dev/null)
                if [[ ${{declaration}} == "declare -a"* ]]; then
                    local hooks=() hook
                    for hook in "${{PROMPT_COMMAND[@]}}"; do
                        [[ ${{hook}} == {function} ]] || hooks+=("${{hook}}")
                    done
                    PROMPT_COMMAND=("${{hooks[@]}}")
                else
                    printf -v PROMPT_COMMAND %s "${{PROMPT_COMMAND//{function};/}}"
                fi
                if [[ -n ${{{variable}-}} ]] && type -P {bin} >/dev/null; then
                    command {bin} {subcommand}
                fi
                return "${{status}}"
            }}
            {function}_declaration=$(declare -p PROMPT_COMMAND 2>/dev/null)
            if [[ ${{{function}_declaration}} == "declare -a"* ]]; then
                if [[ " ${{PROMPT_COMMAND[*]}} " != *" {function} "* ]]; then
                    PROMPT_COMMAND=({function} "${{PROMPT_COMMAND[@]}}")
                fi
            elif [[ ${{PROMPT_COMMAND-}} != *{function}\;* ]]; then
                printf -v PROMPT_COMMAND %s "{function};${{PROMPT_COMMAND-}}"
            fi
            unset {function}_declaration
            "#}
    }

    fn zsh(&self) -> String {
        let Self {
            bin,
            subcommand,
            variable,
        } = self;
        let function = self.function();
        formatdoc! {r#"
            # Load in ~/.zshrc: eval "$({bin} init zsh)"
            {function}() {{
                precmd_functions=(${{precmd_functions:#{function}}})
                if [[ -n ${{{variable}-}} ]] && whence -p {bin} >/dev/null; then
                    command {bin} {subcommand}
                fi
            }}
            if (( ! ${{precmd_functions[(I){function}]:-0}} )); then
                precmd_functions+=({function})
            fi
            "#}
    }

    fn fish(&self) -> String {
        let Self {
            bin,
            subcommand,
            variable,
        } = self;
        let function = self.function();
        formatdoc! {r#"
            # Load in ~/.config/fish/config.fish: {bin} init fish | source
            function {function} --on-event fish_prompt
                functions --erase {function}
                if test -n "${variable}"; and command -q {bin}
                    command {bin} {subcommand}
                end
            end
            "#}
    }
}

fn is_command_word(word: &str) -> bool {
    word.chars()
        .next()
        .is_some_and(|first| first.is_ascii_alphanumeric())
        && word
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn is_variable_name(name: &str) -> bool {
    name.chars()
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// A [`Hook`] part that is not a word the shell reads without quoting.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum HookError {
    /// The binary name is not a plain word. Holds the rejected value.
    Bin(String),
    /// The subcommand is not a plain word. Holds the rejected value.
    Subcommand(String),
    /// The environment variable name is not a plain word. Holds the rejected
    /// value.
    Variable(String),
}

impl fmt::Display for HookError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bin(bin) => write!(
                f,
                "binary name `{bin}` must be letters, digits, `-`, and `_`, starting with a letter or digit"
            ),
            Self::Subcommand(subcommand) => write!(
                f,
                "subcommand `{subcommand}` must be letters, digits, `-`, and `_`, starting with a letter or digit"
            ),
            Self::Variable(variable) => write!(
                f,
                "variable `{variable}` must be letters, digits, and `_`, not starting with a digit"
            ),
        }
    }
}

impl Error for HookError {}

/// The `init` verb, flattened into a CLI's subcommands with
/// `#[command(flatten)]`, so its name, help, and argument read the same in
/// every CLI built on ctl-core.
#[derive(Debug, Clone, PartialEq, Eq, clap::Subcommand)]
pub enum InitCommand {
    /// Print the script that hooks this command into your shell.
    Init(InitArgs),
}

impl InitCommand {
    /// The script for the shell the user asked for.
    #[must_use]
    pub fn script(&self, hook: &Hook) -> Script {
        match self {
            Self::Init(args) => hook.script(args.shell),
        }
    }
}

/// The arguments of the `init` verb.
#[derive(Debug, Clone, PartialEq, Eq, clap::Args)]
pub struct InitArgs {
    /// Shell to hook into: bash, zsh, or fish.
    #[arg(value_enum)]
    pub shell: Shell,
}
