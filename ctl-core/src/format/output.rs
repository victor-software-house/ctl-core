use std::cell::OnceCell;
use std::{fmt, io};

use crate::color::ColorMode;
use crate::document::Document;
use crate::render::RenderOptions;
use crate::view::{JsonLayout, Outcome};

/// A result as a [`Format`](crate::format::Format) reads it: its data, its
/// document, and whether it succeeded.
///
/// The three faces share one value, so they cannot drift apart. The document
/// is built on first use, so a format that only serializes never pays for the
/// layout model.
pub struct Output<'a> {
    data: &'a dyn erased_serde::Serialize,
    present: &'a dyn Fn() -> Document,
    document: OnceCell<Document>,
    outcome: Outcome,
}

impl<'a> Output<'a> {
    /// The faces of one result. `present` builds its document.
    pub(crate) const fn new(
        data: &'a dyn erased_serde::Serialize,
        present: &'a dyn Fn() -> Document,
        outcome: Outcome,
    ) -> Self {
        Self {
            data,
            present,
            document: OnceCell::new(),
            outcome,
        }
    }

    /// The result as a serializable value. Any serde serializer writes it in
    /// the result's own field order, and `&dyn erased_serde::Serialize`
    /// implements `serde::Serialize`.
    #[must_use]
    pub fn data(&self) -> &dyn erased_serde::Serialize {
        self.data
    }

    /// The document the result's `present` builds, made on the first call.
    #[must_use]
    pub fn document(&self) -> &Document {
        self.document.get_or_init(|| (self.present)())
    }

    /// Whether the result succeeded or failed.
    #[must_use]
    pub const fn outcome(&self) -> Outcome {
        self.outcome
    }
}

/// Names the outcome only: the result and its document have no `Debug`.
impl fmt::Debug for Output<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Output")
            .field("outcome", &self.outcome)
            .finish_non_exhaustive()
    }
}

/// Where a [`Format`](crate::format::Format) writes, and what ctl-core decided
/// for that stream.
///
/// A sink holds the writer, the render options for the stream the result goes
/// to, and the JSON layout. The colour is forced to
/// [`ColorMode::Never`](crate::color::ColorMode::Never) when the format is a
/// [machine format](crate::format::Format::machine), so no machine format can
/// write an escape sequence whatever the colour variables say.
///
/// It implements [`io::Write`], and [`Self::render`] lays a document out as
/// `pretty` does, so a format that only restyles text reuses the engine.
pub struct Sink<'a> {
    writer: &'a mut dyn io::Write,
    stream: ctl_term::Stream,
    options: RenderOptions,
    json_layout: JsonLayout,
}

impl<'a> Sink<'a> {
    /// A sink for `stream` over `writer`. `machine` takes the colour away.
    pub(crate) fn new(
        writer: &'a mut dyn io::Write,
        stream: ctl_term::Stream,
        options: &RenderOptions,
        json_layout: JsonLayout,
        machine: bool,
    ) -> Self {
        let options = if machine {
            options.color(ColorMode::Never)
        } else {
            *options
        };
        Self {
            writer,
            stream,
            options,
            json_layout,
        }
    }

    /// `document` at the colour and width decided for this sink's stream.
    #[must_use]
    pub fn render(&self, document: &Document) -> String {
        self.options.render_for(self.stream, document)
    }

    /// The JSON layout in force: [`JsonLayout::Compact`] or
    /// [`JsonLayout::Pretty`]. `PrettyOnTerminal` is already settled, so a
    /// format never asks the terminal.
    #[must_use]
    pub const fn json_layout(&self) -> JsonLayout {
        self.json_layout
    }

    /// The colour, width, and style settings this sink renders with.
    #[must_use]
    pub const fn render_options(&self) -> RenderOptions {
        self.options
    }
}

impl io::Write for Sink<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.writer.write(bytes)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

/// Names the stream: the writer has no `Debug`.
impl fmt::Debug for Sink<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Sink")
            .field("stream", &self.stream)
            .field("json_layout", &self.json_layout)
            .finish_non_exhaustive()
    }
}
