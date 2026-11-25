use std::io::{self, Write as _};

use serde::Serialize as _;
use serde_json::Serializer;
use serde_json::ser::{CompactFormatter, Formatter, PrettyFormatter};

use crate::format::{Format, FormatName, JSON_ABOUT, Output, Sink};
use crate::model::{Envelope, ErrorBody};
use crate::view::JsonLayout;

/// The `json` format: the result serialized as one JSON document.
///
/// It is a machine format, so it never carries ANSI. The document is in the
/// sink's layout and ends in a newline. A failure is an [`Envelope`] with the
/// [`ErrorBody`], so a reader that parses stderr gets the same shape for every
/// failure. Its shorthand is `--json`.
#[derive(Clone, Copy, Debug, Default)]
pub struct Json;

/// The built-in `json` format, ready to register or to give
/// [`View::new`](crate::view::View::new).
pub static JSON: Json = Json;

/// A `serde_json` formatter that lays out as the sink says, chosen at run time,
/// so one `serde_json::Serializer` type serves both layouts.
///
/// The pretty arm is two-space indentation, as `serde_json::to_writer_pretty`
/// writes. Both arms agree on every method this does not forward.
struct Layout(Option<PrettyFormatter<'static>>);

/// Each method that `PrettyFormatter` overrides goes to it, or to the compact
/// default when the layout is compact.
macro_rules! forward {
    ($($method:ident($($arg:ident: $ty:ty),*);)*) => {
        $(
            fn $method<W: ?Sized + io::Write>(
                &mut self,
                writer: &mut W,
                $($arg: $ty),*
            ) -> io::Result<()> {
                match &mut self.0 {
                    Some(pretty) => pretty.$method(writer, $($arg),*),
                    None => CompactFormatter.$method(writer, $($arg),*),
                }
            }
        )*
    };
}

impl Formatter for Layout {
    forward! {
        begin_array();
        end_array();
        begin_array_value(first: bool);
        end_array_value();
        begin_object();
        end_object();
        begin_object_key(first: bool);
        begin_object_value();
        end_object_value();
    }
}

/// One JSON document in the sink's layout, ending in a newline. A success and
/// an error envelope share it, so both follow the same layout.
fn document(value: &dyn erased_serde::Serialize, sink: &mut Sink<'_>) -> io::Result<()> {
    let pretty = sink.json_layout() == JsonLayout::Pretty;
    let layout = Layout(pretty.then(PrettyFormatter::new));
    value.serialize(&mut Serializer::with_formatter(&mut *sink, layout))?;
    sink.write_all(b"\n")
}

impl Format for Json {
    fn name(&self) -> FormatName {
        FormatName::JSON
    }

    fn about(&self) -> &'static str {
        JSON_ABOUT
    }

    fn shorthand(&self) -> Option<&'static str> {
        Some("json")
    }

    fn write(&self, output: &Output<'_>, sink: &mut Sink<'_>) -> io::Result<()> {
        document(output.data(), sink)
    }

    fn write_error(&self, error: &ErrorBody, sink: &mut Sink<'_>) -> io::Result<()> {
        let envelope = Envelope::<()>::err(error.clone());
        document(&envelope, sink)
    }
}

#[cfg(test)]
mod tests {
    use super::JSON;
    use crate::color::ColorMode;
    use crate::format::{Format, Sink};
    use crate::model::ErrorBody;
    use crate::render::RenderOptions;
    use crate::view::JsonLayout;

    fn error_text(layout: JsonLayout) -> String {
        let mut written = Vec::new();
        let mut sink = Sink::new(
            &mut written,
            ctl_term::Stream::Stderr,
            RenderOptions::new(ColorMode::Never),
            layout,
            true,
        );
        JSON.write_error(&ErrorBody::new("toy", "failed"), &mut sink)
            .unwrap();
        String::from_utf8(written).unwrap()
    }

    #[test]
    fn error_envelopes_follow_the_json_layout() {
        assert!(error_text(JsonLayout::Pretty).contains("\n  "));
        let compact = error_text(JsonLayout::Compact);
        assert_eq!(compact.lines().count(), 1, "{compact}");
    }
}
