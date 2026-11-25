//! A command that produces results over time: a header record, then records,
//! each `Result`, written when they are produced or all at once.

use std::cell::RefCell;
use std::io::{self, Write};
use std::process::ExitCode;
use std::rc::Rc;
use std::{fmt, mem};

use serde::{Serialize, Serializer};

use crate::document::{Block, Document};
use crate::format::{Format, Output, RecordWriter};
use crate::view::{Outcome, Present, View, write_stdout};

/// A [`Present`] result behind one object-safe trait, so everything that
/// writes records compiles once, whatever the record type is.
pub(super) trait Record {
    /// The record's data, as [`Output::data`] serves it.
    fn data(&self) -> &dyn erased_serde::Serialize;
    /// The record's document.
    fn present(&self) -> Document;
    /// [`Present::outcome`].
    fn outcome(&self) -> Outcome;
    /// [`Present::exit_code`].
    fn exit_code(&self) -> u8;
}

impl<P: Present> Record for P {
    fn data(&self) -> &dyn erased_serde::Serialize {
        self
    }

    fn present(&self) -> Document {
        Present::present(self)
    }

    fn outcome(&self) -> Outcome {
        Present::outcome(self)
    }

    fn exit_code(&self) -> u8 {
        Present::exit_code(self)
    }
}

/// What an iterator of records yields: a record, or the error that ends the
/// stream.
type Item<'a> = anyhow::Result<Box<dyn Record + 'a>>;

/// A header record, then records the command produces one at a time.
///
/// Every record is [`Present`] and serializes as a map with a string `type`
/// field, so a reader tells the records apart. Write
/// `#[serde(tag = "type")]` on an enum of the record types. A debug build
/// panics on a record that does not, header included.
///
/// Under `--stream` each record is written when the iterator yields it, by the
/// format's [`RecordWriter`], and stdout is flushed after each one. A record
/// reaches stdout whole, in one write: stdout is not locked while the iterator
/// or a record's serialization runs, and a record that fails to serialize
/// leaves no partial line.
///
/// Without `--stream` the records are collected and written as one result.
/// Its data is one sequence, header first, so `json` writes one array. Its
/// document is the header's document followed by each record's. Adjacent
/// documents that are each one table with equal headers merge into one table.
///
/// Every record goes to stdout. A record's [`Present::outcome`] and
/// [`Present::exit_code`] are ignored. The run exits 0 unless the iterator
/// yields an error.
///
/// An error from the iterator ends the stream. ctl-core stops pulling, writes
/// the records already pulled, and closes the output, so a `json` array is
/// complete. `App` then writes the error to stderr in the format's error form,
/// with its exit code and `kind`, as for a handler error.
///
/// When the reader closes stdout, ctl-core stops pulling and drops the
/// iterator, unless the iterator already failed: then the error is the result,
/// and `App` writes it. `--quiet` with no format named drains the iterator and
/// writes no record. Side effects and a late error still happen.
///
/// ```
/// use ctl_core::document::Document;
/// use ctl_core::view::{Present, Records};
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// #[serde(tag = "type", rename_all = "snake_case")]
/// enum Event {
///     Started { total: usize },
///     Done { item: usize },
/// }
///
/// impl Present for Event {
///     fn present(&self) -> Document {
///         Document::new().paragraph(match self {
///             Self::Started { total } => format!("{total} items"),
///             Self::Done { item } => format!("item {item} done"),
///         })
///     }
/// }
///
/// let records = Records::new(
///     Event::Started { total: 2 },
///     (1..=2).map(|item| Ok(Event::Done { item })),
/// );
/// # let _ = records;
/// ```
#[must_use = "a stream of records writes nothing until it is run"]
pub struct Records<'a> {
    header: Box<dyn Record + 'a>,
    rest: Box<dyn Iterator<Item = Item<'a>> + 'a>,
}

/// Names the type only: a record and its iterator have no `Debug`.
impl fmt::Debug for Records<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("Records").finish_non_exhaustive()
    }
}

impl<'a> Records<'a> {
    /// A stream of `header`, then each record of `rest` as it is pulled.
    ///
    /// This is the only generic part: each record is boxed at once behind a
    /// private trait, and everything that writes records compiles once.
    pub fn new<R: Present + 'a>(
        header: R,
        rest: impl Iterator<Item = anyhow::Result<R>> + 'a,
    ) -> Self {
        let rest = rest.map(|item| item.map(|record| typed(Box::new(record))));
        Self {
            header: typed(Box::new(header)),
            rest: Box::new(rest),
        }
    }
}

