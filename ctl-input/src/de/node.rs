//! The Deserializer over one node of the tree.

use std::borrow::Cow;

use serde::de::{self, DeserializeSeed, Deserializer, Unexpected, Visitor};

use super::access::{Entries, Items, Variant};
use super::error::{Error, Place};
use super::tagged;
use crate::format::{Fault, Format, Resolved, Wanted};
use crate::tree::{Node, NodeKind, Scalar, Tag};

/// What holds the node, which says where a missing field of it sits.
#[derive(Clone, Copy)]
pub(super) enum Parent<'de> {
    /// Nothing: the node is the root of the tree.
    Root,
    /// A sequence, or a key: a node with no key of its own.
    Item,
    /// The mapping entry whose value the node is, with the key's node.
    Key(&'de Node),
}

/// A node of the tree read as a serde value, by the rules of its format.
#[derive(Clone, Copy)]
pub(super) struct NodeDe<'de> {
    node: &'de Node,
    format: &'static dyn Format,
    parent: Parent<'de>,
    tagged: bool,
}

impl<'de> NodeDe<'de> {
    pub(super) fn new(node: &'de Node, format: &'static dyn Format, parent: Parent<'de>) -> Self {
        Self {
            node,
            format,
            parent,
            tagged: true,
        }
    }

    /// The same node read as if it had no tag.
    pub(super) fn untagged(self) -> Self {
        Self {
            tagged: false,
            ..self
        }
    }

    /// Where an error the node's own read raised sits.
    fn place(&self) -> Place {
        Place::of(self.node)
    }

    /// Where a missing field of the mapping this node is sits: on the key
    /// that holds it, naming where the mapping is defined when an alias
    /// replayed it, on the start of the text for the root, or on the node.
    fn place_of_missing(&self) -> Place {
        match self.parent {
            Parent::Root => Place::start(),
            Parent::Item => self.place(),
            Parent::Key(key) => Place::of(key).defining(self.node),
        }
    }

    fn tag(&self) -> Option<&'de Tag> {
        self.node.tag().filter(|_| self.tagged)
    }

    /// What the node is, for a fault that expected another kind.
    fn found(&self) -> String {
        match self.node.kind() {
            NodeKind::Scalar(_) => "a scalar".to_owned(),
            NodeKind::Sequence(_) => "a sequence".to_owned(),
            NodeKind::Mapping(_) => "a mapping".to_owned(),
        }
    }

    fn unexpected(&self) -> Unexpected<'static> {
        match self.node.kind() {
            NodeKind::Scalar(_) => Unexpected::Other("a scalar"),
            NodeKind::Sequence(_) => Unexpected::Other("a sequence"),
            NodeKind::Mapping(_) => Unexpected::Other("a mapping"),
        }
    }

    /// `read` on this node, with an error it raises placed at the node unless
    /// a node inside already placed it. A missing field is raised by the
    /// mapping's own `Deserialize`, after it read every entry, so it goes where
    /// a missing field of this node sits.
    fn located<T>(self, read: impl FnOnce(Self) -> Result<T, Error>) -> Result<T, Error> {
        read(self).map_err(|error| {
            let missing = matches!(error.fault(), Fault::MissingField { .. });
            error.at(|| {
                if missing {
                    self.place_of_missing()
                } else {
                    self.place()
                }
            })
        })
    }

    /// `seed` read from this node, with an error its own `Deserialize` raises
    /// after the node has given its value, such as a refusal of a `try_from`,
    /// placed at the node.
    pub(super) fn seeded<S: DeserializeSeed<'de>>(self, seed: S) -> Result<S::Value, Error> {
        self.located(|node| seed.deserialize(node))
    }

    fn resolve(&self, scalar: &'de Scalar, wanted: Wanted) -> Result<Resolved<'de>, Error> {
        self.format
            .resolve(scalar, self.tag(), wanted)
            .map_err(|mismatch| Error::invalid(mismatch.to_string()))
    }

    /// The scalar read as `wanted` and handed to `visitor`, which refuses it
    /// with its own words when it is the wrong type.
    fn scalar_as<V: Visitor<'de>>(self, wanted: Wanted, visitor: V) -> Result<V::Value, Error> {
        match self.node.kind() {
            NodeKind::Scalar(scalar) => visit(self.resolve(scalar, wanted)?, visitor),
            NodeKind::Sequence(_) | NodeKind::Mapping(_) => {
                Err(de::Error::invalid_type(self.unexpected(), &visitor))
            }
        }
    }

    fn any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.node.kind() {
            NodeKind::Mapping(_) => self.mapping(visitor),
            NodeKind::Sequence(_) => self.sequence(visitor),
            NodeKind::Scalar(_) => self.scalar_as(Wanted::Any, visitor),
        }
    }

    fn bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.node.kind() {
            NodeKind::Scalar(scalar) => match self.resolve(scalar, Wanted::Str)? {
                Resolved::Str(Cow::Borrowed(text)) => visitor.visit_borrowed_bytes(text.as_bytes()),
                Resolved::Str(Cow::Owned(text)) => visitor.visit_byte_buf(text.into_bytes()),
                resolved @ (Resolved::Null
                | Resolved::Bool(_)
                | Resolved::Int(_)
                | Resolved::Uint(_)
                | Resolved::Float(_)) => visit(resolved, visitor),
            },
            NodeKind::Sequence(_) | NodeKind::Mapping(_) => {
                Err(de::Error::invalid_type(self.unexpected(), &visitor))
            }
        }
    }

    fn option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        if let NodeKind::Scalar(scalar) = self.node.kind()
            && self
                .format
                .resolve(scalar, self.tag(), Wanted::Null)
                .is_ok()
        {
            return visitor.visit_none();
        }
        visitor.visit_some(self)
    }

    /// The entries of a mapping, as serde reads a map or a struct, or an
    /// error that expects a mapping.
    fn mapping<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        let NodeKind::Mapping(entries) = self.node.kind() else {
            return Err(Error::expected_kind("a mapping", self.found()));
        };
        let mut access = Entries::new(entries, self.format);
        visitor
            .visit_map(&mut access)
            .map_err(|error| self.claim(error, &access))
    }

    /// Places a field written twice, which the visitor raised on the whole
    /// mapping, on the key that repeats it.
    fn claim(&self, error: Error, access: &Entries<'de>) -> Error {
        if error.is_placed() {
            return error;
        }
        let repeated = matches!(error.fault(), Fault::DuplicateKey { .. });
        match access.last_key() {
            Some(key) if repeated => error.at(|| Place::of(key)),
            _ => error,
        }
    }

    fn sequence<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        let NodeKind::Sequence(items) = self.node.kind() else {
            return Err(Error::expected_kind("a sequence", self.found()));
        };
        let mut access = Items::new(items, self.format);
        let value = visitor.visit_seq(&mut access)?;
        access.finish()?;
        Ok(value)
    }

    fn enumeration<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.node.kind() {
            NodeKind::Scalar(_) => visitor.visit_enum(Variant::unit(self)),
            NodeKind::Mapping(entries) => match entries.as_slice() {
                [(key, value)] => {
                    let key_de = Self::new(key, self.format, Parent::Item);
                    let value_de = Self::new(value, self.format, Parent::Key(key));
                    visitor.visit_enum(Variant::new(key_de, value_de))
                }
                _ => Err(Error::expected_kind(
                    "a variant name or a mapping with one entry",
                    format!("a mapping with {} entries", entries.len()),
                )),
            },
            NodeKind::Sequence(_) => Err(Error::expected_kind(
                "a variant name or a mapping with one entry",
                self.found(),
            )),
        }
    }
}

