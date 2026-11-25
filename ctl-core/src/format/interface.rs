use std::io::{self, Write as _};

use crate::document::{Document, Text};
use crate::format::{FormatName, Output, Sink};
use crate::model::ErrorBody;

/// How a result becomes output: one implementation per `--format` name.
///
/// ctl-core's own formats, [`PRETTY`](crate::format::PRETTY),
/// [`JSON`](crate::format::JSON), [`TSV`](crate::format::TSV),
/// [`CSV`](crate::format::CSV), and [`MARKDOWN`](crate::format::MARKDOWN),
/// implement it and nothing more, so a CLI's format can do whatever they do.
/// A CLI declares its format as a `static` and registers a reference on
/// [`Formats`](crate::format::Formats). A [`View`](crate::view::View) holds
/// that reference, never the format, so it stays `Copy`.
///
/// The trait takes no type parameter and has no generic method, so a registry
/// can hold formats chosen at run time. A format reads the result through
/// [`Output`], which offers its serialized face and its document, and writes
/// through [`Sink`], which carries the stream's decisions. It never reads the
/// terminal itself.
///
/// `Sync` is required so a `static` implementation is allowed and a view that
/// holds a reference stays `Send` and `Sync`.
pub trait Format: Sync {
    /// The `--format` value that selects this format, such as `tsv`.
    fn name(&self) -> FormatName;

    /// One line that says what the format writes, for help and completion.
    fn about(&self) -> &'static str;

    /// The long flag that selects this format without `--format`, such as
    /// `json` for `--json`. The default is none.
    fn shorthand(&self) -> Option<&'static str> {
        None
    }

    /// Whether the output is for programs: it never carries an escape
    /// sequence, and `--quiet` does not hide it when a flag, a shorthand, or a
    /// variable named it. The default is `true`.
    fn machine(&self) -> bool {
        true
    }

    /// Write one whole result to `sink`.
    ///
    /// # Errors
    ///
    /// Returns the error from serializing the result or from writing it.
    fn write(&self, output: &Output<'_>, sink: &mut Sink<'_>) -> io::Result<()>;

    /// Write a failure that has no typed result, such as a parse error or an
    /// error the handler returned. The default is `{bin}: {message}` laid out
    /// as a document, as `pretty` writes it.
    ///
    /// # Errors
    ///
    /// Returns the error from writing it.
    fn write_error(&self, error: &ErrorBody, sink: &mut Sink<'_>) -> io::Result<()> {
        let document = Document::new().paragraph(
            Text::new()
                .error(error.bin.as_str())
                .then(": ")
                .then(error.message.as_str()),
        );
        let text = sink.render(&document);
        sink.write_all(text.as_bytes())
    }
}
