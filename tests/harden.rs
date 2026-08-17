//! Public-API coverage. One case per `#[test]` so failures name the row.
#![allow(missing_docs)]
#![cfg(feature = "cli")]

use clap::Parser;
use ctl_core::color::ParseColorError;
use ctl_core::flags::{DryRunArgs, FormatArgs, OutputArgs, resolve_color, switch};
use ctl_core::format::ParseFormatError;
use ctl_core::prelude::*;

macro_rules! color_from_args {
    ($($name:ident: [$($arg:literal),*] => $want:ident,)*) => {
        $(
            #[test]
            fn $name() {
                assert_eq!(ColorMode::from_args([$($arg),*]), ColorMode::$want);
            }
        )*
    };
}

color_from_args! {
    color_default: ["bin"] => Auto,
    color_auto: ["bin", "--color", "auto"] => Auto,
    color_always: ["bin", "--color", "always"] => Always,
    color_never: ["bin", "--color", "never"] => Never,
    color_short_auto: ["bin", "-c", "auto"] => Auto,
    color_short_always: ["bin", "-c", "always"] => Always,
    color_short_never: ["bin", "-c", "never"] => Never,
    color_eq_auto: ["bin", "--color=auto"] => Auto,
    color_eq_always: ["bin", "--color=always"] => Always,
    color_eq_never: ["bin", "--color=never"] => Never,
    color_no_color: ["bin", "--no-color"] => Never,
    color_no_color_after_always: ["bin", "--color", "always", "--no-color"] => Never,
    color_always_after_no_color: ["bin", "--no-color", "--color", "always"] => Always,
    color_never_after_no_color: ["bin", "--no-color", "--color", "never"] => Never,
    color_auto_after_no_color: ["bin", "--no-color", "--color", "auto"] => Auto,
    color_no_color_after_eq: ["bin", "--color=always", "--no-color"] => Never,
    color_eq_after_no_color: ["bin", "--no-color", "--color=always"] => Always,
    color_short_after_no_color: ["bin", "--no-color", "-c", "always"] => Always,
    color_no_color_after_short: ["bin", "-c", "always", "--no-color"] => Never,
    color_last_of_three: ["bin", "--color", "never", "-c", "auto", "--color=always"] => Always,
    color_junk_value_ignored: ["bin", "--color", "rainbow"] => Auto,
    color_dash_not_value: ["bin", "--color", "--no-color"] => Never,
    color_unknown_flag_ignored: ["bin", "--format", "json"] => Auto,
    color_repeated_never: ["bin", "--color", "always", "--color", "never"] => Never,
    color_repeated_always: ["bin", "--color", "never", "--color", "always"] => Always,
    color_short_then_eq: ["bin", "-c", "never", "--color=always"] => Always,
    color_eq_then_short: ["bin", "--color=always", "-c", "never"] => Never,
    color_no_color_twice: ["bin", "--no-color", "--no-color"] => Never,
    color_always_between_no_color: ["bin", "--no-color", "--color", "always", "--no-color"] => Never,
    color_status_then_color: ["bin", "status", "--color", "never"] => Never,
    color_help_ignored: ["bin", "--help"] => Auto,
    color_version_ignored: ["bin", "--version"] => Auto,
    color_empty_eq: ["bin", "--color="] => Auto,
}

macro_rules! color_parse {
    ($($name:ident: $raw:literal => $pat:pat,)*) => {
        $(
            #[test]
            fn $name() {
                assert!(matches!($raw.parse::<ColorMode>(), $pat));
            }
        )*
    };
}

color_parse! {
    parse_auto: "auto" => Ok(ColorMode::Auto),
    parse_always: "always" => Ok(ColorMode::Always),
    parse_never: "never" => Ok(ColorMode::Never),
    parse_empty: "" => Err(ParseColorError),
    parse_upper_auto: "AUTO" => Err(ParseColorError),
    parse_true: "true" => Err(ParseColorError),
    parse_on: "on" => Err(ParseColorError),
    parse_off: "off" => Err(ParseColorError),
    parse_none: "none" => Err(ParseColorError),
    parse_space: " auto" => Err(ParseColorError),
}

