use clap::Args;

/// `-n/--dry-run` with `--preview` as the visible alias.
#[derive(Args, Clone, Debug, Default, Eq, PartialEq)]
#[command(about = None, long_about = None)]
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
