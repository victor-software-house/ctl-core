use std::io;

use crate::format::{Format, Output, Sink};

/// How a [`Format`] writes records as they are produced, one call per record.
///
/// [`Format::stream`] returns one for a run under `--stream`, or for
/// [`View::show_records`](crate::view::View::show_records). ctl-core hands
/// it each record as an [`Output`] when the command produces it, in the
/// order produced, then calls [`Self::finish`] once. The writer owns its
/// [`Sink`], so it keeps whatever it needs between records, such as the
/// header of a table already written.
///
/// The sink is a buffer. When [`Self::record`] returns, ctl-core locks stdout,
/// writes what the call wrote, and flushes, so a writer needs no flush of its
/// own, and no lock is held while a record serializes. What a call wrote before
/// it failed is dropped, so no part of a failed record reaches stdout. ctl-core
/// never calls [`Self::record`] after an error, and it calls [`Self::finish`]
/// only when every record before it was written.
pub trait RecordWriter {
    /// Write one record.
    ///
    /// # Errors
    ///
    /// Returns the error from serializing the record or from writing it to the
    /// sink. ctl-core reports it on stderr with exit code 1.
    fn record(&mut self, output: &Output<'_>) -> io::Result<()>;

    /// End the stream: write what closes it, if anything.
    ///
    /// # Errors
    ///
    /// Returns the error from writing it.
    fn finish(self: Box<Self>) -> io::Result<()>;
}

/// The writer a format gets from the default [`Format::stream`]: it calls
/// `write` for each record, and writes nothing between records or at the end.
pub(crate) struct Each<'s, F: Format + ?Sized> {
    format: &'s F,
    sink: Sink<'s>,
}

impl<'s, F: Format + ?Sized> Each<'s, F> {
    /// A writer that gives each record to `format`'s `write` over `sink`.
    pub(crate) fn new(format: &'s F, sink: Sink<'s>) -> Self {
        Self { format, sink }
    }
}

impl<F: Format + ?Sized> RecordWriter for Each<'_, F> {
    fn record(&mut self, output: &Output<'_>) -> io::Result<()> {
        self.format.write(output, &mut self.sink)
    }

    fn finish(self: Box<Self>) -> io::Result<()> {
        Ok(())
    }
}
