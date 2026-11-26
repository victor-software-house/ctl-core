use std::io::{self, Write as _};

use crate::document::{Document, Text};
use crate::format::stream::Each;
use crate::format::{FormatName, Output, RecordWriter, Sink};
use crate::model::{ErrorBody, InputProblem};

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

    /// The writer for a run under `--stream`, which writes each record when
    /// the command produces it.
    ///
    /// The default writer calls [`Self::write`] for each record and writes
    /// nothing between records or at the end, so a format that has no stream
    /// form of its own streams as it writes a whole result. A format
    /// overrides this to write records differently, such as `json`, which
    /// writes each record compact on its own line. The writer owns `sink` and
    /// is dropped when the stream ends.
    fn stream<'s>(&'s self, sink: Sink<'s>) -> Box<dyn RecordWriter + 's> {
        Box::new(Each::new(self, sink))
    }

    /// Write a failure that has no typed result, such as a parse error or an
    /// error the handler returned. The default lays it out as a document, as
    /// `pretty` writes it: `{bin}: {message}` with each cause after it as
    /// `: {cause}`, then one line per problem as
    /// `{input}:{line}:{column}: {field}: {message}`, leaving out what is
    /// unknown, then each tip of a parse error, then `Usage: {usage}`.
    ///
    /// # Errors
    ///
    /// Returns the error from writing it.
    fn write_error(&self, error: &ErrorBody, sink: &mut Sink<'_>) -> io::Result<()> {
        let text = sink.render(&error_document(error));
        sink.write_all(text.as_bytes())
    }
}

/// The failure as `pretty` shows it: the headline and the problems, then the
/// tips, then the usage line, each block apart as clap lays them out.
fn error_document(error: &ErrorBody) -> Document {
    let mut headline = Text::new()
        .error(error.bin.as_str())
        .then(": ")
        .then(error.message.as_str());
    for cause in &error.causes {
        headline = headline.then(": ").then(cause.as_str());
    }
    for problem in &error.problems {
        headline = headline.then("\n").then(problem_line(problem));
    }
    let mut document = Document::new().paragraph(headline);
    if let Some(parse) = &error.parse
        && !parse.tips.is_empty()
    {
        let tips = parse
            .tips
            .iter()
            .map(|tip| format!("  tip: {tip}"))
            .collect::<Vec<_>>();
        document = document.verbatim(tips.join("\n"));
    }
    if let Some(usage) = &error.usage {
        document = document.paragraph(format!("Usage: {usage}"));
    }
    document
}

/// One problem as `{input}:{line}:{column}: {field}: {message}`, leaving out
/// what is unknown.
fn problem_line(problem: &InputProblem) -> String {
    let mut line = problem.input.clone();
    if let Some(number) = problem.line {
        line = format!("{line}:{number}");
        if let Some(column) = problem.column {
            line = format!("{line}:{column}");
        }
    }
    match problem.field.as_deref() {
        Some(field) => format!("{line}: {field}: {}", problem.message),
        None => format!("{line}: {}", problem.message),
    }
}

#[cfg(test)]
mod tests {
    use crate::color::ColorMode;
    use crate::format::{Format, PRETTY, Sink};
    use crate::model::{ErrorBody, InputProblem, ParseFailure};
    use crate::render::RenderOptions;
    use crate::view::JsonLayout;

    fn pretty(error: &ErrorBody) -> String {
        let mut written = Vec::new();
        let mut sink = Sink::new(
            &mut written,
            ctl_term::Stream::Stderr,
            &RenderOptions::new(ColorMode::Never),
            JsonLayout::Compact,
            true,
        );
        PRETTY.write_error(error, &mut sink).unwrap();
        String::from_utf8(written).unwrap()
    }

    #[test]
    fn a_cause_follows_the_message_on_the_same_line() {
        let error = ErrorBody::new("toy", "reading config").causes(["no such file", "denied"]);
        assert_eq!(
            pretty(&error),
            "toy: reading config: no such file: denied\n"
        );
    }

    #[test]
    fn a_problem_is_a_line_that_leaves_out_what_is_unknown() {
        let error = ErrorBody::new("toy", "app.yaml has 3 problems").problems([
            InputProblem::new("app.yaml", "must name a label")
                .line(2)
                .column(9)
                .field("labels"),
            InputProblem::new("app.yaml", "not a mapping").line(3),
            InputProblem::new("app.yaml", "names no service"),
        ]);
        assert_eq!(
            pretty(&error),
            "toy: app.yaml has 3 problems\n\
             app.yaml:2:9: labels: must name a label\n\
             app.yaml:3: not a mapping\n\
             app.yaml: names no service\n"
        );
    }

    #[test]
    fn tips_and_the_usage_line_follow_as_clap_lays_them_out() {
        let error = ErrorBody::new("toy", "unexpected argument '--bogus' found")
            .usage("toy show [OPTIONS] <ID>")
            .parse(
                ParseFailure::new("unknown_argument")
                    .tips(["to pass '--bogus' as a value, use '-- --bogus'"]),
            );
        assert_eq!(
            pretty(&error),
            "toy: unexpected argument '--bogus' found\n\
             \n\
             \x20 tip: to pass '--bogus' as a value, use '-- --bogus'\n\
             \n\
             Usage: toy show [OPTIONS] <ID>\n"
        );
    }
}
