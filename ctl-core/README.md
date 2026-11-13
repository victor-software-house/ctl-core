# ctl-core

The shared clap chassis for a family of command-line programs.
It is a library, not a command.

A program built on ctl-core owns its typed requests and results.
ctl-core owns the rest: parsing, help, shared flags, colour policy, terminal layout, streams, errors, exit codes, and JSON output.
One serializable result feeds the pretty, colourless, and JSON output.

```text
Clap types ──→ App ──→ typed domain result ──┬──→ JSON
                                             └──→ Document ──→ pretty/colourless
```

`Document` is a tree of semantic blocks: headings, prose, fields, tables, sections, notices, rules, and verbatim text.
It holds meaning, not escape codes or table borders.
The renderer chooses style, wrapping, width, and colour, through the [`ctl-text`](https://crates.io/crates/ctl-text), [`ctl-layout`](https://crates.io/crates/ctl-layout), and [`ctl-term`](https://crates.io/crates/ctl-term) crates.

## What it provides

- `-h`/`--help` and `-V`/`--version`, rendered as documents and wrapped to the terminal's width.
- Shared flags to flatten into the parser: `-c`/`--color`, `-f`/`--format`, `-n`/`--dry-run`, and `-q`/`--quiet`.
  With `#[command(env_prefix = "MYCTL")]` on the root parser, every long option also reads `MYCTL_<LONG>` from the environment.
- JSON on stdout only, never coloured, and human failures on stderr.
  Quiet hides only successful human output.
- A reader that closes stdout early, such as `head -1`, leaves the exit code as the command set it.
  `App::broken_pipe(BrokenPipe::Fail)` turns every closed pipe into a failure.
- `<bin> completion <SHELL>` and `<bin> init <SHELL>` for bash, zsh, and fish, behind the `completion` and `shell-init` features.
- A loader for declared YAML or TOML input that validates once and reports every problem with its file and line, behind `input-yaml` and `input-toml`.
  Validation with garde is behind `validate`, and `input` is `input-yaml` with `validate`.
  - `Input::place` turns a `FieldPath`, the steps down to a field, into its line and column, so a CLI places a problem of its own as the loader places garde's.
  - `Input::place_key` gives the line and column of the key instead, for a fault about the key itself.
  - `Input::flattened` declares a field that `#[serde(flatten)]` puts under its parent, so its problems print and sit where the file writes them.
  - `Tagged<T>` reads a field with its YAML tag, so a CLI can give a tag such as `!override` a meaning of its own.
  - `Input::top_level` lists the entries of the root mapping or table with the byte range of each value, how it is written and its string.

## Example

```rust,no_run
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
        Document::new().fields(Fields::new().row("pending", self.pending.to_string()))
    }
}

fn main() -> ExitCode {
    App::<Cli>::new("example")
        .view(|cli| cli.output.view())
        .run(|cli| match cli.command {
            Command::Status => Ok(Status { pending: 0 }),
        })
}
```

`example status` prints the field as a record.
`example status --format json` prints the same `Status` value as JSON, with no escape byte, whatever the terminal.
A failure in JSON mode prints an error envelope instead.

## Features

The default feature is `app`.
Each feature turns on only what its code calls.

| Feature | Adds |
|:--|:--|
| `document` | Semantic documents, with no terminal engine |
| `render` | Renders documents to a `String` through `ctl-text`, `ctl-layout`, and the `ctl-term` encoder, with a replaceable `Theme` |
| `json` | Serde models |
| `view` | Pretty, JSON, and colourless output over one model, with `ctl-term` deciding colour and width |
| `cli` | Clap helpers (`OutputArgs`, `DryRunArgs`) and parser defaults |
| `help` | Styled help through the document renderer |
| `app` | The whole lifecycle: parse, run, present, and exit |
| `usage` | The mise Usage spec (`--usage-spec`) for a mounted task |
| `surface-model`, `surface-serde`, `surface-templates`, `surface` | The operator model drawn from the Clap types, its serialization, and shared MiniJinja fragments |
| `unsafe-chars` | `unsafe_chars::is_unsafe` and `strip_unsafe`, the characters never printed as they are |
| `schema` | JSON Schema for the envelope |
| `validate`, `input-yaml`, `input-toml`, `input` | Shared garde rules, and the declared-input loader for each format; `input` is `input-yaml` with `validate` |
| `served` | Files rendered from templates that name their own target |
| `shell`, `completion`, `shell-init` | The shell verbs |
| `fixture` | Temporary file trees for integration tests |

## Documentation

The API reference is on [docs.rs](https://docs.rs/ctl-core), with each feature and the items it enables.

## License

MIT.
