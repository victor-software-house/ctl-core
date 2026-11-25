use std::io::{self, Write as _};
use std::mem;

use crate::format::{Format, FormatName, Output, PRETTY_ABOUT, RecordWriter, Sink};

/// The `pretty` format: the result's document laid out for people.
///
/// It is the one built-in format that is not a machine format, so colour
/// follows the colour mode and the stream, and `--quiet` hides it. Its error
/// form is `{bin}: {message}`, the default of [`Format::write_error`].
///
/// Under `--stream` it renders each record's document as the whole output
/// renders it when the record arrives, with one blank line between records,
/// none before the first or after the last. Each record is laid out alone, so
/// the columns of a table are not aligned across records.
#[derive(Clone, Copy, Debug, Default)]
pub struct Pretty;

/// The built-in `pretty` format, ready to register or to give
/// [`View::new`](crate::view::View::new).
pub static PRETTY: Pretty = Pretty;

/// The writer of `pretty` under `--stream`: each record rendered alone, a blank
/// line before every record but the first.
struct Separated<'s> {
    sink: Sink<'s>,
    written: bool,
}

impl RecordWriter for Separated<'_> {
    fn record(&mut self, output: &Output<'_>) -> io::Result<()> {
        let text = self.sink.render(output.document());
        if mem::replace(&mut self.written, true) {
            self.sink.write_all(b"\n")?;
        }
        self.sink.write_all(text.as_bytes())
    }

    fn finish(self: Box<Self>) -> io::Result<()> {
        Ok(())
    }
}

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

    fn stream<'s>(&'s self, sink: Sink<'s>) -> Box<dyn RecordWriter + 's> {
        Box::new(Separated {
            sink,
            written: false,
        })
    }
}
