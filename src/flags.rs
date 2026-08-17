//! Shared clap flatten structs.

use clap::Args;

use crate::color::ColorMode;
use crate::format::OutputFormat;

/// Global pretty/JSON output, color, quiet. Flatten onto the root parser.
///
/// `-c/--color` matches forkctl. A consumer that already owns `-c` (state-sync
/// uses it for `--config`) should flatten [`ColorLong`] instead and keep `-c`.
#[derive(Args, Clone, Debug, Default, Eq, PartialEq)]
pub struct OutputArgs {
    /// Output representation.
    #[arg(
        short = 'f',
        long,
        global = true,
        value_enum,
        default_value = "pretty",
        help_heading = "Output"
    )]
    pub format: OutputFormat,
    /// Pretty-output color policy. JSON never contains ANSI.
    #[arg(
        short = 'c',
        long,
        global = true,
        value_enum,
        default_value = "auto",
        help_heading = "Output"
    )]
    pub color: ColorMode,
    /// Force colorless pretty output. Wins over `--color`.
    #[arg(long, global = true, help_heading = "Output")]
    pub no_color: bool,
    /// Suppress successful pretty output.
    #[arg(short = 'q', long, global = true, help_heading = "Output")]
    pub quiet: bool,
}

impl OutputArgs {
    /// Effective color after `--no-color`.
    #[must_use]
    pub fn color(&self) -> ColorMode {
        if self.no_color {
            ColorMode::Never
        } else {
            self.color
        }
    }

    #[cfg(feature = "view")]
    #[must_use]
    /// Build a [`View`](crate::view::View) from these flags.
    pub fn view(&self) -> crate::view::View {
        crate::view::View::new(self.format, self.color()).quiet(self.quiet)
    }
}

/// `--color` without `-c`, for CLIs that already use `-c` for config.
#[derive(Args, Clone, Debug, Default, Eq, PartialEq)]
pub struct ColorLong {
    /// Pretty-output color policy. JSON never contains ANSI.
    #[arg(
        long,
        global = true,
        value_enum,
        default_value = "auto",
        help_heading = "Output"
    )]
    pub color: ColorMode,
    /// Force colorless pretty output. Wins over `--color`.
    #[arg(long, global = true, help_heading = "Output")]
    pub no_color: bool,
}

/// `-f/--format` and `-q/--quiet` without `-c` (verctl uses `-c` for config).
#[derive(Args, Clone, Debug, Default, Eq, PartialEq)]
pub struct FormatArgs {
    /// Output representation.
    #[arg(
        short = 'f',
        long,
        global = true,
        value_enum,
        default_value = "pretty",
        help_heading = "Output"
    )]
    pub format: OutputFormat,
    /// Suppress successful pretty output.
    #[arg(short = 'q', long, global = true, help_heading = "Output")]
    pub quiet: bool,
}

impl FormatArgs {
    /// Build a [`View`](crate::view::View) with an explicit color policy.
    #[cfg(feature = "view")]
    #[must_use]
    pub fn view(&self, color: ColorMode) -> crate::view::View {
        crate::view::View::new(self.format, color).quiet(self.quiet)
    }
}

impl ColorLong {
    /// Effective color after `--no-color`.
    #[must_use]
    pub fn color(&self) -> ColorMode {
        if self.no_color {
            ColorMode::Never
        } else {
            self.color
        }
    }
}

/// `-n/--dry-run` with `--preview` as the visible alias (verctl).
#[derive(Args, Clone, Debug, Default, Eq, PartialEq)]
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

/// Last-wins boolean pair after clap `overrides_with`.
#[must_use]
/// Last-wins boolean after clap `overrides_with`. `on` is the yes flag.
pub fn switch(on: bool, _off: bool) -> bool {
    on
}

#[cfg(test)]
mod tests {
    use clap::{CommandFactory, Parser};

    use super::{ColorLong, DryRunArgs, FormatArgs, OutputArgs};
    use crate::color::ColorMode;

    #[derive(Parser, Debug)]
    #[command(version)]
    struct Root {
        #[command(flatten)]
        output: OutputArgs,
        #[command(flatten)]
        dry: DryRunArgs,
    }

    #[derive(Parser, Debug)]
    struct ConfigCli {
        #[arg(short = 'c', long)]
        config: Option<String>,
        #[command(flatten)]
        color: ColorLong,
    }

    #[test]
    fn output_shorts_and_longs() {
        let cli = Root::parse_from(["x", "-f", "json", "-c", "never", "-q", "-n"]);
        assert!(cli.output.format.is_json());
        assert_eq!(cli.output.color(), ColorMode::Never);
        assert!(cli.output.quiet);
        assert!(cli.dry.dry_run);
    }

    #[test]
    fn preview_alias_and_no_color() {
        let cli = Root::parse_from(["x", "--preview", "--color", "always", "--no-color"]);
        assert!(cli.dry.dry_run);
        assert_eq!(cli.output.color(), ColorMode::Never);
    }

    #[test]
    fn color_long_leaves_short_c_for_config() {
        let cli = ConfigCli::parse_from(["x", "-c", "path.json", "--color", "never"]);
        assert_eq!(cli.config.as_deref(), Some("path.json"));
        assert_eq!(cli.color.color(), ColorMode::Never);
    }

    #[derive(Parser, Debug)]
    struct SplitCli {
        #[command(flatten)]
        format: FormatArgs,
        #[command(flatten)]
        color: ColorLong,
        #[arg(short = 'c', long)]
        config: Option<String>,
    }

    #[test]
    fn format_args_compose_with_color_long() {
        let cli =
            SplitCli::parse_from(["x", "-f", "json", "-q", "--no-color", "-c", "verctl.toml"]);
        assert!(cli.format.format.is_json());
        assert!(cli.format.quiet);
        assert_eq!(cli.color.color(), ColorMode::Never);
        assert_eq!(cli.config.as_deref(), Some("verctl.toml"));
    }

    #[test]
    fn help_lists_short_and_long() {
        let cmd = Root::command();
        let help = cmd.clone().render_long_help().to_string();
        assert!(help.contains("-n, --dry-run"));
        assert!(help.contains("--preview"));
        assert!(help.contains("-f, --format"));
        assert!(help.contains("-c, --color"));
        assert!(help.contains("--no-color"));
        assert!(help.contains("-q, --quiet"));
        assert!(help.contains("-h, --help"));
        assert!(help.contains("-V, --version"));
    }
}