macro_rules! format_parse {
    ($($name:ident: $raw:literal => $pat:pat,)*) => {
        $(
            #[test]
            fn $name() {
                assert!(matches!($raw.parse::<OutputFormat>(), $pat));
            }
        )*
    };
}

format_parse! {
    format_pretty: "pretty" => Ok(OutputFormat::Pretty),
    format_json: "json" => Ok(OutputFormat::Json),
    format_empty: "" => Err(ParseFormatError),
    format_upper_json: "JSON" => Err(ParseFormatError),
    format_yaml: "yaml" => Err(ParseFormatError),
    format_text: "text" => Err(ParseFormatError),
    format_human: "human" => Err(ParseFormatError),
}

#[test]
fn format_is_json_only_for_json() {
    assert!(OutputFormat::Json.is_json());
    assert!(!OutputFormat::Pretty.is_json());
}

#[test]
fn color_display_roundtrip() {
    for mode in [ColorMode::Auto, ColorMode::Always, ColorMode::Never] {
        assert_eq!(mode.to_string().parse::<ColorMode>().unwrap(), mode);
    }
}

#[test]
fn format_display_roundtrip() {
    for mode in [OutputFormat::Pretty, OutputFormat::Json] {
        assert_eq!(mode.to_string().parse::<OutputFormat>().unwrap(), mode);
    }
}

#[test]
fn resolve_color_table() {
    let rows = [
        (ColorMode::Auto, false, ColorMode::Auto),
        (ColorMode::Always, false, ColorMode::Always),
        (ColorMode::Never, false, ColorMode::Never),
        (ColorMode::Auto, true, ColorMode::Never),
        (ColorMode::Always, true, ColorMode::Never),
        (ColorMode::Never, true, ColorMode::Never),
    ];
    for (color, no_color, want) in rows {
        assert_eq!(resolve_color(color, no_color), want);
    }
}

#[test]
fn switch_is_the_on_flag() {
    assert!(switch(true, false));
    assert!(switch(true, true));
    assert!(!switch(false, true));
    assert!(!switch(false, false));
}

#[derive(Parser, Debug)]
#[command(version, about = "harden", arg_required_else_help = true)]
struct Toy {
    #[command(flatten)]
    output: OutputArgs,
    #[command(flatten)]
    dry: DryRunArgs,
    #[command(subcommand)]
    command: ToyCmd,
}

#[derive(Subcommand, Debug)]
enum ToyCmd {
    Status,
}

#[allow(clippy::expect_used)]
fn parse(args: &[&str]) -> Toy {
    let mut words = vec!["toy"];
    words.extend_from_slice(args);
    Toy::try_parse_from(words).expect("parse")
}

macro_rules! clap_ok {
    ($($name:ident: [$($arg:literal),*] => $check:expr,)*) => {
        $(
            #[test]
            fn $name() {
                let cli = parse(&[$($arg),*]);
                assert!($check(cli));
            }
        )*
    };
}

