//! The toy CLI whose `.text` size `check:size` holds to a budget at each
//! feature tier.
//!
//! One status command, built four ways: `cli` parses and prints, `help` adds
//! ctl-core's help, `view` presents a document through a view, and `app` runs
//! the whole lifecycle. The budgets and the features of each tier are in
//! `[workspace.metadata.size]` of the root `Cargo.toml`.
//!
//! ```text
//! cargo build --release --package ctl-core --example size --no-default-features --features app
//! ```

#[cfg(not(feature = "view"))]
use std::io::{self, Write as _};

use ctl_core::prelude::*;

#[derive(Parser)]
#[command(version, about = "toy")]
struct Cli {
    #[command(flatten)]
    output: OutputArgs,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Show status.
    Status {
        /// Name to show.
        name: Option<String>,
    },
}

#[cfg(feature = "view")]
#[derive(serde::Serialize)]
struct Status {
    name: String,
    pending: usize,
}

#[cfg(feature = "view")]
impl Present for Status {
    fn present(&self) -> Document {
        Document::new()
            .fields(
                Fields::new()
                    .row("name", self.name.clone())
                    .row("pending", self.pending.to_string()),
            )
            .table(
                Table::new(["id", "title"])
                    .row(["T-1", "first row"])
                    .row(["T-2", "second row"]),
            )
    }
}

#[cfg(feature = "app")]
fn main() -> ExitCode {
    App::<Cli>::new("toy")
        .view(|cli| cli.output.view())
        .run(|cli| match cli.command {
            Command::Status { name } => Ok(Status {
                name: name.unwrap_or_default(),
                pending: 1,
            }),
        })
}

#[cfg(all(feature = "view", not(feature = "app")))]
fn main() -> ExitCode {
    let cli = Cli::parse();
    let view = cli.output.view();
    let Command::Status { name } = cli.command;
    view.show(&Status {
        name: name.unwrap_or_default(),
        pending: 1,
    })
    .unwrap_or(ExitCode::FAILURE)
}

#[cfg(all(feature = "help", not(feature = "view")))]
fn main() -> ExitCode {
    if ctl_core::help::try_emit::<Cli>().unwrap_or(false) {
        return ExitCode::SUCCESS;
    }
    print_status()
}

#[cfg(not(any(feature = "help", feature = "view")))]
fn main() -> ExitCode {
    print_status()
}

/// Parses the command line and prints the name and the output format, for the
/// tiers below `view`.
#[cfg(not(feature = "view"))]
fn print_status() -> ExitCode {
    let cli = Cli::parse();
    let Command::Status { name } = cli.command;
    writeln!(
        io::stdout(),
        "{} {:?}",
        name.unwrap_or_default(),
        cli.output.format
    )
    .map_or(ExitCode::FAILURE, |()| ExitCode::SUCCESS)
}
