//! The documents the consumers render, built as a consumer builds them.
//!
//! Every case uses only ctl-core's public API, with invented data in the
//! shape of each consumer's output. A case name starts with the consumer it
//! stands for. A new case is added to [`CASES`] and gets its baselines with
//! `PARITY_UPDATE=1`.

use clap::Args as _;
use ctl_core::help;
use ctl_core::prelude::{
    Document, EnvPrefix as _, Fields, Notice, NoticeLevel, OutputArgs, Role, Section, Table, Text,
    indoc,
};

/// One case: the name used in its baseline file names, and the function that
/// builds its document.
pub(crate) type Case = (&'static str, fn() -> Document);

/// Every case in the corpus.
pub(crate) const CASES: &[Case] = &[
    ("verctl-prepare", verctl_prepare),
    ("verctl-release-urls", verctl_release_urls),
    ("verctl-status-fragments", verctl_status_fragments),
    ("verctl-publish-notes", verctl_publish_notes),
    ("verctl-asset-targets", verctl_asset_targets),
    ("verctl-version-drift", verctl_version_drift),
    ("qctl-status", qctl_status),
    ("qctl-status-empty-queue", qctl_status_empty_queue),
    ("qctl-check-problems", qctl_check_problems),
    ("qctl-check-ok", qctl_check_ok),
    ("qctl-show-queued", qctl_show_queued),
    ("qctl-show-archived", qctl_show_archived),
    ("qctl-state-line", qctl_state_line),
    ("qctl-close-from-git", qctl_close_from_git),
    ("qctl-hook-snippet", qctl_hook_snippet),
    ("forkctl-status", forkctl_status),
    ("forkctl-plan", forkctl_plan),
    ("forkctl-error", forkctl_error),
    ("forkctl-no-patches", forkctl_no_patches),
    ("forkctl-patch-show", forkctl_patch_show),
    ("shared-instructions", shared_instructions),
    ("rawr-verbatim-line", rawr_verbatim_line),
    ("rawr-empty", rawr_empty),
    ("text-roles", text_roles),
    ("help-root", help_root),
    ("help-subcommand", help_subcommand),
];

/// verctl `prepare`: a table of version bumps with the name as an id column,
/// the changelog as a multi-line paragraph, then fields with one row per
/// consumed fragment, pin, and next command.
fn verctl_prepare() -> Document {
    let bumps = Table::new(["name", "from", "to", "bump"])
        .id_column(0)
        .row(["demo-core", "0.4.2", "0.5.0", "minor"])
        .row(["demo-cli", "1.8.0", "1.8.1", "patch"])
        .row(["demo-docs-site", "2.0.0-rc.1", "2.0.0", "major"]);
    let changelog = indoc! {"
        ## demo-core 0.5.0

        - Add the quiet flag to every report command.
        - Fix a stale lock file after a failed publish.
    "};
    Document::new()
        .table(bumps)
        .paragraph(changelog.trim_end())
        .fields(
            Fields::new()
                .row("consume", ".changeset/quiet-lions-sing.md")
                .row("consume", ".changeset/tidy-owls-wave.md")
                .row("pin", "ci/release.yml")
                .row("pr", "https://example.test/acme/demo-repository/pull/42")
                .row("next", "mise run ver publish")
                .row("dry-run", "nothing written"),
        )
}

/// verctl `publish`: fields only, with release URLs longer than the narrow
/// widths, which a reader copies out of a CI log and so must stay whole.
fn verctl_release_urls() -> Document {
    Document::new().fields(
        Fields::new()
            .row(
                "release",
                "https://example.test/acme/demo-repository/releases/tag/demo-package@12.34.56",
            )
            .row(
                "release",
                "https://example.test/acme/demo-repository/releases/tag/demo-assets@2.0.1",
            )
            .row("dry-run", "nothing published"),
    )
}

/// verctl `status`: one table row per fragment and package with the package
/// as the id column, then the pending count and the largest bump as fields.
fn verctl_status_fragments() -> Document {
    let table = Table::new(["file", "package", "bump"])
        .id_column(1)
        .row(["quiet-lions-sing.md", "demo-core", "minor"])
        .row(["quiet-lions-sing.md", "demo-cli", "patch"])
        .row(["tidy-owls-wave.md", "demo-docs-site", "major"]);
    Document::new()
        .table(table)
        .fields(Fields::new().row("pending", "2").row("max", "major"))
}

/// verctl `publish`: a table whose last column is empty for most rows, the
/// shape that appears when only some packages carry a note.
fn verctl_publish_notes() -> Document {
    let table = Table::new(["name", "version", "via", "note"])
        .id_column(0)
        .row(["demo-core", "0.5.0", "registry", ""])
        .row([
            "demo-cli",
            "1.8.1",
            "registry",
            "tag already exists, skipped",
        ])
        .row(["demo-docs-site", "2.0.0", "release-asset", ""]);
    Document::new().table(table)
}

/// verctl `assets` and `ci`: a table whose first column carries token
/// semantics, with a runner list joined by commas in the middle column.
fn verctl_asset_targets() -> Document {
    let table = Table::new(["target", "runs-on", "asset"])
        .token_column(0)
        .row([
            "linux-x64",
            "linux, x64, large",
            "demo-cli-linux-x64.tar.gz",
        ])
        .row(["macos-arm64", "macos, arm64", "demo-cli-macos-arm64.tar.gz"]);
    Document::new().table(table).fields(
        Fields::new()
            .row("tag", "demo-cli@1.8.1")
            .row("tarball", "dist/demo-cli-1.8.1.tar.gz"),
    )
}

/// verctl `check-versions` when versions drift: a table of the drifted
/// packages, then an error notice with a long message.
fn verctl_version_drift() -> Document {
    let table = Table::new(["name", "default", "local"])
        .id_column(0)
        .row(["demo-core", "0.5.0", "0.4.2"])
        .row(["demo-cli", "-", "1.8.1"]);
    Document::new().table(table).notice(Notice::new(
        NoticeLevel::Error,
        "2 package(s) differ from the default branch; run the prepare step before you merge",
    ))
}

/// qctl `status`: fields with an id value, then a `queue` section and a
/// `horizon` section whose tables use `id_column` and `stacked_below(80, 2)`.
fn qctl_status() -> Document {
    let queue = Table::new(["priority", "id", "title"])
        .id_column(1)
        .stacked_below(80, 2)
        .row(["1*", "TSK-012", "Move the parser onto the shared input loader"])
        .row(["2", "TSK-014", "Add a format flag to the report command"])
        .row([
            "3",
            "TSK-019",
            "Document the retry rules for the publish step, including the long wait after a rate limit",
        ]);
    let horizon = Table::new(["id", "kind", "title"])
        .id_column(0)
        .stacked_below(80, 2)
        .row([
            "TSK-030",
            "research",
            "Compare two ways to cache parsed ledgers",
        ])
        .row([
            "TSK-031",
            "evaluation",
            "Measure start-up time with a thousand rows",
        ]);
    Document::new()
        .fields(
            Fields::new()
                .row("ledger", "tasks.yaml")
                .row("active", Text::new().id("TSK-012")),
        )
        .section(Section::new("queue", Document::new().table(queue)))
        .section(Section::new("horizon", Document::new().table(horizon)))
}

/// qctl `status` with nothing queued: a section whose body is one muted
/// paragraph, and no horizon section.
fn qctl_status_empty_queue() -> Document {
    Document::new()
        .fields(
            Fields::new()
                .row("ledger", "tasks.yaml")
                .row("active", Text::new().id("none")),
        )
        .section(Section::new(
            "queue",
            Document::new().paragraph(Text::new().muted("empty")),
        ))
}

/// qctl `check` with problems: a one-column table with a header inside
/// a `problems` section, then an error notice.
fn qctl_check_problems() -> Document {
    let problems = Table::new(["problem"])
        .row(["TSK-014: outcome is empty"])
        .row(["TSK-019: scope names a path that does not exist (src/retry.rs)"]);
    Document::new()
        .section(Section::new("problems", Document::new().table(problems)))
        .notice(Notice::new(
            NoticeLevel::Error,
            "2 problem(s) in tasks.yaml",
        ))
}

/// qctl `check` when the ledger is clean: a success notice whose message
/// mixes a success span, plain text, and a token span.
fn qctl_check_ok() -> Document {
    Document::new().notice(Notice::new(
        NoticeLevel::Success,
        Text::new().success("ok").then("  ").token("tasks.yaml"),
    ))
}

/// qctl `show` for a queued row: a heading built from an id span and a
/// title, then fields for scope, outcome, and patch.
fn qctl_show_queued() -> Document {
    Document::new()
        .heading(
            Text::new()
                .id("TSK-014")
                .then("  ")
                .then("Add a format flag to the report command"),
        )
        .fields(
            Fields::new()
                .row("scope", "src/report.rs, src/flags.rs")
                .row(
                    "outcome",
                    "The report command accepts a format flag and prints the same rows as JSON.",
                )
                .row("patch", "report-format"),
        )
}

/// qctl `show` for an archived row: a heading that ends in muted text, then
/// a notes field joined with middle dots.
fn qctl_show_archived() -> Document {
    Document::new()
        .heading(
            Text::new()
                .id("TSK-003")
                .then("  ")
                .then("Split the loader from the report command")
                .muted("  (archived 2026-10-02 14:30:00)"),
        )
        .fields(Fields::new().row(
            "notes",
            "merged in a single pull request · no follow-up needed · loader now has its own tests",
        ))
}

/// qctl `start`, `archive`, `park`, `edit`, and `promote`: one paragraph with
/// a success word, two spaces, and an id.
fn qctl_state_line() -> Document {
    Document::new().paragraph(Text::new().success("archived").then("  ").id("TSK-003"))
}

/// qctl `close-from-git` when the hook must push again: fields, a one-column
/// id table, and an error notice whose message is longer than the narrow
/// widths.
fn qctl_close_from_git() -> Document {
    let archived = Table::new(["archived"])
        .id_column(0)
        .row(["TSK-003"])
        .row(["TSK-009"]);
    Document::new()
        .fields(Fields::new().row("ledger", "tasks.yaml"))
        .table(archived)
        .notice(Notice::new(
            NoticeLevel::Error,
            "commit the updated ledger and push again; the hook does not amend",
        ))
}

/// qctl `hook` when the hook config needs a manual edit: a verbatim YAML
/// snippet, then an error notice.
fn qctl_hook_snippet() -> Document {
    let snippet = indoc! {"
        pre-push:
          commands:
            close-from-git:
              run: qctl close-from-git
    "};
    Document::new().verbatim(snippet).notice(Notice::new(
        NoticeLevel::Error,
        "add the command under pre-push.commands in lefthook.yml; qctl does not edit Lefthook config; hook not installed",
    ))
}

/// forkctl `status`: a heading, many fields whose labels differ in length,
/// a patch table with the patch as the id column, and an `operation` section
/// holding its own fields.
fn forkctl_status() -> Document {
    let patches = Table::new(["state", "patch", "kind", "commit", "active"])
        .id_column(1)
        .row(["applied", "tidy-config", "change", "4f9c2ab", "yes"])
        .row(["applied", "pin-runtime", "change", "b71e90d", ""])
        .row(["draft", "drop-legacy-flag", "fix", "draft", ""]);
    let operation = Fields::new()
        .row("id", "op-20261101-1")
        .row("kind", "rebase")
        .row("phase", "replaying")
        .row("recovery", "recovery/op-20261101-1")
        .row(
            "next",
            "forkctl operation continue, forkctl operation abort",
        );
    Document::new()
        .heading("forkctl · status")
        .fields(
            Fields::new()
                .row("repository", "demo-repository")
                .row("manifest", ".ctl/fork.yaml")
                .row("branch", "main (declared main)")
                .row("publish", "proposal")
                .row("downstream", "origin 4f9c2ab")
                .row("base", "upstream/main 81d3e07")
                .row("active", "tidy-config")
                .row("staged", "none")
                .row("unstaged", "src/config.rs, src/lib.rs")
                .row("untracked", "none")
                .row("check", "passed"),
        )
        .table(patches)
        .section(Section::new("operation", Document::new().fields(operation)))
}

/// forkctl in plan mode: a heading, a success notice, a plan whose fields
/// hold long comma lists, then a warning notice with a code and one without.
fn forkctl_plan() -> Document {
    Document::new()
        .heading("forkctl · patch refresh")
        .notice(Notice::new(
            NoticeLevel::Success,
            "mutation plan ready; no state changed",
        ))
        .fields(
            Fields::new()
                .row("command", "patch refresh")
                .row("reads", "manifest, patch ledger, working tree")
                .row(
                    "writes",
                    "patch export, ledger, recovery tag, three generated files under exports",
                )
                .row("hooks", "none")
                .row(
                    "ref updates",
                    "refs/heads/main, refs/tags/recovery/op-20261101-2",
                )
                .row("paths", "src/config.rs, src/lib.rs, tests/config.rs")
                .row("confirmation", "not required"),
        )
        .notice(
            Notice::new(NoticeLevel::Warning, "the base moved since the last check")
                .code("BaseMoved"),
        )
        .notice(Notice::new(
            NoticeLevel::Warning,
            "a newer release of forkctl is available",
        ))
}

/// forkctl when a command fails: a heading, an error notice, and fields with
/// a code, several causes, and a suggested command as a token span.
fn forkctl_error() -> Document {
    Document::new()
        .heading("forkctl · patch finish · error")
        .notice(Notice::new(
            NoticeLevel::Error,
            "the patch check failed, so the patch was not finished",
        ))
        .fields(
            Fields::new()
                .row("code", "check_failed")
                .row("caused by", "src/config.rs is outside the declared scope")
                .row("caused by", "a required text assertion did not match")
                .row(
                    "next",
                    Text::new().token("forkctl patch refresh tidy-config"),
                ),
        )
}

/// forkctl `status` with no patches and no running operation: muted
/// paragraphs and a section whose body is one muted paragraph.
fn forkctl_no_patches() -> Document {
    Document::new()
        .heading("forkctl · status")
        .fields(
            Fields::new()
                .row("repository", "demo-repository")
                .row("active", "none"),
        )
        .paragraph(Text::new().muted("no patches"))
        .section(Section::new(
            "operation",
            Document::new().paragraph(Text::new().muted("none")),
        ))
}

/// forkctl `patch show`: fields whose values are sentences and lists that
/// wrap at the narrow widths.
fn forkctl_patch_show() -> Document {
    Document::new().heading("forkctl · patch show").fields(
        Fields::new()
            .row("patch", "tidy-config")
            .row("kind", "change")
            .row(
                "purpose",
                "Read the config file once at start-up instead of once per command.",
            )
            .row("upstream status", "proposed, not yet reviewed")
            .row("drop when", "upstream reads the config file once")
            .row("scope", "src/config.rs, src/lib.rs, tests/config.rs")
            .row("commit", "4f9c2ab")
            .row("changed paths", "src/config.rs, src/lib.rs")
            .row("export", "exports/tidy-config.patch")
            .row("active", "true"),
    )
}

/// The `instructions` verb of verctl, qctl, and forkctl: a Markdown page
/// printed verbatim, with a line longer than every width, a list, a table,
/// a fenced block, and blank lines.
fn shared_instructions() -> Document {
    let markdown = indoc! {"
        # demo instructions

        Run the commands from the repository root. Every command reads its settings from the environment before it reads the config file, so a CI job sets `DEMO_FORMAT=json` and runs the bare verb.

        ## Commands

        | Command | What it does |
        |:--|:--|
        | `status` | Show the queue and the active row. |
        | `check` | Report every problem in the ledger. |

        - Pass `--dry-run` to see what a command would write · nothing is written.
        - Pass `--format json` for machine output.

        ```sh
        demo status --format json | jq .active
        ```
    "};
    Document::new().verbatim(markdown)
}

/// rawr `setup`: one verbatim line that holds backticks and a path, longer
/// than the narrow widths and never wrapped.
fn rawr_verbatim_line() -> Document {
    Document::new().verbatim("Added the `rawr record` hook to .claude/settings.json")
}

/// rawr `record` and `find` with no result, and qctl `close-from-git` with
/// nothing archived: a document with no blocks, which renders as no text.
fn rawr_empty() -> Document {
    Document::new()
}

/// Role spans: the roles qctl and forkctl build (id, success, muted, token)
/// and the ones help builds (heading, value, warning, error), in one
/// paragraph long enough to wrap at every width.
fn text_roles() -> Document {
    Document::new().paragraph(
        Text::new()
            .span(Role::Heading, "Heading")
            .then(" then plain words, a flag ")
            .token("--format")
            .then(", a row ")
            .id("TSK-012")
            .then(", a metavar ")
            .value("PATH")
            .then(", a note ")
            .muted("(muted)")
            .then(", and the outcomes ")
            .success("done")
            .then(", ")
            .warning("careful")
            .then(", and ")
            .error("failed")
            .then(", followed by enough ordinary words to wrap the paragraph at every width."),
    )
}

/// A small `ledgerctl` command as a CLI built on ctl-core declares it: the
/// shared output flags under the `Output` heading, flags bound to
/// `LEDGERCTL_*` variables with a default, and three subcommands.
fn ledgerctl() -> clap::Command {
    let root = clap::Command::new("ledgerctl")
        .version("1.2.3")
        .about("Work queue kept in the repository")
        .arg(
            clap::Arg::new("config")
                .long("config")
                .value_name("PATH")
                .default_value(".ctl/q.yaml")
                .help("Ledger file to read"),
        )
        .subcommand(clap::Command::new("status").about("Show the queue and the active row"))
        .subcommand(clap::Command::new("add").about("Add a row to the queue"))
        .subcommand(clap::Command::new("show").about("Show one row"));
    OutputArgs::augment_args(root).env_prefix("LEDGERCTL")
}

/// Help for the root command, through `help::document`: usage, about, a
/// commands table, and option sections that stack below 64 columns.
fn help_root() -> Document {
    help::document(ledgerctl())
}

/// Help for a subcommand with a positional argument, a flag with possible
/// values, a flag with a default, and a boolean flag.
fn help_subcommand() -> Document {
    let show = clap::Command::new("show")
        .bin_name("ledgerctl show")
        .about("Show one row")
        .arg(
            clap::Arg::new("id")
                .value_name("ID")
                .required(true)
                .help("The row to show"),
        )
        .arg(
            clap::Arg::new("section")
                .long("section")
                .value_name("NAME")
                .value_parser(["scope", "outcome", "notes"])
                .help("Limit the output to one section"),
        )
        .arg(
            clap::Arg::new("limit")
                .long("limit")
                .value_name("COUNT")
                .default_value("20")
                .help("Most rows to print"),
        )
        .arg(
            clap::Arg::new("all")
                .long("all")
                .action(clap::ArgAction::SetTrue)
                .help("Include archived rows"),
        );
    help::document(OutputArgs::augment_args(show).env_prefix("LEDGERCTL"))
}
