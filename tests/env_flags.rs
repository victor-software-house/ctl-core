//! `env_prefix` reads flags from the environment, and `env_under_flags` keeps
//! the command line above them.
//!
//! Clap reads a variable when the command is built, and edition 2024 makes
//! `set_var` unsafe, so each case reruns this test binary as a child with the
//! variables set, and the child asserts what it parsed.
#![allow(
    missing_docs,
    reason = "an integration test crate has no public API to document"
)]
#![cfg(feature = "cli")]

use std::ffi::OsString;
use std::process::Command as Process;

use clap::error::ErrorKind;
use clap::{CommandFactory, FromArgMatches, Parser};
use ctl_core::flags::{DryRunArgs, OutputArgs, env_under_flags};
use ctl_core::parser::apply_defaults;
use ctl_core::prelude::*;

#[derive(Parser, Debug)]
#[command(version, about = "toy", env_prefix = "ENVTOY")]
struct Toy {
    #[command(flatten)]
    output: OutputArgs,
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    Prepare {
        #[arg(long, default_value = ".ctl/ver.yaml")]
        config: String,
        #[command(flatten)]
        dry: DryRunArgs,
        #[arg(long, overrides_with = "no_pr")]
        pr: bool,
        #[arg(long, overrides_with = "pr")]
        no_pr: bool,
        #[arg(long, conflicts_with = "before")]
        after: Option<String>,
        #[arg(long)]
        before: Option<String>,
        #[arg(long, conflicts_with = "check")]
        frozen: bool,
        #[arg(long)]
        check: bool,
        #[command(flatten)]
        bump: Bump,
    },
    Show {
        #[arg(long)]
        config: Option<String>,
        #[arg(long, exclusive = true)]
        list: bool,
    },
}

#[derive(Args, Debug)]
#[group(multiple = false)]
struct Bump {
    #[arg(long)]
    major: bool,
    #[arg(long)]
    minor: bool,
}

const CHILD: &str = "CTL_CORE_ENV_FLAGS_CHILD";

/// Parse as `App` does: the parser defaults, then the variables under the
/// command line.
fn try_parse(args: &[&str]) -> Result<Toy, clap::Error> {
    let raw = std::iter::once("toy")
        .chain(args.iter().copied())
        .map(OsString::from)
        .collect::<Vec<_>>();
    let matches =
        env_under_flags(apply_defaults(Toy::command()), &raw)?.try_get_matches_from(&raw)?;
    Toy::from_arg_matches(&matches)
}

#[expect(
    clippy::expect_used,
    reason = "a test helper: a failure here is the calling test failing"
)]
fn parse(args: &[&str]) -> Toy {
    try_parse(args).expect("parse")
}

/// Run `test` in a child with `vars` set, or report `true` when this process
/// is that child and should assert.
#[expect(
    clippy::expect_used,
    reason = "a test helper: a failure here is the calling test failing"
)]
fn in_child(test: &str, vars: &[(&str, &str)]) -> bool {
    if std::env::var_os(CHILD).as_deref() == Some(test.as_ref()) {
        return true;
    }
    let output = Process::new(std::env::current_exe().expect("test binary"))
        .args(["--exact", test, "--nocapture", "--test-threads=1"])
        .env(CHILD, test)
        .env_remove("ENVTOY_FORMAT")
        .env_remove("ENVTOY_CONFIG")
        .env_remove("ENVTOY_DRY_RUN")
        .env_remove("ENVTOY_PR")
        .env_remove("ENVTOY_AFTER")
        .env_remove("ENVTOY_BEFORE")
        .env_remove("ENVTOY_FROZEN")
        .env_remove("ENVTOY_CHECK")
        .env_remove("ENVTOY_MAJOR")
        .env_remove("ENVTOY_MINOR")
        .env_remove("ENVTOY_LIST")
        .envs(vars.iter().copied())
        .output()
        .expect("spawn child");
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    false
}

fn config(toy: &Toy) -> (&str, bool) {
    match &toy.command {
        Cmd::Prepare { config, dry, .. } => (config.as_str(), dry.dry_run),
        Cmd::Show { .. } => panic!("not prepare"),
    }
}