/// The panic message of a record without a string `type` field.
#[cfg(debug_assertions)]
const UNTYPED: &str = "a record of `Records` must serialize as a map with a string `type` field; \
     put `#[serde(tag = \"type\")]` on the enum of record types";

/// `record`, after a debug build checked that it serializes as a map with a
/// string `type` entry. A record that does not serialize passes, as the format
/// reports that error.
#[cfg(debug_assertions)]
fn typed(record: Box<dyn Record + '_>) -> Box<dyn Record + '_> {
    let typed = match serde_json::to_value(record.data()) {
        Ok(value) => matches!(value.get("type"), Some(serde_json::Value::String(_))),
        Err(_) => true,
    };
    assert!(typed, "{UNTYPED}");
    record
}

/// `record`: a release build does not check.
#[cfg(not(debug_assertions))]
fn typed(record: Box<dyn Record + '_>) -> Box<dyn Record + '_> {
    record
}

/// What a command returns: one value, or a stream of [`Records`].
///
/// A handler that returns a [`Present`] value needs no `Reply`: [`Respond`]
/// takes either. A CLI whose subcommands return a value and records returns a
/// `Reply` from its handler, made by [`Self::value`] or by converting
/// [`Records`].
#[must_use = "a reply writes nothing until it is run"]
pub struct Reply<'a>(Inner<'a>);

/// The two things a [`Reply`] is: one value, or a stream of records.
enum Inner<'a> {
    Value(Box<dyn Record + 'a>),
    Records(Records<'a>),
}

impl<'a> Reply<'a> {
    /// A reply that is the one value `value`.
    pub fn value(value: impl Present + 'a) -> Self {
        Self(Inner::Value(Box::new(value)))
    }

    /// The exit code the reply ends the run with when its output was written:
    /// the value's, or 0 for records.
    fn exit_code(&self) -> u8 {
        match &self.0 {
            Inner::Value(value) => value.exit_code(),
            Inner::Records(_) => 0,
        }
    }
}

/// Names the kind of reply only: a record and its iterator have no `Debug`.
impl fmt::Debug for Reply<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("Reply")
            .field(&match self.0 {
                Inner::Value(_) => "value",
                Inner::Records(_) => "records",
            })
            .finish()
    }
}

impl<'a> From<Records<'a>> for Reply<'a> {
    fn from(records: Records<'a>) -> Self {
        Self(Inner::Records(records))
    }
}

/// Keeps [`Respond`] to the types ctl-core implements it for.
mod sealed {
    use std::io;

    use crate::view::View;

    /// Implemented only inside ctl-core. Its path is private, so no other crate
    /// can name it, and its methods are the ones `App` runs a result with.
    pub trait Sealed: Sized {
        /// The exit code the run ends with when the output was written.
        fn reply_exit_code(&self) -> u8;

        /// Write the result through `view`; the error is the one a stream's
        /// iterator ended with.
        fn deliver(self, view: &View) -> io::Result<Option<anyhow::Error>>;
    }
}

impl<P: Present> sealed::Sealed for P {
    fn reply_exit_code(&self) -> u8 {
        Present::exit_code(self)
    }

    fn deliver(self, view: &View) -> io::Result<Option<anyhow::Error>> {
        view.show_record(&self).map(|()| None)
    }
}

impl sealed::Sealed for Reply<'_> {
    fn reply_exit_code(&self) -> u8 {
        self.exit_code()
    }

    fn deliver(self, view: &View) -> io::Result<Option<anyhow::Error>> {
        match self.0 {
            Inner::Value(value) => view.show_record(&*value).map(|()| None),
            Inner::Records(records) => view.run_records(records),
        }
    }
}

/// What `App::run` takes from a handler: any [`Present`] value, or a
/// [`Reply`].
///
/// It is sealed. A handler that returns a value compiles as it always did, and
/// one whose subcommands return a value and records returns [`Reply`].
pub trait Respond: sealed::Sealed {}

impl<P: Present> Respond for P {}

impl Respond for Reply<'_> {}

/// The records of a collected run as one sequence, header first.
struct Sequence<'a, 'r>(&'a [Box<dyn Record + 'r>]);

