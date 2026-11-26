//! The input document: `--input` fills the arguments of one command from a
//! YAML, JSON or TOML file, and the command line and the environment keep
//! their precedence over it.
//!
//! Each case runs this test binary again as a child, as the other
//! environment tests do: the workspace denies `set_var`, a case may feed
//! stdin, and the child's stderr is what a person reads. The parent writes
//! the fixture files and the child reads them from the root the parent names.
#![allow(
    missing_docs,
    reason = "an integration test crate has no public API to document"
)]
#![cfg(all(
    feature = "input-document",
    feature = "fixture",
    feature = "input-yaml",
    feature = "input-json",
    feature = "input-toml"
))]

use std::any::type_name;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::{env, iter, panic};

use clap::ArgAction;
use ctl_core::fixture::Suite;
use ctl_core::prelude::*;
use serde::Serialize;

#[path = "support/child.rs"]
#[expect(
    dead_code,
    reason = "this file runs its children itself, with a piped stdin, and never calls in_child"
)]
mod child;

const CHILD: &str = "CTL_CORE_INPUT_DOCUMENT_CHILD";
const ROOT: &str = "CTL_CORE_INPUT_DOCUMENT_ROOT";
const FAIL: &str = "CTL_CORE_INPUT_DOCUMENT_FAIL";
const WITHOUT: &str = "CTL_CORE_INPUT_DOCUMENT_WITHOUT";
const LABEL: &str = "CTL_CORE_INPUT_DOCUMENT_LABEL";

#[derive(Parser, Debug)]
#[command(
    name = "toy",
    version = "1.0",
    about = "toy",
    env_prefix = "DOCTOY",
    infer_subcommands = true
)]
struct Toy {
    #[command(flatten)]
    output: OutputArgs,
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    Add(Add),
    Show {
        id: String,
    },
    Copy {
        from: String,
        to: String,
    },
    Rows {
        rows: Vec<String>,
    },
    #[command(infer_subcommands = true)]
    Group {
        #[command(subcommand)]
        command: Nested,
    },
    #[command(short_flag = 'S', long_flag = "sync")]
    Sync {
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand, Debug)]
enum Nested {
    Label {
        #[arg(long)]
        title: Option<String>,
    },
}

#[derive(Args, Debug)]
struct Add {
    #[arg(long)]
    title: Option<String>,
    #[arg(long)]
    priority: Option<u8>,
    #[arg(long, overrides_with = "no_pr")]
    pr: bool,
    #[arg(long, env = "DOCTOY_NO_PR", overrides_with = "pr")]
    no_pr: bool,
    #[arg(long, action = ArgAction::Append)]
    label: Vec<String>,
    #[arg(long, value_delimiter = ',')]
    tag: Vec<String>,
    #[arg(short, long, action = ArgAction::Count)]
    verbose: u8,
    #[arg(long, conflicts_with = "before")]
    after: Option<String>,
    #[arg(long)]
    before: Option<String>,
    #[arg(long, num_args = 2)]
    pair: Vec<String>,
    #[arg(long)]
    force: bool,
}

#[derive(Parser, Debug)]
#[command(name = "own", version = "1.0", about = "own", env_prefix = "OWN")]
struct Own {
    #[command(flatten)]
    output: OutputArgs,
    #[arg(long)]
    input: Option<String>,
}

#[derive(Serialize)]
struct Done;

impl Present for Done {
    fn present(&self) -> Document {
        Document::new()
    }
}

fn execute(args: impl IntoIterator<Item = String>) -> ExitCode {
    let mut app = App::<Toy>::new("toy").view(|cli| cli.output.view());
    if env::var_os(FAIL).is_some() {
        app = app.input_conflicts(InputConflicts::Fail);
    }
    if env::var_os(WITHOUT).is_some() {
        app = app.without_input();
    }
    app.run_from(iter::once("toy".to_owned()).chain(args), |cli| {
        let _ = writeln!(io::stdout().lock(), "{cli:?}");
        Ok(Done)
    })
}

