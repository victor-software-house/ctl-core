# ctl-core

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/banner-dark.svg">
  <img src="docs/banner.svg" alt="ctl-core — Shared Rust CLI chassis and presentation kernel.">
</picture>

Shared Rust CLI chassis and presentation kernel for `forkctl`, `qctl`,
`verctl`, and `state-sync`. It is a library, not a command.

A consumer owns typed domain requests and results. ctl-core owns parsing,
help, Usage mounts, color policy, terminal layout, streams, errors, and JSON
emission. The same serializable result feeds pretty, colorless, and JSON
output.

ctl-core renders through this workspace's own crates: `ctl-text` for styled
text and widths, `ctl-layout` for tables and records, and `ctl-term` for the
colour, width, and escape decisions of each stream. Consumer crates compose
ctl-core semantic documents; a CLI that needs a widget directly reaches the
crates through `ctl_core::{text, layout, term}`.

## Model

```text
Clap types ──→ App ──→ typed domain result ──┬──→ JSON
                                             └──→ Document ──→ pretty/colorless
```

`Document` is a fluent semantic tree: headings, prose, verbatim blocks, fields,
grids, sections, notices, and rules. It carries meaning, not ANSI or table
borders. The renderer chooses style, wrapping, width, and color; verbatim blocks
preserve preformatted Markdown and protocol lines without wrapping, remove
terminal controls, bidi controls, and Unicode line separators, preserve other
Unicode formatting, and normalize trailing newlines to the document contract.
The `verbatim` feature exports that set as `verbatim::is_unsafe`, so a CLI can
filter text it prints outside a document the same way.

## Use

```rust,ignore
use ctl_core::prelude::*;
use serde::Serialize;

#[derive(Parser)]
#[command(version, about = "example")]
struct Cli {
    #[command(flatten)]
    output: OutputArgs,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Show status.
    Status,
}

#[derive(Serialize)]
struct Status {
    pending: usize,
}

impl Present for Status {
    fn present(&self) -> Document {
        Document::new().fields(
            Fields::new().row("pending", self.pending.to_string()),
        )
    }
}

fn main() -> ExitCode {
    App::<Cli>::new("example")
        .mounted_as("example")
        .view(|cli| cli.output.view())
        .run(|cli| match cli.command {
            Command::Status => Ok(Status { pending: 0 }),
        })
}
```

Enable `app` plus `usage` for that shape. A CLI that already owns `-f` composes
`FormatLong` with `ColorLong`; one that owns `-c` composes `FormatArgs` with
`ColorLong`. Features remain additive and explicit: `document` has no engine,
`render` adds `ctl-text` and `ctl-layout` and renders to a `String` with no
terminal crate, `view` adds JSON emission and `ctl-term`, `help` adds Clap help,
`app` composes the runtime lifecycle, and `surface` adds the Clap-derived
operator model plus shared MiniJinja fragments.

## Operator surface

`Surface::new::<Cli>("q")` extracts the binary and mounted names, version,
descriptions, visible and hidden command trees, aliases, locally declared
arguments and flags, and mounted Usage KDL from Clap. Global flags stay on their
declaring ancestor, while each nested command exposes them separately through
`inherited_arguments`. Templates can render effective flags without mixing
provenance into local declarations. `Surface::note` adds audience-specific prose when
Clap's `about` belongs to a different reader. The `ctl/version.md.jinja`,
`ctl/invocation.md.jinja`, and `ctl/commands.md.jinja` fragments render repeated
operator text without copying command lists or mise's no-`--` rule.

Consumer-owned templates keep domain prose. Their tests render through
`surface::render` and byte-compare the result with committed
`skills/<tool>/SKILL.md` and `src/instructions.md`. Verctl can call
`surface::add_fragments` on its existing MiniJinja environment so Version PRs
render the same fragments.

## Shell completion

The `completion` feature gives every ctl CLI one verb,
`<bin> completion <SHELL>`, for bash, zsh, and fish. Flatten
`completion::CompletionCommand` into the CLI's subcommands, call
`App::completion()`, route the verb to `shell::view(format)`, and
return `command.script("<bin>")`. The script is `clap_complete`'s dynamic
registration: the shell asks the binary for candidates at each TAB through
`COMPLETE=<shell>`. Load it at shell start with
`source <(<bin> completion zsh)`, or let Homebrew install it with
`generate_completions_from_executable(bin/"<bin>", "completion")`.

## Shell integration

The `shell-init` feature gives a ctl CLI `<bin> init <SHELL>` for bash, zsh,
and fish. Declare the command once with
`shell_init::Hook::new("<bin>", "<subcommand>", "<VARIABLE>")`, flatten
`shell_init::InitCommand` into the CLI's subcommands, route the verb to
`shell::view(format)`, and return `command.script(&hook)`. The script runs
`<bin> <subcommand>` once, in the foreground, before the shell's first
prompt, when `VARIABLE` is non-empty and `<bin>` is on `PATH`. Load it with
`eval "$(<bin> init zsh)"` in bash or zsh, or `<bin> init fish | source` in
fish; the script's first line names that line.

## Contract

- Domain handlers return data. They do not print, inspect terminal state,
  choose output format, or construct engine tables.
- `Present` maps a serializable result to a semantic `Document`.
- `View` serializes the result directly for JSON and renders its document for
  pretty/colorless output.
- JSON always goes to stdout and never contains ANSI.
- Human failures go to stderr. Quiet suppresses successful human output only.
- A reader that closes stdout early, such as `head -1`, leaves the exit code
  as the command set it and puts nothing on stderr: success stays 0, and a
  failure keeps its code. `App::broken_pipe(BrokenPipe::Fail)` makes every
  closed pipe a failure.
- Help comes from the Clap graph, uses the same document renderer, and wraps
  long Usage lines within the detected width.
- `App` runs Usage, pre-parse hooks, help, Clap, execution, and
  presentation in that order. `App::usage_spec` lets a consumer enrich the
  mounted Usage document without taking back stream or short-circuit ownership.
- `parser::apply_defaults` keeps `-h`/`--help` and `-V`/`--version` enabled.

Boolean pairs that are domain-specific (`--pr` / `--no-pr`) remain in the
consumer. Use Clap `overrides_with` both ways and `warn_opposites`; the last
flag wins without silence.

See [`docs/presentation.md`](docs/presentation.md) for the architecture and
migration boundary.