impl Serialize for Sequence<'_, '_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(self.0.iter().map(|record| record.data()))
    }
}

/// The document of a collected run: each record's document in turn, where
/// adjacent documents that are each one table with equal headers make one
/// table. Blocks and rows are moved, not copied.
fn merged(records: &[Box<dyn Record + '_>]) -> Document {
    let mut blocks = Vec::new();
    let mut open = false;
    for record in records {
        match <[Block; 1]>::try_from(record.present().into_blocks()) {
            Ok([Block::Table(table)]) => match blocks.last_mut() {
                Some(Block::Table(run)) if open && run.headers() == table.headers() => {
                    run.extend_rows(table);
                }
                _ => {
                    blocks.push(Block::Table(table));
                    open = true;
                }
            },
            Ok([block]) => {
                blocks.push(block);
                open = false;
            }
            Err(part) => {
                blocks.extend(part);
                open = false;
            }
        }
    }
    blocks.into_iter().fold(Document::new(), Document::block)
}

/// Pull `records` to the end or the first error and write nothing.
fn drain(records: Records<'_>) -> Option<anyhow::Error> {
    let Records { header, mut rest } = records;
    drop(header);
    rest.find_map(Result::err)
}

/// The bytes a record writer wrote, kept apart from stdout until the record is
/// complete, so stdout is locked only to write them and a record that fails
/// half way leaves no partial line.
#[derive(Clone, Default)]
struct Pending(Rc<RefCell<Vec<u8>>>);

impl Write for Pending {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Pending {
    /// Write what the writer wrote to stdout, flush it, and start over.
    fn deliver(&self) -> io::Result<()> {
        let bytes = mem::take(&mut *self.0.borrow_mut());
        let mut stdout = io::stdout().lock();
        stdout.write_all(&bytes)?;
        stdout.flush()
    }

    /// Forget what the writer wrote.
    fn discard(&self) {
        self.0.borrow_mut().clear();
    }
}

/// Give `record` to `writer` and deliver what it wrote. A record that fails
/// to write reaches stdout in no part.
fn emit(writer: &mut dyn RecordWriter, pending: &Pending, record: &dyn Record) -> io::Result<()> {
    let present = || record.present();
    let output = Output::new(record.data(), &present, Outcome::Success);
    writer
        .record(&output)
        .and_then(|()| pending.deliver())
        .inspect_err(|_| pending.discard())
}

/// End `writer` and deliver what it wrote to close the stream.
fn finish(writer: Box<dyn RecordWriter + '_>, pending: &Pending) -> io::Result<()> {
    writer
        .finish()
        .and_then(|()| pending.deliver())
        .inspect_err(|_| pending.discard())
}

/// The result of a run whose iterator ended with `failure`, if it did, after
/// `written` wrote the output. A closed stdout never hides the failure, which
/// carries the exit code. Any other write error is the error to report, and
/// `App` writes it to stderr as it writes the failure.
fn settle(
    failure: Option<anyhow::Error>,
    written: io::Result<()>,
) -> io::Result<Option<anyhow::Error>> {
    match written {
        Ok(()) => Ok(failure),
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => {
            failure.map_or(Err(error), |failure| Ok(Some(failure)))
        }
        Err(error) => Ok(Some(error.into())),
    }
}

/// Write each record of `records` as it is pulled.
fn stream_records(
    view: &View,
    format: &dyn Format,
    records: Records<'_>,
) -> io::Result<Option<anyhow::Error>> {
    let Records { header, rest } = records;
    let mut pending = Pending::default();
    let delivery = pending.clone();
    let mut writer = format.stream(view.sink(&mut pending, ctl_term::Stream::Stdout, format));
    let mut failure = None;
    let mut written = emit(&mut *writer, &delivery, &*header);
    if written.is_ok() {
        for item in rest {
            match item {
                Ok(record) => written = emit(&mut *writer, &delivery, &*record),
                Err(error) => {
                    failure = Some(error);
                    break;
                }
            }
            if written.is_err() {
                break;
            }
        }
    }
    if written.is_ok() {
        written = finish(writer, &delivery);
    }
    settle(failure, written)
}

/// Pull `records` to the end or the first error, and write them as one result.
fn collect_records(
    view: &View,
    format: &dyn Format,
    records: Records<'_>,
) -> io::Result<Option<anyhow::Error>> {
    let Records { header, rest } = records;
    let mut all = vec![header];
    let mut failure = None;
    for item in rest {
        match item {
            Ok(record) => all.push(record),
            Err(error) => {
                failure = Some(error);
                break;
            }
        }
    }
    let data = Sequence(&all);
    let present = || merged(&all);
    let output = Output::new(&data, &present, Outcome::Success);
    let mut written = Vec::new();
    format.write(
        &output,
        &mut view.sink(&mut written, ctl_term::Stream::Stdout, format),
    )?;
    settle(failure, write_stdout(&written))
}

impl View {
    /// Write `records` as the view says, and return the error their iterator
    /// ended with.
    ///
    /// Under [`Self::stream`] each record is written to stdout when the
    /// iterator yields it, and stdout is flushed after each. Otherwise the
    /// records are collected and written as one result.
    ///
    /// Either way an error from the iterator stops it. The records before it
    /// are written and the output is closed, so a `json` array is complete.
    /// Then the error is returned for the caller to write. `--quiet` with no
    /// format named drains the iterator and writes no record.
    ///
    /// # Errors
    ///
    /// Either the error the iterator ended with, or an [`io::Error`] from
    /// writing to stdout, so a caller tells them apart by downcasting. A
    /// closed stdout is an [`io::Error`] of kind [`io::ErrorKind::BrokenPipe`];
    /// the iterator is then dropped without another pull. A closed stdout
    /// after the iterator failed returns the iterator's error, so the failure
    /// is never hidden.
    pub fn show_records(self, records: Records<'_>) -> anyhow::Result<ExitCode> {
        match self.run_records(records)? {
            None => Ok(ExitCode::SUCCESS),
            Some(error) => Err(error),
        }
    }

    /// [`Self::show_records`] before it turns the error of the iterator into
    /// a return value, so a closed stdout and an iterator error stay apart.
    pub(crate) fn run_records(self, records: Records<'_>) -> io::Result<Option<anyhow::Error>> {
        let view = self.settled()?;
        let (format, defaulted) = view.chosen()?;
        if view.hides(format, defaulted) {
            Ok(drain(records))
        } else if view.stream {
            stream_records(&view, format, records)
        } else {
            collect_records(&view, format, records)
        }
    }

    /// Write `value` as a stream of one record. It goes through the format's
    /// writer without the driver of [`Records`], so a CLI that never returns
    /// records does not link the collecting path.
    pub(super) fn stream_value(self, value: &dyn Record) -> io::Result<()> {
        let view = self.settled()?;
        let (format, defaulted) = view.chosen()?;
        if view.hides(format, defaulted) {
            return Ok(());
        }
        let mut pending = Pending::default();
        let delivery = pending.clone();
        let mut writer = format.stream(view.sink(&mut pending, ctl_term::Stream::Stdout, format));
        emit(&mut *writer, &delivery, value)?;
        finish(writer, &delivery)
    }
}

#[cfg(all(test, debug_assertions))]
mod tests {
    use std::iter;

    use serde::ser::SerializeMap as _;
    use serde::{Serialize, Serializer};

    use super::{Records, drain};
    use crate::document::Document;
    use crate::view::Present;

    /// An enum with no `tag = "type"`: it serializes as `{"Tick": {...}}`.
    #[derive(Serialize)]
    enum Untagged {
        Tick { n: usize },
    }

    impl Present for Untagged {
        fn present(&self) -> Document {
            Document::new()
        }
    }

    #[test]
    #[should_panic(expected = "string `type` field")]
    fn a_record_without_a_type_field_panics_in_a_debug_build() {
        let _ = Records::new(Untagged::Tick { n: 1 }, iter::empty());
    }

    /// A record that has a `type` field only when `typed` is set.
    struct Maybe {
        typed: bool,
    }

    impl Serialize for Maybe {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            let mut map = serializer.serialize_map(Some(1))?;
            if self.typed {
                map.serialize_entry("type", "tick")?;
            } else {
                map.serialize_entry("n", &1)?;
            }
            map.end()
        }
    }

    impl Present for Maybe {
        fn present(&self) -> Document {
            Document::new()
        }
    }

    #[test]
    #[should_panic(expected = "string `type` field")]
    fn a_record_after_the_header_without_a_type_field_panics_in_a_debug_build() {
        let records = Records::new(
            Maybe { typed: true },
            iter::once(Ok(Maybe { typed: false })),
        );
        let _ = drain(records);
    }
}
