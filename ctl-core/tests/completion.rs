//! The `completion` verb: each shell's script for a toy CLI, its help and
//! view, and the `COMPLETE` request `App::completion` answers.
//!
//! Each script and the help are reviewed snapshots in `tests/snapshots`, and
//! each script also passes its own shell's syntax check.
//!
//! Edition 2024 makes `set_var` unsafe, so each environment case reruns this
//! test binary as a child with `COMPLETE` set, and the parent reads the
//! child's stdout and stderr.
#![cfg(all(feature = "completion", feature = "app", feature = "fixture"))]

#[path = "support/shells.rs"]
mod shells;

use std::process::{Command as Process, Output};

use clap::error::ErrorKind;
use clap::{CommandFactory, Parser};
use ctl_core::completion::{self, CompletionArgs, CompletionCommand};
use ctl_core::help;
use ctl_core::prelude::*;
use ctl_core::shell::{self, Script, Shell};
use serde::Serialize;

#[derive(Parser, Debug)]
#[command(name = "toy", version, about = "toy")]
struct Toy {
    #[command(flatten)]
    output: OutputArgs,
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Show status.
    Status,
    #[command(flatten)]
    Completion(CompletionCommand),
}

#[derive(Serialize)]
struct Pending {
    pending: usize,
}

impl Present for Pending {
    fn present(&self) -> Document {
        Document::new().fields(Fields::new().row("pending", self.pending.to_string()))
    }
}

#[derive(Serialize)]
#[serde(untagged)]
enum Report {
    Pending(Pending),
    Script(Script),
}

impl Present for Report {
    fn present(&self) -> Document {
        match self {
            Self::Pending(pending) => pending.present(),
            Self::Script(script) => script.present(),
        }
    }
}

/// The toy `main`: the hook on, the verb routed to its view.
fn run(args: &[&str]) -> ExitCode {
    App::<Toy>::new("toy")
        .completion()
        .view(|cli| match cli.command {
            Cmd::Completion(_) => shell::view(cli.output.format),
            Cmd::Status => cli.output.view(),
        })
        .run_from(std::iter::once("toy").chain(args.iter().copied()), |cli| {
            Ok(match cli.command {
                Cmd::Status => Report::Pending(Pending { pending: 0 }),
                Cmd::Completion(command) => Report::Script(command.script("toy")?),
            })
        })
}

const CHILD: &str = "CTL_CORE_COMPLETION_CHILD";

/// Run `test` in a child with `vars` set and return its output, or `None`
/// when this process is that child and should run the case.
#[expect(
    clippy::expect_used,
    reason = "a test helper: a failure here is the calling test failing"
)]
fn in_child(test: &str, vars: &[(&str, &str)]) -> Option<Output> {
    if std::env::var_os(CHILD).as_deref() == Some(test.as_ref()) {
        return None;
    }
    let output = Process::new(std::env::current_exe().expect("test binary"))
        .args(["--exact", test, "--nocapture", "--test-threads=1"])
        .env(CHILD, test)
        .env_remove(completion::VAR)
        .env_remove("CLICOLOR")
        .env_remove("CLICOLOR_FORCE")
        .env_remove("FORCE_COLOR")
        .env_remove("NO_COLOR")
        .env_remove("COLORTERM")
        .envs(vars.iter().copied())
        .output()
        .expect("spawn child");
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Some(output)
}

#[expect(
    clippy::unwrap_used,
    reason = "a test helper: a failure here is the calling test failing"
)]
fn script(shell: Shell, bin: &str) -> String {
    shell.script(bin).unwrap().text
}

#[test]
fn a_shell_parses_from_its_name() {
    let toy = Toy::try_parse_from(["toy", "completion", "fish"]).unwrap();
    assert!(matches!(
        toy.command,
        Cmd::Completion(CompletionCommand::Completion(CompletionArgs {
            shell: Shell::Fish,
            ..
        }))
    ));
    let error = Toy::try_parse_from(["toy", "completion", "elvish"]).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::InvalidValue);
}

