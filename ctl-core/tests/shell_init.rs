//! The `init` verb: each shell's script for a toy CLI, its help, and each
//! script running in its own shell on a terminal.
//!
//! Each script and the help are reviewed snapshots in `tests/snapshots`, and
//! each script also passes its own shell's syntax check. The hook tests start
//! bash, zsh, and fish on a pseudo-terminal with a startup file that loads the
//! script twice, wait for three prompts, and read what the fake binary wrote.
#![cfg(all(feature = "shell-init", feature = "app", feature = "fixture"))]

#[path = "support/shells.rs"]
mod shells;
#[path = "support/terminal.rs"]
mod terminal;

use std::fs;
use std::os::unix::fs::PermissionsExt as _;

use clap::error::ErrorKind;
use clap::{CommandFactory, Parser};
use ctl_core::fixture::{Root, Suite};
use ctl_core::help;
use ctl_core::prelude::*;
use ctl_core::shell::Shell;
use ctl_core::shell_init::{Hook, HookError, InitArgs, InitCommand};
use pty_process::blocking::Command;
use terminal::{PROMPT, Terminal};

#[derive(Parser, Debug)]
#[command(name = "toy-cli", version, about = "toy")]
struct Toy {
    #[command(flatten)]
    output: OutputArgs,
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    #[command(flatten)]
    Init(InitCommand),
}

const BIN: &str = "toy-cli";
const VARIABLE: &str = "TOY_BLOCK";

#[expect(
    clippy::unwrap_used,
    reason = "a test helper: a failure here is the calling test failing"
)]
fn hook() -> Hook {
    Hook::new(BIN, "pick", VARIABLE).unwrap()
}

#[test]
fn a_shell_parses_from_its_name() {
    let toy = Toy::try_parse_from([BIN, "init", "fish"]).unwrap();
    let Cmd::Init(command) = toy.command;
    assert_eq!(command, InitCommand::Init(InitArgs { shell: Shell::Fish }));
    assert_eq!(command.script(&hook()), hook().script(Shell::Fish));
    let error = Toy::try_parse_from([BIN, "init", "elvish"]).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::InvalidValue);
}

#[test]
fn the_verb_help_names_the_shells() {
    let command = Toy::command().find_subcommand("init").cloned().unwrap();
    let rendered = help::document(command).render(RenderOptions::new(ColorMode::Never).width(100));
    insta::assert_snapshot!("help", rendered);
}

#[test]
fn a_part_the_shell_would_need_quoted_is_an_error() {
    assert_eq!(
        Hook::new("toy cli", "pick", VARIABLE),
        Err(HookError::Bin("toy cli".to_owned()))
    );
    assert_eq!(
        Hook::new(BIN, "-pick", VARIABLE),
        Err(HookError::Subcommand("-pick".to_owned()))
    );
    assert_eq!(
        Hook::new(BIN, "pick", "1BLOCK"),
        Err(HookError::Variable("1BLOCK".to_owned()))
    );
    assert_eq!(
        Hook::new(BIN, "pick;ls", VARIABLE).unwrap_err().to_string(),
        "subcommand `pick;ls` must be letters, digits, `-`, and `_`, starting with a letter or digit"
    );
}

/// Snapshot the script for `shell`, then check that the shell reads it.
fn assert_script(shell: Shell, name: &str) {
    let text = hook().script(shell).text;
    insta::assert_snapshot!(name, text);
    shells::check(shell, &text, &[]);
}

#[test]
fn bash_runs_the_hook_from_prompt_command() {
    assert_script(Shell::Bash, "bash");
}

#[test]
fn zsh_runs_the_hook_from_precmd_functions() {
    assert_script(Shell::Zsh, "zsh");
}

#[test]
fn fish_runs_the_hook_on_the_prompt_event() {
    assert_script(Shell::Fish, "fish");
}

/// What a hook test sets up around the shell.
#[derive(Clone, Copy)]
enum Setup {
    /// The variable set and the binary on `PATH`: the hook runs.
    Ready,
    /// The variable unset.
    NoVariable,
    /// The binary missing from `PATH`.
    NoBinary,
}

/// bash's prompt command as a string, or as the array bash 5.1 accepts.
#[derive(Clone, Copy)]
enum PromptCommand {
    String,
    Array,
}