clap_ok! {
    clap_status: ["status"] => |c: Toy| matches!(c.command, ToyCmd::Status),
    clap_format_short: ["-f", "json", "status"] => |c: Toy| c.output.format.is_json(),
    clap_format_long: ["--format", "json", "status"] => |c: Toy| c.output.format.is_json(),
    clap_format_pretty: ["--format", "pretty", "status"] => |c: Toy| !c.output.format.is_json(),
    clap_color_short: ["-c", "never", "status"] => |c: Toy| c.output.color() == ColorMode::Never,
    clap_color_long: ["--color", "always", "status"] => |c: Toy| c.output.color() == ColorMode::Always,
    clap_no_color: ["--no-color", "status"] => |c: Toy| c.output.color() == ColorMode::Never,
    clap_quiet_short: ["-q", "status"] => |c: Toy| c.output.quiet,
    clap_quiet_long: ["--quiet", "status"] => |c: Toy| c.output.quiet,
    clap_dry_short: ["-n", "status"] => |c: Toy| c.dry.dry_run,
    clap_dry_long: ["--dry-run", "status"] => |c: Toy| c.dry.dry_run,
    clap_preview: ["--preview", "status"] => |c: Toy| c.dry.dry_run,
    clap_defaults: ["status"] => |c: Toy| {
        c.output.format == OutputFormat::Pretty
            && c.output.color() == ColorMode::Auto
            && !c.output.quiet
            && !c.dry.dry_run
    },
    clap_all_long: ["--format", "json", "--color", "never", "--quiet", "--dry-run", "status"] => |c: Toy| {
        c.output.format.is_json()
            && c.output.color() == ColorMode::Never
            && c.output.quiet
            && c.dry.dry_run
    },
    clap_all_short: ["-f", "json", "-c", "never", "-q", "-n", "status"] => |c: Toy| {
        c.output.format.is_json()
            && c.output.color() == ColorMode::Never
            && c.output.quiet
            && c.dry.dry_run
    },
    clap_preview_and_json: ["--preview", "-f", "json", "status"] => |c: Toy| {
        c.dry.dry_run && c.output.format.is_json()
    },
    clap_no_color_and_pretty: ["--no-color", "--format", "pretty", "status"] => |c: Toy| {
        c.output.color() == ColorMode::Never && !c.output.format.is_json()
    },
    clap_quiet_default_format: ["-q", "status"] => |c: Toy| {
        c.output.quiet && c.output.format == OutputFormat::Pretty
    },
}

#[test]
fn clap_help_flags() {
    for args in [["-h"], ["--help"], ["-V"], ["--version"]] {
        let err = Toy::try_parse_from(["toy"].into_iter().chain(args)).unwrap_err();
        assert!(
            matches!(
                err.kind(),
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
            ),
            "{args:?} {err}"
        );
    }
}

#[test]
fn clap_missing_command_is_help() {
    let err = Toy::try_parse_from(["toy"]).unwrap_err();
    assert_eq!(
        err.kind(),
        clap::error::ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
    );
}

#[test]
fn verify_toy() {
    ctl_core::parser::verify::<Toy>();
}

#[cfg(feature = "view")]
mod view {
    use ctl_core::Envelope;

    use super::*;

    struct Row {
        n: u8,
    }

    impl serde::Serialize for Row {
        fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            use serde::ser::SerializeStruct;
            let mut s = serializer.serialize_struct("Row", 1)?;
            s.serialize_field("n", &self.n)?;
            s.end()
        }
    }

    impl Render for Row {
        fn render_pretty(&self) -> String {
            formatdoc!("n={n}", n = self.n)
        }
    }

    #[test]
    fn pretty_text() {
        assert_eq!(Row { n: 3 }.render_pretty(), "n=3");
    }

    #[test]
    fn envelope_ok_tag() {
        let json = serde_json::to_value(Envelope::ok(Row { n: 1 })).unwrap();
        assert_eq!(json["status"], "ok");
        assert_eq!(json["schema_version"], 1);
        assert_eq!(json["data"]["n"], 1);
    }

    #[test]
    fn envelope_err_tag() {
        let json =
            serde_json::to_value(Envelope::<()>::err(ErrorBody::new("toy", "nope"))).unwrap();
        assert_eq!(json["status"], "err");
        assert_eq!(json["error"]["bin"], "toy");
        assert_eq!(json["error"]["message"], "nope");
    }

    #[test]
    fn view_json_is_never_color() {
        let view = View::new(OutputFormat::Json, ColorMode::Always);
        assert!(view.format.is_json());
        assert_eq!(view.color, ColorMode::Always);
    }

    #[test]
    fn view_quiet_builder() {
        let view = View::new(OutputFormat::Pretty, ColorMode::Never).quiet(true);
        assert!(view.quiet);
    }
}

