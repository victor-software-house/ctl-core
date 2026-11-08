//! The `completion` verb: each shell's script for a toy CLI, its help and
//! view, and the `COMPLETE` request `App::completion` answers.
//!
//! Edition 2024 makes `set_var` unsafe, so each environment case reruns this
//! test binary as a child with `COMPLETE` set, and the parent reads the
//! child's stdout and stderr.
#![allow(
    missing_docs,
    reason = "an integration test crate has no public API to document"
)]
#![cfg(all(feature = "completion", feature = "app"))]

use std::process::{Command as Process, Output};

use clap::error::ErrorKind;
use clap::{CommandFactory, Parser};
use ctl_core::completion::{self, CompletionArgs, CompletionCommand, Script};
use ctl_core::prelude::*;
use ctl_core::shell::Shell;
use ctl_core::{help, indoc};
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
            Cmd::Completion(_) => completion::view(cli.output.format),
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
            shell: Shell::Fish
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
    assert!(
        rendered.contains("Print the script that loads shell completion for this command"),
        "{rendered}"
    );
    assert!(rendered.contains("[bash|zsh|fish]"), "{rendered}");
    assert!(rendered.contains("bash, zsh, or fish"), "{rendered}");
}

#[test]
fn fish_registers_one_line_that_calls_the_binary_by_name() {
    assert_eq!(
        script(Shell::Fish, "toy"),
        indoc! {r#"
            complete --keep-order --exclusive --command toy --arguments "(COMPLETE=fish toy -- (commandline --current-process --tokenize --cut-at-cursor) (commandline --current-token))"
        "#}
    );
}

#[test]
fn bash_registers_a_function_that_calls_the_binary_by_name() {
    let text = script(Shell::Bash, "toy");
    for line in [
        "_clap_complete_toy() {",
        r#"COMPLETE="bash" \"#,
        r#""toy" -- "${words[@]}" \"#,
        "complete -o nospace -o bashdefault -o nosort -F _clap_complete_toy toy",
    ] {
        assert!(text.contains(line), "missing {line:?} in\n{text}");
    }
}

#[test]
fn zsh_completes_on_the_first_tab_from_fpath() {
    let text = script(Shell::Zsh, "state-sync");
    assert!(text.starts_with("#compdef state-sync\n"), "{text}");
    assert!(text.contains(r#"COMPLETE="zsh" \"#), "{text}");
    assert!(
        text.contains(r#"state-sync -- "${words[@]}" 2>/dev/null \"#),
        "{text}"
    );
    assert!(
        text.ends_with(indoc! {r#"
            compdef _clap_dynamic_completer_state_sync state-sync
            if [[ ${funcstack[1]} == _state-sync ]]; then
                _clap_dynamic_completer_state_sync "$@"
            fi
        "#}),
        "{text}"
    );
}

#[test]
fn the_view_prints_the_script_as_it_is() {
    let script = Shell::Zsh.script("toy").unwrap();
    let pretty = completion::view(OutputFormat::Pretty)
        .capture(&script)
        .unwrap();
    assert_eq!(pretty.text(), script.text);
    assert!(!pretty.text().contains('\u{1b}'));

    let json = completion::view(OutputFormat::Json)
        .capture(&script)
        .unwrap();
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
