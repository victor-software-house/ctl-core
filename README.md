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

Comfy Table, Anstyle, and Anstream are private rendering engines. Consumer
crates compose ctl-core semantic documents and never import terminal engines.

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

Enable `app` plus `usage` for that shape. Features remain additive and
explicit: `document` has no terminal engine, `render` adds terminal layout,
`view` adds JSON emission, `help` adds Clap help, and `app` composes the
runtime lifecycle.

## Contract

- Domain handlers return data. They do not print, inspect terminal state,
  choose output format, or construct engine tables.
- `Present` maps a serializable result to a semantic `Document`.
- `View` serializes the result directly for JSON and renders its document for
  pretty/colorless output.
- JSON always goes to stdout and never contains ANSI.
- Human failures go to stderr. Quiet suppresses successful human output only.
- Help comes from the Clap graph and uses the same document renderer.
- `App` runs Usage, pre-parse hooks, help, Clap, execution, and
  presentation in that order. `App::usage_spec` lets a consumer enrich the
  mounted Usage document without taking back stream or short-circuit ownership.
- `parser::apply_defaults` keeps `-h`/`--help` and `-V`/`--version` enabled.

Boolean pairs that are domain-specific (`--pr` / `--no-pr`) remain in the
consumer. Use Clap `overrides_with` both ways and `warn_opposites`; the last
flag wins without silence.

See [`docs/presentation.md`](docs/presentation.md) for the architecture and
migration boundary.
