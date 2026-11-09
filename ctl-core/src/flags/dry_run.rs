use clap::Args;

/// `-n/--dry-run`, with `--preview` as a visible alias.
///
/// The flag is a plain `bool`. A command that honours it validates and prints
/// its plan, and writes nothing. A flag can join the struct in a minor
/// release.
#[derive(Args, Clone, Debug, Default, Eq, PartialEq)]
#[command(about = None, long_about = None)]
#[non_exhaustive]
pub struct DryRunArgs {
    /// Validate and print the plan. Write nothing.
    #[arg(
        short = 'n',
        long,
        visible_alias = "preview",
        help_heading = "Execution"
    )]
    pub dry_run: bool,
}