#[test]
fn the_verb_help_names_the_shells() {
    let command = Toy::command()
        .find_subcommand("completion")
        .cloned()
        .unwrap();
    let rendered = help::document(command).render(RenderOptions::new(ColorMode::Never).width(100));
    insta::assert_snapshot!("help", rendered);
}

/// shellcheck codes the bash script `clap_complete` 4.6.11 writes breaks, in
/// lines ctl-core does not own: SC2207 splits the candidates on the `\013`
/// separator the script sets in `IFS`, SC2181 reads `$?` after that
/// assignment, and SC2250 is a brace style its template does not use.
const CLAP_BASH: &[&str] = &["SC2207", "SC2181", "SC2250"];

/// Snapshot the script `shell` registers for `bin`, then check that the
/// shell reads it.
fn assert_script(name: &str, shell: Shell, bin: &str, exclude: &[&str]) {
    let text = script(shell, bin);
    insta::assert_snapshot!(name, text);
    shells::check(shell, &text, exclude);
}

#[test]
fn bash_registers_a_function_that_calls_the_binary_by_name() {
    assert_script("bash", Shell::Bash, "toy", CLAP_BASH);
}

#[test]
fn zsh_completes_on_the_first_tab_from_fpath() {
    assert_script("zsh", Shell::Zsh, "demo-cli", &[]);
}

#[test]
fn fish_registers_one_line_that_calls_the_binary_by_name() {
    assert_script("fish", Shell::Fish, "toy", &[]);
}

#[test]
fn the_view_prints_the_script_as_it_is() {
    let script = Shell::Zsh.script("toy").unwrap();
    let pretty = shell::view(OutputFormat::Pretty).capture(&script).unwrap();
    assert_eq!(pretty.text(), script.text);
    assert!(!pretty.text().contains('\u{1b}'));

    let json = shell::view(OutputFormat::Json).capture(&script).unwrap();
    let value: serde_json::Value = serde_json::from_str(json.text()).unwrap();
    assert_eq!(value["shell"], "zsh");
    assert_eq!(value["text"], script.text);
}

#[test]
fn the_verb_prints_the_script_under_quiet() {
    let Some(output) = in_child("the_verb_prints_the_script_under_quiet", &[]) else {
        assert_eq!(run(&["--quiet", "completion", "zsh"]), ExitCode::SUCCESS);
        return;
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(&script(Shell::Zsh, "toy")), "{stdout}");
}

#[test]
fn the_environment_prints_the_registration() {
    let Some(output) = in_child(
        "the_environment_prints_the_registration",
        &[(completion::VAR, "zsh")],
    ) else {
        assert_eq!(run(&[]), ExitCode::SUCCESS);
        return;
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(&script(Shell::Zsh, "toy")), "{stdout}");
}

#[test]
fn the_environment_prints_candidates() {
    let Some(output) = in_child(
        "the_environment_prints_candidates",
        &[(completion::VAR, "fish")],
    ) else {
        assert_eq!(run(&["--", "toy", "st"]), ExitCode::SUCCESS);
        return;
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("status\tShow status"), "{stdout}");
}

#[test]
fn a_disabled_request_runs_the_command() {
    let Some(output) = in_child(
        "a_disabled_request_runs_the_command",
        &[(completion::VAR, "0")],
    ) else {
        assert_eq!(run(&["status"]), ExitCode::SUCCESS);
        return;
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("pending"), "{stdout}");
    assert!(!stdout.contains("compdef"), "{stdout}");
}

#[test]
fn an_unsupported_shell_is_an_error() {
    let Some(output) = in_child(
        "an_unsupported_shell_is_an_error",
        &[(completion::VAR, "elvish")],
    ) else {
        assert_ne!(run(&[]), ExitCode::SUCCESS);
        return;
    };
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("expected one of bash, zsh, fish"),
        "{stderr}"
    );
}
