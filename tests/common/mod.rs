#![allow(dead_code, missing_docs)]

use clap::{CommandFactory, FromArgMatches, Parser};
use ctl_core::flags::{DryRunArgs, OutputArgs};
use ctl_core::parser::apply_defaults;
use ctl_core::prelude::*;

#[derive(Parser, Debug)]
#[command(version, about = "toy", arg_required_else_help = true)]
pub struct Toy {
    #[command(flatten)]
    pub output: OutputArgs,
    #[command(flatten)]
    pub dry: DryRunArgs,
    #[command(subcommand)]
    pub command: ToyCmd,
}

#[derive(Subcommand, Debug)]
pub enum ToyCmd {
    Status,
}

#[allow(clippy::expect_used)]
pub fn parse(args: &[&str]) -> Toy {
    let mut words = vec!["toy"];
    words.extend_from_slice(args);
    let matches = apply_defaults(Toy::command())
        .try_get_matches_from(&words)
        .expect("parse");
    Toy::from_arg_matches(&matches).expect("from matches")
}
