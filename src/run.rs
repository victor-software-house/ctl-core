//! Process exit wrapper.

use std::process::ExitCode;

use anyhow::Result;
#[cfg(feature = "help")]
use clap::CommandFactory;

use crate::color::ColorMode;
use crate::format::OutputFormat;

/// Parse `C`, emit styled help if asked, then run `body`.
#[cfg(feature = "help")]
#[must_use]
pub fn go<C: clap::Parser + CommandFactory>(
    bin: &str,
    body: impl FnOnce(C) -> Result<()>,
) -> ExitCode {
    main_with_help::<C>(bin, || {
        let raw: Vec<std::ffi::OsString> = std::env::args_os().collect();
        let words: Vec<String> = raw
            .iter()
            .skip(1)
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        let warnings = crate::flags::chassis_warnings(words.iter().map(String::as_str));
        crate::flags::emit_warnings(bin, &warnings);
        let matches = legacy_command::<C>().get_matches_from(&raw);
        let cli = C::from_arg_matches(&matches).unwrap_or_else(|err| err.exit());
        body(cli)
    })
}

#[cfg(feature = "help")]
fn legacy_command<C: CommandFactory>() -> clap::Command {
    crate::parser::apply_defaults(C::command()).arg_required_else_help(true)
}

/// Parse-free entry: print `{bin}: {error:#}` and return 1.
#[must_use]
pub fn main(bin: &str, body: impl FnOnce() -> Result<()>) -> ExitCode {
    main_with(bin, OutputFormat::Pretty, ColorMode::Auto, body)
}

/// Same as [`main`] with an explicit format and color for the error path.
#[must_use]
pub fn main_with(
    bin: &str,
    format: OutputFormat,
    color: ColorMode,
    body: impl FnOnce() -> Result<()>,
) -> ExitCode {
    match body() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            #[cfg(feature = "view")]
            {
                let _ = crate::view::View::new(format, color).emit_err(bin, &format!("{error:#}"));
            }
            #[cfg(not(feature = "view"))]
            {
                let _ = (format, color);
                eprintln!("{bin}: {error:#}");
            }
            ExitCode::FAILURE
        }
    }
}

/// Emit styled help when `-h`/`--help` is present, then run `body`.
#[cfg(feature = "help")]
#[must_use]
pub fn main_with_help<C: CommandFactory>(bin: &str, body: impl FnOnce() -> Result<()>) -> ExitCode {
    let raw = std::env::args_os().collect::<Vec<_>>();
    if raw.len() == 1 {
        return crate::help::emit_bare::<C>(ColorMode::Auto)
            .map_or(ExitCode::FAILURE, |()| ExitCode::from(2));
    }
    match crate::help::try_emit::<C>() {
        Ok(true) => return ExitCode::SUCCESS,
        Ok(false) => {}
        Err(_) => return ExitCode::FAILURE,
    }
    main(bin, body)
}

#[cfg(all(test, feature = "help"))]
mod tests {
    use clap::Parser;

    use super::legacy_command;

    #[derive(Parser)]
    #[command(version)]
    struct OptionalCli {}

    #[test]
    fn legacy_go_keeps_bare_help_required() {
        assert!(legacy_command::<OptionalCli>().is_arg_required_else_help_set());
    }
}