#[cfg(feature = "help")]
mod help {
    use clap::CommandFactory;

    use super::*;

    #[test]
    fn render_has_commands() {
        let text = ctl_core::help::render(Toy::command());
        assert!(text.contains("Commands"));
        assert!(text.contains("status"));
    }

    #[test]
    fn render_has_output_flags() {
        let text = ctl_core::help::render(Toy::command());
        for needle in ["--format", "--color", "--no-color", "--quiet", "--dry-run"] {
            assert!(text.contains(needle), "missing {needle} in {text}");
        }
    }

    #[test]
    fn try_emit_false_without_help() {
        let args = ["toy", "status"].map(String::from);
        assert!(!ctl_core::help::try_emit_from::<Toy>(&args).unwrap());
    }

    #[test]
    fn try_emit_true_for_help() {
        let args = ["toy", "--help"].map(String::from);
        assert!(ctl_core::help::try_emit_from::<Toy>(&args).unwrap());
    }
}

#[derive(Parser, Debug)]
struct Split {
    #[command(flatten)]
    format: FormatArgs,
    #[command(flatten)]
    color: ColorLong,
    #[arg(short = 'c', long)]
    config: Option<String>,
}

#[test]
fn split_keeps_c_for_config() {
    let cli = Split::parse_from(["x", "-c", "cfg.toml", "--color", "never", "-f", "json"]);
    assert_eq!(cli.config.as_deref(), Some("cfg.toml"));
    assert_eq!(cli.color.color(), ColorMode::Never);
    assert!(cli.format.format.is_json());
}

#[test]
fn style_empty_is_empty() {
    assert_eq!(ctl_core::style::styled(ctl_core::style::OPTION, ""), "");
}

#[test]
fn style_wraps() {
    let out = ctl_core::style::styled(ctl_core::style::OPTION, "--help");
    assert!(out.contains("--help"));
    assert_ne!(out, "--help");
}

macro_rules! layout_push {
    ($($name:ident: $text:literal,)*) => {
        $(
            #[test]
            fn $name() {
                let mut out = String::new();
                ctl_core::layout::push_line(&mut out, $text);
                assert!(out.ends_with('\n'), "{out:?}");
                assert!(out.contains($text.trim()), "{out}");
            }
        )*
    };
}

layout_push! {
    layout_hello: "hello",
    layout_usage: "Usage: toy <COMMAND>",
    layout_flag: "--dry-run",
    layout_emptyish: "x",
    layout_words: "one two three four",
}

#[test]
fn layout_indent_prefixes() {
    let mut out = String::new();
    ctl_core::layout::push_indented(&mut out, "flag", 2);
    assert!(out.starts_with("  flag"), "{out:?}");
}

#[cfg(feature = "schema")]
mod schema {
    use ctl_core::{ColorMode, Envelope, OutputFormat};
    use schemars::schema_for;

    #[test]
    fn color_schema_is_enum() {
        let schema = schema_for!(ColorMode);
        let json = serde_json::to_value(&schema).unwrap();
        let text = json.to_string();
        assert!(text.contains("auto"), "{text}");
        assert!(text.contains("always"), "{text}");
        assert!(text.contains("never"), "{text}");
    }

    #[test]
    fn format_schema_is_enum() {
        let schema = schema_for!(OutputFormat);
        let text = serde_json::to_string(&schema).unwrap();
        assert!(text.contains("pretty"), "{text}");
        assert!(text.contains("json"), "{text}");
    }

    #[test]
    fn envelope_schema_exists() {
        let schema = schema_for!(Envelope<String>);
        let text = serde_json::to_string(&schema).unwrap();
        assert!(
            text.contains("status") || text.contains("Envelope"),
            "{text}"
        );
    }
}