/// One case. A test with several cases gives each a label, and loops with
/// `continue` where a single case returns, because the child runs the case it
/// is told and skips the others.
///
/// A case is the files the parent writes, the variables and stdin the child
/// gets, its arguments, and the exit code the child must return. A `@name` in
/// an argument or a variable value is the path of the file called `name`.
#[derive(Default)]
struct Case<'a> {
    /// Tells the cases of one test apart: the child runs the one it is told.
    label: &'a str,
    files: &'a [(&'a str, &'a str)],
    vars: &'a [(&'a str, &'a str)],
    stdin: Option<&'a str>,
    args: &'a [&'a str],
    code: u8,
}

impl<'a> Case<'a> {
    fn label(mut self, label: &'a str) -> Self {
        self.label = label;
        self
    }

    fn vars(mut self, vars: &'a [(&'a str, &'a str)]) -> Self {
        self.vars = vars;
        self
    }

    fn stdin(mut self, text: &'a str) -> Self {
        self.stdin = Some(text);
        self
    }

    fn code(mut self, code: u8) -> Self {
        self.code = code;
        self
    }
}

/// A case that runs `args` with the fixture `files` and expects exit code 0.
fn case<'a>(files: &'a [(&'a str, &'a str)], args: &'a [&'a str]) -> Case<'a> {
    Case {
        files,
        args,
        ..Case::default()
    }
}

/// What a child wrote.
struct Outcome {
    stdout: String,
    stderr: String,
}

fn expand(text: &str, root: &Path) -> String {
    text.strip_prefix('@').map_or_else(
        || text.to_owned(),
        |name| root.join(name).display().to_string(),
    )
}

/// The name of the test that holds the closure `here`.
fn test_name<F>(_here: &F) -> &'static str {
    type_name::<F>()
        .trim_end_matches("::{{closure}}")
        .rsplit("::")
        .next()
        .unwrap_or_default()
}

/// Run `case` in a child and return what it wrote, or `None` in the child,
/// which has run the case and asserted its exit code.
#[expect(
    clippy::expect_used,
    reason = "a test helper: a failure here is the calling test failing"
)]
fn run<F: Fn()>(here: &F, case: &Case<'_>) -> Option<Outcome> {
    let test = test_name(here);
    if env::var_os(CHILD).as_deref() == Some(test.as_ref()) {
        if env::var_os(LABEL).as_deref() != Some(case.label.as_ref()) {
            return None;
        }
        let root = PathBuf::from(env::var_os(ROOT).expect("the parent names the root"));
        let args = case.args.iter().map(|arg| expand(arg, &root));
        assert_eq!(
            child::case(|| execute(args)),
            ExitCode::from(case.code),
            "the exit code"
        );
        return None;
    }
    let suite = Suite::new("input-document");
    let root = suite.scenario(test);
    for (name, body) in case.files {
        root.file(name, body);
    }
    let vars: Vec<(String, String)> = case
        .vars
        .iter()
        .map(|(name, value)| ((*name).to_owned(), expand(value, root.path())))
        .collect();
    let mut command = child::command(CHILD, test, &[]).expect("the parent runs the child");
    command
        .env(ROOT, root.path())
        .env(LABEL, case.label)
        .envs(vars.iter().map(|(name, value)| (name, value)))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut running = command.spawn().expect("spawn the child");
    let mut stdin = running.stdin.take().expect("the child's stdin");
    if let Some(text) = case.stdin {
        stdin.write_all(text.as_bytes()).expect("write stdin");
    }
    drop(stdin);
    let output = running.wait_with_output().expect("wait for the child");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(
        output.status.success(),
        "{}{stderr}",
        String::from_utf8_lossy(&output.stdout)
    );
    Some(Outcome {
        stdout: child::case_stdout(&output),
        stderr,
    })
}

/// Run a case, bind what the child wrote to `$out`, and in the child, which has
/// run it, leave the test, or the loop with `continue`.
macro_rules! ran {
    ($out:ident = $case:expr) => {
        let Some($out) = run(&|| {}, &$case) else {
            return;
        };
    };
    ($out:ident = $case:expr,continue) => {
        let Some($out) = run(&|| {}, &$case) else {
            continue;
        };
    };
}