#[test]
fn a_variable_sets_a_subcommand_option_and_a_shared_flag() {
    if !in_child(
        "a_variable_sets_a_subcommand_option_and_a_shared_flag",
        &[("ENVTOY_CONFIG", "ci/ver.yaml"), ("ENVTOY_DRY_RUN", "true")],
    ) {
        return;
    }
    assert_eq!(config(&parse(&["prepare"])), ("ci/ver.yaml", true));
}

#[test]
fn the_flag_wins_over_its_variable() {
    if !in_child(
        "the_flag_wins_over_its_variable",
        &[("ENVTOY_CONFIG", "ci/ver.yaml"), ("ENVTOY_FORMAT", "json")],
    ) {
        return;
    }
    let toy = parse(&["--format", "pretty", "prepare", "--config", "own.yaml"]);
    assert!(!toy.output.format.is_json());
    assert_eq!(config(&toy).0, "own.yaml");
}

#[test]
fn a_global_flag_reads_its_variable() {
    if !in_child(
        "a_global_flag_reads_its_variable",
        &[("ENVTOY_FORMAT", "json")],
    ) {
        return;
    }
    assert!(parse(&["prepare"]).output.format.is_json());
}

#[test]
fn a_global_flag_after_the_subcommand_beats_its_variable() {
    if !in_child(
        "a_global_flag_after_the_subcommand_beats_its_variable",
        &[("ENVTOY_FORMAT", "json")],
    ) {
        return;
    }
    assert!(
        !parse(&["prepare", "--format", "pretty"])
            .output
            .format
            .is_json()
    );
}

#[test]
fn an_empty_variable_is_unset_and_false_is_false() {
    if !in_child(
        "an_empty_variable_is_unset_and_false_is_false",
        &[("ENVTOY_CONFIG", ""), ("ENVTOY_DRY_RUN", "false")],
    ) {
        return;
    }
    assert_eq!(config(&parse(&["prepare"])), (".ctl/ver.yaml", false));
}

fn pr(toy: &Toy) -> (bool, bool) {
    match &toy.command {
        Cmd::Prepare { pr, no_pr, .. } => (*pr, *no_pr),
        Cmd::Show { .. } => panic!("not prepare"),
    }
}

#[test]
fn a_negation_on_the_command_line_beats_the_variable() {
    if !in_child(
        "a_negation_on_the_command_line_beats_the_variable",
        &[("ENVTOY_PR", "true")],
    ) {
        return;
    }
    assert_eq!(pr(&parse(&["prepare"])), (true, false));
    assert_eq!(pr(&parse(&["prepare", "--no-pr"])), (false, true));
    assert_eq!(pr(&parse(&["prepare", "--no-pr", "--pr"])), (true, false));
}

#[test]
fn the_positive_side_on_the_command_line_beats_the_variable() {
    if !in_child(
        "the_positive_side_on_the_command_line_beats_the_variable",
        &[("ENVTOY_PR", "false")],
    ) {
        return;
    }
    assert_eq!(pr(&parse(&["prepare"])), (false, false));
    assert_eq!(pr(&parse(&["prepare", "--pr"])), (true, false));
    assert_eq!(pr(&parse(&["prepare", "--pr", "--no-pr"])), (false, true));
}

fn range(toy: &Toy) -> (Option<&str>, Option<&str>) {
    match &toy.command {
        Cmd::Prepare { after, before, .. } => (after.as_deref(), before.as_deref()),
        Cmd::Show { .. } => panic!("not prepare"),
    }
}

#[test]
fn a_conflicting_flag_beats_the_variable() {
    if !in_child(
        "a_conflicting_flag_beats_the_variable",
        &[("ENVTOY_AFTER", "a1")],
    ) {
        return;
    }
    assert_eq!(range(&parse(&["prepare"])), (Some("a1"), None));
    assert_eq!(
        range(&parse(&["prepare", "--before", "b1"])),
        (None, Some("b1"))
    );
}

