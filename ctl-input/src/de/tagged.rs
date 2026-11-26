//! A value and the tag written on its node.

use std::fmt;
use std::marker::PhantomData;

use serde::de::value::StringDeserializer;
use serde::de::{self, DeserializeSeed, Deserializer, IntoDeserializer, SeqAccess, Visitor};
use serde::{Deserialize, forward_to_deserialize_any};

use super::error::Error;
use super::node::NodeDe;

/// The name a [`Tagged`] asks its newtype for, which only this crate's
/// Deserializer answers.
pub(super) const NAME: &str = "ctl_input::de::Tagged";

/// A value and the tag written on its node, such as `!override` in
/// `code_span_size: !override 0`: `Tagged(0, Some("!override".into()))`.
///
/// The tag is spelled as [`Tag::as_str`](crate::tree::Tag::as_str) spells it,
/// and the value is read from the same node without its tag. A node with no
/// tag reads as `Tagged(value, None)`, and so does a value any other
/// Deserializer reads.
///
/// A tag inside a value serde buffers, such as an untagged enum or a
/// flattened field, is not passed on: the buffered value reads as
/// `Tagged(value, None)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tagged<T>(
    /// The value, read without its tag.
    pub T,
    /// The tag written on the node, if any.
    pub Option<String>,
);

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Tagged<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_newtype_struct(NAME, TaggedVisitor(PhantomData))
    }
}

struct TaggedVisitor<T>(PhantomData<T>);

impl<'de, T: Deserialize<'de>> Visitor<'de> for TaggedVisitor<T> {
    type Value = Tagged<T>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a value")
    }

    fn visit_newtype_struct<D: Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Tagged<T>, D::Error> {
        T::deserialize(deserializer).map(|value| Tagged(value, None))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Tagged<T>, A::Error> {
        let value = seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(0, &self))?;
        let tag = seq.next_element::<Option<String>>()?.flatten();
        Ok(Tagged(value, tag))
    }
}

/// The value of a tagged node, then its tag, as the two items a [`Tagged`]
/// reads.
pub(super) struct Items<'de> {
    value: Option<NodeDe<'de>>,
    tag: Option<String>,
    tag_unread: bool,
}

impl<'de> Items<'de> {
    pub(super) fn new(value: NodeDe<'de>, tag: Option<String>) -> Self {
        Self {
            value: Some(value),
            tag,
            tag_unread: true,
        }
    }
}

impl<'de> SeqAccess<'de> for Items<'de> {
    type Error = Error;

    fn next_element_seed<S: DeserializeSeed<'de>>(
        &mut self,
        seed: S,
    ) -> Result<Option<S::Value>, Error> {
        if let Some(value) = self.value.take() {
            return value.seeded(seed).map(Some);
        }
        if self.tag_unread {
            self.tag_unread = false;
            return seed.deserialize(TagValue(self.tag.take())).map(Some);
        }
        Ok(None)
    }

    fn size_hint(&self) -> Option<usize> {
        Some(usize::from(self.value.is_some()) + usize::from(self.tag_unread))
    }
}

/// A tag as the `Option<String>` a [`Tagged`] reads it as.
struct TagValue(Option<String>);

impl<'de> Deserializer<'de> for TagValue {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_option(visitor)
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.0 {
            Some(tag) => {
                let tag: StringDeserializer<Error> = tag.into_deserializer();
                visitor.visit_some(tag)
            }
            None => visitor.visit_none(),
        }
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes byte_buf
        unit unit_struct newtype_struct seq tuple tuple_struct map struct enum identifier
        ignored_any
    }
}
