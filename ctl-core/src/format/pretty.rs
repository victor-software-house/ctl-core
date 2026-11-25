use std::io::{self, Write as _};

use crate::format::{Format, FormatName, Output, PRETTY_ABOUT, Sink};

/// The `pretty` format: the result's document laid out for people.
///
/// It is the one built-in format that is not a machine format, so colour
/// follows the colour mode and the stream, and `--quiet` hides it. Its error
/// form is `{bin}: {message}`, the default of [`Format::write_error`].
#[derive(Clone, Copy, Debug, Default)]
pub struct Pretty;

/// The built-in `pretty` format, ready to register or to give
/// [`View::new`](crate::view::View::new).
pub static PRETTY: Pretty = Pretty;

impl Format for Pretty {
    fn name(&self) -> FormatName {
        FormatName::PRETTY
    }

    fn about(&self) -> &'static str {
        PRETTY_ABOUT
    }

    fn machine(&self) -> bool {
        false
    }

    fn write(&self, output: &Output<'_>, sink: &mut Sink<'_>) -> io::Result<()> {
        let text = sink.render(output.document());
        sink.write_all(text.as_bytes())
    }
}