/// Start `shell` on a terminal with a startup file that sets a prompt
/// showing the last exit status, then loads the script twice. Type two
/// lines and `exit`, and return what the fake binary logged and what the
/// shell wrote to the terminal.
///
/// The fake binary logs its arguments and exits 7, so a hook that leaked
/// the binary's status would show `[7]` in the first prompt.
fn run(shell: Shell, setup: Setup, prompt_command: PromptCommand) -> (String, String) {
    let suite = Suite::new("shell-init");
    let root = suite.scenario("hook");
    let bin = root.path().join("bin");
    match setup {
        Setup::Ready | Setup::NoVariable => fake_binary(&root),
        Setup::NoBinary => {
            root.file("bin/.keep", "");
        }
    }
    let script = root.file("init", hook().script(shell).text);
    let script = script.display();
    let command = match shell {
        Shell::Bash => {
            let prompt_command = match prompt_command {
                PromptCommand::String => "PROMPT_COMMAND='seen=$?'",
                PromptCommand::Array => indoc::indoc! {"
                    if (( BASH_VERSINFO[0] * 100 + BASH_VERSINFO[1] >= 501 )); then
                        PROMPT_COMMAND=('seen=$?')
                    else
                        PROMPT_COMMAND='seen=$?'
                    fi"},
            };
            let rc = root.file(
                "bashrc",
                indoc::formatdoc! {"
                    {prompt_command}
                    PS1='[${{seen-}}] {PROMPT}'
                    source '{script}'
                    source '{script}'
                    "},
            );
            Command::new("bash")
                .env_clear()
                .arg("--noprofile")
                .arg("--rcfile")
                .arg(rc)
                .arg("-i")
        }
        Shell::Zsh => {
            root.file(
                ".zshrc",
                indoc::formatdoc! {"
                    PROMPT='[%?] {PROMPT}'
                    source '{script}'
                    source '{script}'
                    "},
            );
            Command::new("zsh")
                .env_clear()
                .arg("-d")
                .arg("-i")
                .env("ZDOTDIR", root.path())
        }
        Shell::Fish => Command::new("fish")
            .env_clear()
            .arg("--no-config")
            .arg("-i")
            .arg("--init-command")
            .arg(indoc::formatdoc! {"
                function fish_prompt; printf '[%s] {PROMPT}' $status; end
                source '{script}'
                source '{script}'"})
            .env("fish_features", "no-query-term"),
    };
    let path = std::env::var("PATH").unwrap_or_default();
    let log = root.path().join("log");
    let mut command = command
        .env("PATH", format!("{}:{path}", bin.display()))
        .env("HOME", root.path())
        .env("TERM", "xterm")
        .env("TOY_LOG", &log);
    match setup {
        Setup::Ready | Setup::NoBinary => command = command.env(VARIABLE, "block-1"),
        Setup::NoVariable => {}
    }
    let mut terminal = Terminal::start(command);
    terminal.type_line("true one");
    terminal.type_line("true two");
    let output = terminal.exit();
    (fs::read_to_string(log).unwrap_or_default(), output)
}

fn fake_binary(root: &Root<'_>) {
    let path = root.file(
        format!("bin/{BIN}"),
        indoc::indoc! {r#"
            #!/bin/sh
            echo "$*" >> "$TOY_LOG"
            exit 7
            "#},
    );
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755))
        .unwrap_or_else(|error| panic!("make {} executable: {error}", path.display()));
}

/// The hook ran once, and the first prompt still showed status 0.
fn assert_ran_once(shell: Shell, prompt_command: PromptCommand) {
    let (log, output) = run(shell, Setup::Ready, prompt_command);
    assert_eq!(log, "pick\n", "{output}");
    assert!(output.contains(&format!("[0] {PROMPT}")), "{output}");
    assert!(!output.contains(&format!("[7] {PROMPT}")), "{output}");
}

/// The hook ran nothing and printed no error.
fn assert_skipped(shell: Shell, setup: Setup) {
    let (log, output) = run(shell, setup, PromptCommand::String);
    assert_eq!(log, "", "{output}");
    assert!(!output.contains("not found"), "{output}");
    assert!(!output.contains("Unknown command"), "{output}");
}

#[test]
fn bash_runs_the_hook_once_from_a_string() {
    assert_ran_once(Shell::Bash, PromptCommand::String);
}

#[test]
fn bash_runs_the_hook_once_from_an_array() {
    assert_ran_once(Shell::Bash, PromptCommand::Array);
}

#[test]
fn zsh_runs_the_hook_once() {
    assert_ran_once(Shell::Zsh, PromptCommand::String);
}

#[test]
fn fish_runs_the_hook_once() {
    assert_ran_once(Shell::Fish, PromptCommand::String);
}

#[test]
fn no_shell_runs_the_hook_without_its_variable() {
    for shell in [Shell::Bash, Shell::Zsh, Shell::Fish] {
        assert_skipped(shell, Setup::NoVariable);
    }
}

#[test]
fn no_shell_runs_the_hook_without_the_binary() {
    for shell in [Shell::Bash, Shell::Zsh, Shell::Fish] {
        assert_skipped(shell, Setup::NoBinary);
    }
}
