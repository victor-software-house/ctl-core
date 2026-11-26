//! A build without `error-context` still gives a parse error its reason and
//! its usage line, and leaves clap's other parts out. `test:error-context-off`
//! runs this file with `app` alone; a build with `error-context` skips it,
//! because clap then names the argument in its message.
#![allow(
    missing_docs,
    reason = "an integration test crate has no public API to document"
)]
#![cfg(all(feature = "app", not(feature = "error-context")))]

use std::iter;

use ctl_core::prelude::*;
use serde_json::json;

#[path = "support/child.rs"]
mod child;

#[derive(Parser)]
#[command(name = "toy", version = "1.2.3", about = "toy", env_prefix = "OFFTOY")]
struct Toy {
    #[command(flatten)]
    output: OutputArgs,
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Show one row.
    Show {
        /// The row to show.
        id: String,
    },
}

#[derive(serde::Serialize)]
struct Shown;

impl Present for Shown {
    fn present(&self) -> Document {
        Document::new()
    }
}

const CHILD: &str = "CTL_CORE_ERROR_CONTEXT_OFF_CHILD";

#[test]
fn an_unknown_argument() {
    let Some(output) = child::in_child(CHILD, "an_unknown_argument", &[]) else {
        let args = ["show", "--bogus", "--format", "json"];
        assert_eq!(
            child::case(|| App::<Toy>::new("toy")
                .view(|cli| cli.output.view())
                .run_from(iter::once("toy").chain(args), |_| Ok(Shown))),
            ExitCode::from(2)
        );
        return;
    };
    assert_eq!(child::case_stdout(&output), "");
    let envelope: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(
        envelope["error"],
        json!({
            "bin": "toy",
            "message": "unexpected argument found",
            "kind": "usage",
            "usage": "toy show [OPTIONS] <ID>",
            "parse": {"reason": "unknown_argument"},
        })
    );
}
