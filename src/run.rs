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
    main_with_help::<C>(bin, || body(C::parse()))
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
    match crate::help::try_emit::<C>() {
        Ok(true) => return ExitCode::SUCCESS,
        Ok(false) => {}
        Err(_) => return ExitCode::FAILURE,
    }
    main(bin, body)
}