/// What `visitor` is told of a scalar the format resolved.
fn visit<'de, V: Visitor<'de>>(resolved: Resolved<'de>, visitor: V) -> Result<V::Value, Error> {
    match resolved {
        Resolved::Null => visitor.visit_unit(),
        Resolved::Bool(value) => visitor.visit_bool(value),
        Resolved::Int(value) => match (i64::try_from(value), u64::try_from(value)) {
            (Ok(fits), _) => visitor.visit_i64(fits),
            (Err(_), Ok(fits)) => visitor.visit_u64(fits),
            (Err(_), Err(_)) => visitor.visit_i128(value),
        },
        Resolved::Uint(value) => match u64::try_from(value) {
            Ok(fits) => visitor.visit_u64(fits),
            Err(_) => visitor.visit_u128(value),
        },
        Resolved::Float(value) => visitor.visit_f64(value),
        Resolved::Str(Cow::Borrowed(text)) => visitor.visit_borrowed_str(text),
        Resolved::Str(Cow::Owned(text)) => visitor.visit_string(text),
    }
}

macro_rules! scalar {
    ($($method:ident => $wanted:ident),* $(,)?) => {$(
        fn $method<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
            self.located(|node| node.scalar_as(Wanted::$wanted, visitor))
        }
    )*};
}

impl<'de> Deserializer<'de> for NodeDe<'de> {
    type Error = Error;

    scalar! {
        deserialize_bool => Bool,
        deserialize_i8 => Int,
        deserialize_i16 => Int,
        deserialize_i32 => Int,
        deserialize_i64 => Int,
        deserialize_i128 => Int,
        deserialize_u8 => Int,
        deserialize_u16 => Int,
        deserialize_u32 => Int,
        deserialize_u64 => Int,
        deserialize_u128 => Int,
        deserialize_f32 => Float,
        deserialize_f64 => Float,
        deserialize_char => Str,
        deserialize_str => Str,
        deserialize_string => Str,
        deserialize_identifier => Str,
        deserialize_unit => Null,
    }

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.located(|node| node.any(visitor))
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.located(|node| node.bytes(visitor))
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.located(|node| node.bytes(visitor))
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.located(|node| node.option(visitor))
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.located(|node| {
            if name == tagged::NAME && node.tagged {
                let tag = node.node.tag().map(|tag| tag.as_str().to_owned());
                visitor.visit_seq(tagged::Items::new(node.untagged(), tag))
            } else {
                visitor.visit_newtype_struct(node)
            }
        })
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.located(|node| node.sequence(visitor))
    }

    fn deserialize_tuple<V: Visitor<'de>>(
        self,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.located(|node| node.mapping(visitor))
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.located(|node| node.enumeration(visitor))
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_unit()
    }
}
