use std::io::{self, Write as _};

use serde::Serialize;

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

/// One JSON document in the sink's layout, ending in a newline. A success and
/// an error envelope share it, so both follow the same layout.
fn document<T: Serialize + ?Sized>(value: &T, sink: &mut Sink<'_>) -> io::Result<()> {
    if sink.json_layout() == JsonLayout::Pretty {
        serde_json::to_writer_pretty(&mut *sink, value)?;
    } else {
        serde_json::to_writer(&mut *sink, value)?;
    }
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
        document(&Envelope::<()>::err(error.clone()), sink)
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
