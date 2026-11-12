//! The YAML half of [`Input::top_level`]. serde-saphyr reads the keys and the
//! types under the limits, so anchors, aliases and tags are resolved by the
//! parser, and yamled gives each value's span and style from the document
//! that [`Input::place`] keeps.

use std::collections::HashMap;
use std::fmt;

use serde::de::{self, Deserialize, Deserializer, IgnoredAny, MapAccess, SeqAccess, Visitor};
use serde_saphyr::{Location, Spanned};
use yamled::Style;

use super::entries::{Entry, Written};
use super::limits::Raise;
use super::{Input, InputError, limits, yaml_text};

/// What the parser reads a value or a key as: a string, a number, a boolean or
/// a null, or a sequence or a mapping.
enum Typed {
    Text(String),
    Scalar,
    Collection,
}

impl<'de> Deserialize<'de> for Typed {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(TypedVisitor)
    }
}

struct TypedVisitor;

impl<'de> Visitor<'de> for TypedVisitor {
    type Value = Typed;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a YAML value")
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Typed, E> {
        Ok(Typed::Text(value.to_owned()))
    }

    fn visit_string<E: de::Error>(self, value: String) -> Result<Typed, E> {
        Ok(Typed::Text(value))
    }

    fn visit_bool<E: de::Error>(self, _: bool) -> Result<Typed, E> {
        Ok(Typed::Scalar)
    }

    fn visit_i64<E: de::Error>(self, _: i64) -> Result<Typed, E> {
        Ok(Typed::Scalar)
    }

    fn visit_u64<E: de::Error>(self, _: u64) -> Result<Typed, E> {
        Ok(Typed::Scalar)
    }

    fn visit_f64<E: de::Error>(self, _: f64) -> Result<Typed, E> {
        Ok(Typed::Scalar)
    }

    fn visit_unit<E: de::Error>(self) -> Result<Typed, E> {
        Ok(Typed::Scalar)
    }

    fn visit_none<E: de::Error>(self) -> Result<Typed, E> {
        Ok(Typed::Scalar)
    }

    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<Typed, D::Error> {
        deserializer.deserialize_any(self)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Typed, A::Error> {
        while seq.next_element::<IgnoredAny>()?.is_some() {}
        Ok(Typed::Collection)
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Typed, A::Error> {
        while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
        Ok(Typed::Collection)
    }
}

/// The entries of the root mapping with a string or a scalar key, as the
/// parser read them, each with the place of its key, or none when the root is
/// no mapping. A scalar key that is no string has no text yet: it is the
/// text as written.
struct Root(Vec<(Option<String>, Location, Typed)>);

impl<'de> Deserialize<'de> for Root {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(RootVisitor)
    }
}

struct RootVisitor;

impl<'de> Visitor<'de> for RootVisitor {
    type Value = Root;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a YAML document")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Root, A::Error> {
        let mut entries = Vec::new();
        while let Some((key, value)) = map.next_entry::<Spanned<Typed>, Typed>()? {
            match key.value {
                Typed::Text(text) => entries.push((Some(text), key.referenced, value)),
                Typed::Scalar => entries.push((None, key.referenced, value)),
                Typed::Collection => {}
            }
        }
        Ok(Root(entries))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Root, A::Error> {
        while seq.next_element::<IgnoredAny>()?.is_some() {}
        Ok(Root(Vec::new()))
    }

    fn visit_str<E: de::Error>(self, _: &str) -> Result<Root, E> {
        Ok(Root(Vec::new()))
    }

    fn visit_bool<E: de::Error>(self, _: bool) -> Result<Root, E> {
        Ok(Root(Vec::new()))
    }

    fn visit_i64<E: de::Error>(self, _: i64) -> Result<Root, E> {
        Ok(Root(Vec::new()))
    }

    fn visit_u64<E: de::Error>(self, _: u64) -> Result<Root, E> {
        Ok(Root(Vec::new()))
    }

    fn visit_f64<E: de::Error>(self, _: f64) -> Result<Root, E> {
        Ok(Root(Vec::new()))
    }

    fn visit_unit<E: de::Error>(self) -> Result<Root, E> {
        Ok(Root(Vec::new()))
    }

    fn visit_none<E: de::Error>(self) -> Result<Root, E> {
        Ok(Root(Vec::new()))
    }
}

/// [`Input::top_level`] for a YAML text.
pub(super) fn entries(input: &Input) -> Result<Vec<Entry>, InputError> {
    let limits = limits::effective(&input.limits);
    input.fits(&limits, Raise::FOR_TEXT)?;
    let Root(typed) = yaml_text::parse_types::<Root>(input, &limits.budget(), Raise::FOR_TEXT)?;
    let Some(root) = yaml_text::document(input, &limits)
        .and_then(|document| document.node(&yamled::Path::root()))
    else {
        return Ok(Vec::new());
    };
    // Each entry pairs with the yamled entry whose key sits where the parser
    // read it. Both start a key at its scalar, so an anchor or a tag before it
    // moves neither.
    // A key written twice in different types, such as `1` and `'1'`, cannot
    // take the other's span, a key written twice in one type has two places,
    // and a key yamled leaves out, such as an alias, shifts no pair after it.
    let by_place: HashMap<(usize, usize), _> = root
        .entries()
        .filter_map(|(_, node)| node.key().map(|at| ((at.line, at.column), node)))
        .collect();
    let mut found = Vec::with_capacity(typed.len());
    for (key, at, value) in typed {
        let place = (
            usize::try_from(at.line()).unwrap_or_default(),
            usize::try_from(at.column()).unwrap_or_default(),
        );
        let Some(node) = by_place.get(&place) else {
            continue;
        };
        let at = node.value();
        // A key that is a number, a boolean or a null is its text as written,
        // and a null written as nothing has no text to list.
        let Some(key) = key.or_else(|| {
            let written = node.key()?;
            input
                .text()
                .get(written.start..written.end)
                .filter(|text| !text.is_empty())
                .map(str::to_owned)
        }) else {
            continue;
        };
        found.push(Entry {
            key,
            value: at.start..at.end,
            written: written_as(node.style()),
            string: match value {
                Typed::Text(text) => Some(text),
                Typed::Scalar | Typed::Collection => None,
            },
        });
    }
    Ok(found)
}

/// How yamled's `style` is written.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "Style is non-exhaustive: a later yamled style is a collection or unknown"
)]
const fn written_as(style: Style) -> Written {
    match style {
        Style::Plain => Written::Plain,
        Style::SingleQuoted => Written::SingleQuoted,
        Style::DoubleQuoted => Written::DoubleQuoted,
        Style::Literal => Written::Literal,
        Style::Folded => Written::Folded,
        Style::Alias => Written::Alias,
        _ => Written::Collection,
    }
}