#[test]
fn two_conflicting_variables_are_refused_by_name() {
    if !in_child(
        "two_conflicting_variables_are_refused_by_name",
        &[("ENVTOY_AFTER", "a1"), ("ENVTOY_BEFORE", "b1")],
    ) {
        return;
    }
    let Err(error) = try_parse(&["prepare"]) else {
        panic!("two conflicting variables parsed");
    };
    assert_eq!(error.kind(), ErrorKind::ArgumentConflict);
    let message = error.to_string();
    assert!(message.contains("ENVTOY_AFTER"), "{message}");
    assert!(message.contains("ENVTOY_BEFORE"), "{message}");
    assert_eq!(
        range(&parse(&["prepare", "--after", "a2"])),
        (Some("a2"), None),
        "the command line settles the conflict"
    );
}

fn lock(toy: &Toy) -> (bool, bool) {
    match &toy.command {
        Cmd::Prepare { frozen, check, .. } => (*frozen, *check),
        Cmd::Show { .. } => panic!("not prepare"),
    }
}

#[test]
fn a_false_flag_variable_cannot_conflict() {
    if !in_child(
        "a_false_flag_variable_cannot_conflict",
        &[("ENVTOY_CHECK", "false")],
    ) {
        return;
    }
    assert_eq!(lock(&parse(&["prepare", "--frozen"])), (true, false));
}

#[cfg(feature = "app")]
#[test]
fn app_puts_the_command_line_above_the_variables() {
    use std::cell::Cell;
    use std::rc::Rc;

    #[derive(serde::Serialize)]
    struct Done;

    impl Present for Done {
        fn present(&self) -> Document {
            Document::new()
        }
    }

    if !in_child(
        "app_puts_the_command_line_above_the_variables",
        &[("ENVTOY_PR", "true"), ("ENVTOY_AFTER", "a1")],
    ) {
        return;
    }
    let seen = Rc::new(Cell::new(None));
    let observed = Rc::clone(&seen);
    let code = App::<Toy>::new("toy").run_from(
        ["toy", "prepare", "--no-pr", "--before", "b1"],
        move |toy| {
            observed.set(Some((pr(&toy), range(&toy) == (None, Some("b1")))));
            Ok(Done)
        },
    );
    assert_eq!(code, ExitCode::SUCCESS);
    assert_eq!(seen.get(), Some(((false, true), true)));
}

#[cfg(feature = "help")]
#[test]
fn help_names_each_variable() {
    if !in_child("help_names_each_variable", &[]) {
        return;
    }
    let prepare = Toy::command()
        .find_subcommand("prepare")
        .cloned()
        .expect("prepare");
    let text =
        ctl_core::help::document(prepare).render(RenderOptions::new(ColorMode::Never).width(200));
    assert!(text.contains("[env: ENVTOY_CONFIG]"), "{text}");
    assert!(text.contains("[env: ENVTOY_DRY_RUN]"), "{text}");
}

fn bump(toy: &Toy) -> (bool, bool) {
    match &toy.command {
        Cmd::Prepare { bump, .. } => (bump.major, bump.minor),
        Cmd::Show { .. } => panic!("not prepare"),
    }
}

#[test]
fn a_flag_beats_the_variable_of_a_single_choice_group_member() {
    if !in_child(
        "a_flag_beats_the_variable_of_a_single_choice_group_member",
        &[("ENVTOY_MAJOR", "true")],
    ) {
        return;
    }
    assert_eq!(bump(&parse(&["prepare"])), (true, false));
    assert_eq!(bump(&parse(&["prepare", "--minor"])), (false, true));
}

#[test]
fn an_exclusive_flag_beats_every_variable() {
    if !in_child(
        "an_exclusive_flag_beats_every_variable",
        &[("ENVTOY_CONFIG", "ci/ver.yaml")],
    ) {
        return;
    }
    let Cmd::Show { config, list } = parse(&["show", "--list"]).command else {
        panic!("not show");
    };
    assert_eq!((config, list), (None, true));
}
