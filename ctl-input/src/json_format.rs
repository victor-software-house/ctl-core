//! The JSON [`Format`]: the own reader's tree, each scalar carrying the type
//! its syntax gave it, and each scalar resolves by that type.

use std::borrow::Cow;

use super::exceeded::Exceeded;
use super::format::{Format, Mismatch, Raise, Reading, Refusal, Resolved, Wanted};
use super::json;
use super::limits::Limits;
use super::tree::{Node, Scalar, ScalarSyntax, Tag};

/// JSON by RFC 8259, with no option that loosens it. Its extension is `json`.
///
/// A strict reading refuses a key written twice in one object, comparing keys
/// after their escapes are decoded, and a listing reading keeps both entries.
/// A number keeps its source text, and a field decides whether it fits.
#[derive(Clone, Copy, Debug)]
pub struct Json;

/// The JSON format, for [`Formats::add`](crate::Formats::add).
pub static JSON: Json = Json;

impl Format for Json {
    fn name(&self) -> &'static str {
        "json"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["json"]
    }

    /// Checks `max_reader_input_bytes` before the first token, `max_depth`
    /// when an array or an object opens, and `max_nodes` when a value is
    /// added.
    fn read(&self, text: &str, limits: &Limits, reading: Reading) -> Result<Node, Refusal> {
        let bounds = limits.bounds();
        if text.len() > bounds.input_bytes {
            let exceeded = Exceeded::input_bytes(bounds.input_bytes, Raise::FOR_TEXT);
            return Err(Refusal::new(exceeded.fault(), None, None));
        }
        json::read(text, bounds, reading)
    }

    /// Resolves by the type the scalar's syntax gave it, and never by its
    /// text. A scalar that carries no syntax, such as one another format built,
    /// is text. A typeless read has no field to say what was expected, so a
    /// number it cannot hold is refused with its text and `out of range`.
    fn resolve<'a>(
        &self,
        scalar: &'a Scalar,
        _tag: Option<&Tag>,
        wanted: Wanted,
    ) -> Result<Resolved<'a>, Mismatch> {
        let text = scalar.value();
        let syntax = scalar.syntax().unwrap_or(ScalarSyntax::Text);
        let resolved = match (syntax, wanted) {
            (ScalarSyntax::Text, Wanted::Any | Wanted::Str) => {
                Some(Resolved::Str(Cow::Borrowed(text)))
            }
            (ScalarSyntax::Boolean, Wanted::Any | Wanted::Bool) => {
                Some(Resolved::Bool(text == "true"))
            }
            (ScalarSyntax::Integer, Wanted::Int) => integer(text),
            (ScalarSyntax::Integer, Wanted::Any) => integer(text).or_else(|| float(text)),
            (ScalarSyntax::Integer | ScalarSyntax::Float, Wanted::Any | Wanted::Float) => {
                float(text)
            }
            (ScalarSyntax::Null, Wanted::Any | Wanted::Null) => Some(Resolved::Null),
            _ => None,
        };
        resolved.ok_or_else(|| {
            let mismatch = Mismatch::new(wanted);
            if wanted == Wanted::Any
                && matches!(syntax, ScalarSyntax::Integer | ScalarSyntax::Float)
            {
                mismatch.with_detail(format!("number `{text}` is out of range"))
            } else {
                mismatch
            }
        })
    }
}

/// The integer a JSON integer's text spells: an `i128`, or a `u128` above it.
/// A text no `u128` holds is no integer.
fn integer(text: &str) -> Option<Resolved<'static>> {
    text.parse()
        .map(Resolved::Int)
        .or_else(|_| text.parse().map(Resolved::Uint))
        .ok()
}

/// The float a number's text spells. A value that overflows to infinity is
/// refused.
fn float(text: &str) -> Option<Resolved<'static>> {
    text.parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
        .map(Resolved::Float)
}