/// The text with each run of whitespace as one space, so a line the error
/// view wrapped reads as it was written.
fn flat(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `needle` is in `haystack`, or the failure shows `haystack`.
#[track_caller]
fn has(haystack: &str, needle: &str) {
    let flat = flat(haystack);
    assert!(flat.contains(needle), "missing {needle:?} in:\n{haystack}");
}

#[track_caller]
fn lacks(haystack: &str, needle: &str) {
    let flat = flat(haystack);
    assert!(!flat.contains(needle), "found {needle:?} in:\n{haystack}");
}

/// How many problems the error view wrote for the file `name`.
fn problems(stderr: &str, name: &str) -> usize {
    flat(stderr).matches(&format!("{name}:")).count()
}

const TASK: &str = "title: Fix the cache\npriority: 2\n";

#[test]
fn the_flags_are_there() {
    ran!(out = case(&[], &["add", "--help"]));
    has(&out.stdout, "Input");
    has(&out.stdout, "--input ┆ FILE");
    has(&out.stdout, "--input-format ┆ [yaml|json|toml]");
    has(&out.stdout, "DOCTOY_INPUT]");
    has(&out.stdout, "DOCTOY_INPUT_FORMAT]");
}

#[test]
fn a_cli_removes_them() {
    ran!(
        out = case(&[("a.yaml", TASK)], &["add", "--input", "@a.yaml"])
            .vars(&[(WITHOUT, "1")])
            .code(2)
    );
    has(&out.stderr, "unexpected argument '--input'");
}

#[test]
fn a_flag_from_the_document() {
    ran!(out = case(&[("task.yaml", TASK)], &["add", "--input", "@task.yaml"]));
    has(&out.stdout, "title: Some(\"Fix the cache\")");
    has(&out.stdout, "priority: Some(2)");
}

#[test]
fn the_command_line_wins() {
    ran!(
        out = case(
            &[("task.yaml", TASK)],
            &["add", "--input", "@task.yaml", "--priority", "5"],
        )
    );
    has(&out.stdout, "priority: Some(5)");
}

#[test]
fn the_environment_wins_over_the_document() {
    ran!(
        out = case(&[("task.yaml", TASK)], &["add", "--input", "@task.yaml"])
            .vars(&[("DOCTOY_PRIORITY", "3")])
    );
    has(&out.stdout, "priority: Some(3)");
}

#[test]
fn the_document_wins_over_the_default() {
    ran!(
        out = case(
            &[("task.yaml", "title: only\n")],
            &["add", "--input", "@task.yaml"],
        )
    );
    has(&out.stdout, "priority: None");
}

#[test]
fn the_input_variable_names_the_document() {
    ran!(out = case(&[("task.yaml", TASK)], &["add"]).vars(&[("DOCTOY_INPUT", "@task.yaml")]));
    has(&out.stdout, "priority: Some(2)");
}

#[test]
fn the_input_flag_wins_over_its_variable() {
    ran!(
        out = case(
            &[("task.yaml", TASK), ("other.yaml", "priority: 9\n")],
            &["add", "--input", "@task.yaml"],
        )
        .vars(&[("DOCTOY_INPUT", "@other.yaml")])
    );
    has(&out.stdout, "priority: Some(2)");
}

#[test]
fn stdin_needs_a_format() {
    ran!(out = case(&[], &["add", "--input", "-"]).stdin(TASK).code(2));
    has(&out.stderr, "--input-format");
    assert_eq!(problems(&out.stderr, "<stdin>"), 1, "{}", out.stderr);
}

#[test]
fn stdin_is_read_with_a_format() {
    ran!(
        out = case(&[], &["add", "--input", "-", "--input-format", "json"])
            .stdin("{\"priority\": 4}")
    );
    has(&out.stdout, "priority: Some(4)");
}

#[test]
fn a_document_is_read_in_yaml_json_and_toml() {
    for (name, body) in [
        ("task.yaml", "title: x\npriority: 7\n"),
        ("task.json", "{\"title\": \"x\", \"priority\": 7}"),
        ("task.toml", "title = \"x\"\npriority = 7\n"),
    ] {
        ran!(
            out = case(&[(name, body)], &["add", "--input", &format!("@{name}")]).label(name),
            continue
        );
        has(&out.stdout, "priority: Some(7)");
    }
}

#[test]
fn a_boolean_pair() {
    ran!(
        out = case(
            &[("task.yaml", "pr: false\n")],
            &["add", "--input", "@task.yaml"],
        )
    );
    has(&out.stdout, "pr: false, no_pr: true");
}

#[test]
fn a_list() {
    ran!(
        out = case(
            &[("task.yaml", "label: [cache, perf]\n")],
            &["add", "--input", "@task.yaml"],
        )
    );
    has(&out.stdout, "label: [\"cache\", \"perf\"]");
}

#[test]
fn dropped_by_default() {
    ran!(
        out = case(
            &[("task.yaml", "title: x\npriority: 2\nlabel: a\npr: true\n")],
            &["add", "--input", "@task.yaml", "--no-pr"],
        )
    );
    has(&out.stdout, "pr: false, no_pr: true");
    lacks(&out.stderr, "task.yaml");
}

#[test]
fn refused_when_the_cli_asks() {
    ran!(
        out = case(
            &[("task.yaml", "title: x\npriority: 2\nlabel: a\npr: true\n")],
            &["add", "--input", "@task.yaml", "--no-pr"],
        )
        .vars(&[(FAIL, "1")])
        .code(2)
    );
    has(&out.stderr, "task.yaml:4:1: pr:");
    has(&out.stderr, "--no-pr");
    assert_eq!(problems(&out.stderr, "task.yaml"), 1, "{}", out.stderr);
}

#[test]
fn a_key_loses_to_a_kept_partner_variable() {
    let case = Case {
        files: &[("task.yaml", "pr: true\n")],
        vars: &[("DOCTOY_NO_PR", "true")],
        args: &["add", "--input", "@task.yaml"],
        ..Case::default()
    };
    ran!(out = case);
    has(&out.stdout, "pr: false, no_pr: true");
}

#[test]
fn a_key_that_loses_to_a_kept_variable_is_refused_when_the_cli_asks() {
    ran!(
        out = case(
            &[("task.yaml", "pr: true\n")],
            &["add", "--input", "@task.yaml"],
        )
        .vars(&[("DOCTOY_NO_PR", "true"), (FAIL, "1")])
        .code(2)
    );
    has(&out.stderr, "task.yaml:1:1: pr:");
    has(&out.stderr, "DOCTOY_NO_PR");
}

#[test]
fn a_key_loses_to_an_argument_it_conflicts_with() {
    ran!(
        out = case(
            &[("task.yaml", "after: a\n")],
            &["add", "--input", "@task.yaml", "--before", "b"],
        )
    );
    has(&out.stdout, "after: None");
    has(&out.stdout, "before: Some(\"b\")");
}

#[test]
fn two_keys_of_the_document_that_conflict_are_placed_on_the_key() {
    ran!(
        out = case(
            &[("task.yaml", "before: b\nafter: a\n")],
            &["add", "--input", "@task.yaml"],
        )
        .code(2)
    );
    has(&out.stderr, "task.yaml:1:1: before:");
    has(&out.stderr, "cannot be used with '--after <AFTER>'");
}

#[test]
fn a_positional_from_the_document() {
    ran!(
        out = case(
            &[("show.yaml", "id: QCTL-012\n")],
            &["show", "--input", "@show.yaml"],
        )
    );
    has(&out.stdout, "id: \"QCTL-012\"");
}

#[test]
fn both_give_positionals() {
    ran!(
        out = case(
            &[("show.yaml", "id: QCTL-012\n")],
            &["show", "QCTL-007", "--input", "@show.yaml"],
        )
        .code(2)
    );
    has(&out.stderr, "show.yaml:1:1: id:");
    assert_eq!(problems(&out.stderr, "show.yaml"), 1, "{}", out.stderr);
}

#[test]
fn positionals_follow_their_index_order_whatever_the_key_order() {
    ran!(
        out = case(
            &[("copy.yaml", "to: b\nfrom: a\n")],
            &["copy", "--input", "@copy.yaml"],
        )
    );
    has(&out.stdout, "from: \"a\", to: \"b\"");
}

#[test]
fn a_positional_after_a_gap_is_refused() {
    ran!(
        out = case(
            &[("copy.yaml", "to: b\n")],
            &["copy", "--input", "@copy.yaml"],
        )
        .code(2)
    );
    has(&out.stderr, "copy.yaml:1:1: to:");
}

#[test]
fn a_sequence_fills_a_positional_with_many_values() {
    ran!(
        out = case(
            &[("rows.yaml", "rows: [a, b, c]\n")],
            &["rows", "--input", "@rows.yaml"],
        )
    );
    has(&out.stdout, "rows: [\"a\", \"b\", \"c\"]");
}

#[test]
fn a_double_dash_on_the_command_line_refuses_a_positional_key() {
    ran!(
        out = case(
            &[("rows.yaml", "rows: [a]\n")],
            &["rows", "--input", "@rows.yaml", "--"],
        )
        .code(2)
    );
    has(&out.stderr, "rows.yaml:1:1: rows:");
    has(&out.stderr, "--");
}

#[test]
fn an_unknown_key() {
    ran!(
        out = case(
            &[("task.yaml", "title: x\ndue: friday\n")],
            &["add", "--input", "@task.yaml"],
        )
        .code(2)
    );
    has(&out.stderr, "task.yaml:2:1: due: unknown key `due`");
}

#[test]
fn two_unknown_keys_are_reported_together() {
    ran!(
        out = case(
            &[("task.yaml", "due: friday\ntitle: x\nowner: me\n")],
            &["add", "--input", "@task.yaml"],
        )
        .code(2)
    );
    has(&out.stderr, "task.yaml:1:1: due:");
    has(&out.stderr, "task.yaml:3:1: owner:");
}

#[test]
fn a_value_clap_refuses() {
    ran!(
        out = case(
            &[("task.yaml", "title: x\nlabel: a\npriority: high\n")],
            &["add", "--input", "@task.yaml"],
        )
        .code(2)
    );
    has(&out.stderr, "task.yaml:3:11: priority:");
    has(&out.stderr, "invalid digit");
}

#[test]
fn a_value_under_num_args_is_placed_on_its_key() {
    ran!(
        out = case(
            &[("task.yaml", "title: x\npair: a\n")],
            &["add", "--input", "@task.yaml"],
        )
        .code(2)
    );
    has(&out.stderr, "task.yaml:2:1: pair:");
}

#[test]
fn an_error_that_names_a_command_line_argument_is_clap_s_own() {
    ran!(
        out = case(
            &[("task.yaml", TASK)],
            &["add", "--input", "@task.yaml", "--priority", "high"],
        )
        .code(2)
    );
    has(&out.stderr, "invalid value 'high'");
    lacks(&out.stderr, "task.yaml");
}

#[test]
fn a_document_that_cannot_be_read_is_a_usage_error() {
    ran!(out = case(&[], &["add", "--input", "@missing.yaml"]).code(2));
    has(&out.stderr, "missing.yaml");
}

#[test]
fn an_extension_no_format_reads_is_a_usage_error() {
    ran!(out = case(&[("task.txt", TASK)], &["add", "--input", "@task.txt"]).code(2));
    has(&out.stderr, "unsupported extension .txt");
}

#[test]
fn an_input_format_names_a_registered_format() {
    ran!(
        out = case(
            &[("task.txt", TASK)],
            &["add", "--input", "@task.txt", "--input-format", "yaml"],
        )
    );
    has(&out.stdout, "priority: Some(2)");
}

#[test]
fn an_input_format_no_format_has_is_refused() {
    ran!(
        out = case(
            &[("task.txt", TASK)],
            &["add", "--input", "@task.txt", "--input-format", "ini"],
        )
        .code(2)
    );
    has(&out.stderr, "invalid value 'ini'");
}

#[test]
fn a_root_that_is_not_a_mapping_is_refused() {
    ran!(
        out = case(
            &[("task.yaml", "- a\n- b\n")],
            &["add", "--input", "@task.yaml"],
        )
        .code(2)
    );
    has(&out.stderr, "must be a mapping");
}

#[test]
fn a_help_request_reads_no_document() {
    ran!(out = case(&[], &["add", "--input", "@missing.yaml", "--help"]));
    has(&out.stdout, "--input ┆ FILE");
}

#[test]
fn a_subcommand_two_levels_deep() {
    ran!(
        out = case(
            &[("label.yaml", "title: deep\n")],
            &["group", "label", "--input", "@label.yaml"],
        )
    );
    has(&out.stdout, "title: Some(\"deep\")");
}

#[test]
fn an_inferred_subcommand() {
    ran!(
        out = case(
            &[("label.yaml", "title: deep\n")],
            &["gr", "la", "--input", "@label.yaml"],
        )
    );
    has(&out.stdout, "title: Some(\"deep\")");
}

#[test]
fn a_flag_subcommand() {
    ran!(
        out = case(
            &[("sync.yaml", "force: true\n")],
            &["-S", "--input", "@sync.yaml"],
        )
    );
    has(&out.stdout, "Sync { force: true }");
}

#[test]
fn root_options_before_the_subcommand_name() {
    ran!(
        out = case(
            &[("task.yaml", TASK)],
            &["--color", "never", "add", "--input", "@task.yaml"],
        )
    );
    has(&out.stdout, "priority: Some(2)");
}

#[test]
fn a_false_flag_with_no_negation_gives_nothing() {
    ran!(
        out = case(
            &[("sync.yaml", "force: false\n")],
            &["sync", "--input", "@sync.yaml"],
        )
    );
    has(&out.stdout, "Sync { force: false }");
}

#[test]
fn a_global_key_fills_the_flag_of_the_root() {
    ran!(
        out = case(
            &[("task.yaml", "quiet: true\n")],
            &["add", "--input", "@task.yaml"]
        )
    );
    has(&out.stdout, "quiet: true");
}

#[test]
fn a_global_key_loses_to_the_same_flag_before_the_subcommand() {
    ran!(
        out = case(
            &[("task.yaml", "quiet: true\n")],
            &["--quiet", "add", "--input", "@task.yaml"]
        )
        .vars(&[(FAIL, "1")])
        .code(2)
    );
    has(&out.stderr, "task.yaml:1:1: quiet:");
    has(&out.stderr, "--quiet");
}

#[test]
fn a_counted_flag_repeats() {
    ran!(
        out = case(
            &[("task.yaml", "verbose: 3\n")],
            &["add", "--input", "@task.yaml"],
        )
    );
    has(&out.stdout, "verbose: 3");
}

#[test]
fn a_count_of_256_is_refused_on_its_key() {
    ran!(
        out = case(
            &[("task.yaml", "title: x\nverbose: 256\n")],
            &["add", "--input", "@task.yaml"],
        )
        .code(2)
    );
    has(&out.stderr, "task.yaml:2:1: verbose:");
}

#[test]
fn a_delimited_option_reads_a_sequence_and_an_appended_one_reads_each_item() {
    ran!(
        out = case(
            &[("task.yaml", "tag: [a, b]\nlabel: solo\n")],
            &["add", "--input", "@task.yaml"],
        )
    );
    has(&out.stdout, "tag: [\"a\", \"b\"]");
    has(&out.stdout, "label: [\"solo\"]");
}

#[test]
fn a_null_leaves_the_key_out() {
    ran!(
        out = case(
            &[("task.yaml", "title: ~\npriority: null\n")],
            &["add", "--input", "@task.yaml"],
        )
    );
    has(&out.stdout, "title: None, priority: None");
}

#[test]
fn a_value_no_argument_can_take_is_refused_on_its_line() {
    for (body, line) in [
        ("title: [a, b]\n", "task.yaml:1:8: title:"),
        ("title: {a: 1}\n", "task.yaml:1:8: title:"),
        ("pair: [a, b]\n", "task.yaml:1:7: pair:"),
        ("pr: maybe\n", "task.yaml:1:5: pr:"),
        ("label: [a, ~]\n", "task.yaml:1:12: label[1]:"),
        ("tag: [\"a,b\", c]\n", "task.yaml:1:7: tag[0]:"),
    ] {
        ran!(
            out = case(&[("task.yaml", body)], &["add", "--input", "@task.yaml"])
                .label(line)
                .code(2),
            continue
        );
        has(&out.stderr, line);
    }
}

#[test]
fn the_output_flags_are_no_keys() {
    for key in [
        "json",
        "format",
        "stream",
        "color",
        "no-color",
        "tsv",
        "input",
        "input-format",
    ] {
        let body = format!("{key}: x\n");
        ran!(
            out = case(&[("task.yaml", &body)], &["add", "--input", "@task.yaml"])
                .label(key)
                .code(2),
            continue
        );
        has(&out.stderr, &format!("task.yaml:1:1: {key}: unknown key"));
    }
}

#[test]
fn a_parse_error_under_json_carries_no_escape_byte() {
    ran!(
        out = case(
            &[("task.yaml", "priority: high\n")],
            &["add", "--input", "@task.yaml", "--format", "json"],
        )
        .vars(&[("CLICOLOR_FORCE", "1")])
        .code(2)
    );
    lacks(&out.stderr, "\u{1b}");
    let error: serde_json::Value = serde_json::from_str(out.stderr.trim()).unwrap();
    let error = &error["error"];
    assert_eq!(error["kind"], "usage", "{error}");
    assert!(
        error["message"]
            .as_str()
            .is_some_and(|message| message.ends_with("task.yaml has 1 problem")),
        "{error}"
    );
    let problems = error["problems"].as_array().unwrap();
    assert_eq!(problems.len(), 1, "{error}");
    assert_eq!(problems[0]["line"], 1, "{error}");
    assert_eq!(problems[0]["column"], 11, "{error}");
    assert_eq!(problems[0]["field"], "priority", "{error}");
    assert_eq!(error["parse"]["reason"], "value_validation", "{error}");
    assert_eq!(error["parse"]["value"], "high", "{error}");
    assert!(error["usage"].is_string(), "{error}");
}

#[test]
fn a_refused_document_under_json_has_a_problem_for_each_fault() {
    ran!(
        out = case(
            &[("task.yaml", "due: friday\nsize: big\n")],
            &["add", "--input", "@task.yaml", "--format", "json"],
        )
        .code(2)
    );
    let error: serde_json::Value = serde_json::from_str(out.stderr.trim()).unwrap();
    let error = &error["error"];
    assert_eq!(error["kind"], "usage", "{error}");
    assert!(
        error["message"]
            .as_str()
            .is_some_and(|message| message.ends_with("task.yaml has 2 problems")),
        "{error}"
    );
    let problems = error["problems"].as_array().unwrap();
    assert_eq!(problems.len(), 2, "{error}");
    assert_eq!(problems[0]["line"], 1, "{error}");
    assert_eq!(problems[1]["line"], 2, "{error}");
    assert!(error.get("parse").is_none(), "{error}");
}

#[test]
fn a_cli_that_declares_its_own_input_panics() {
    let result = panic::catch_unwind(|| App::<Own>::new("own").run_from(["own"], |_| Ok(Done)));
    let payload = result.unwrap_err();
    let message = payload
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| payload.downcast_ref::<&str>().copied())
        .unwrap_or_default();
    has(message, "App::without_input");
}

#[test]
fn without_input_keeps_the_cli_s_own_flag() {
    let code = App::<Own>::new("own")
        .without_input()
        .view(|cli| cli.output.view())
        .run_from(["own", "--input", "x"], |cli| {
            assert_eq!(cli.input.as_deref(), Some("x"));
            Ok(Done)
        });
    assert_eq!(code, ExitCode::SUCCESS);
}
