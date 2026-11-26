//! What the Deserializer hands a visitor to walk a mapping, a sequence or an
//! enum.

use std::marker::PhantomData;
use std::slice;

use serde::de::{
    self, DeserializeSeed, EnumAccess, MapAccess, SeqAccess, Unexpected, VariantAccess, Visitor,
};

use super::error::Error;
use super::node::{NodeDe, Parent};
use crate::format::Format;
use crate::tree::Node;

/// The entries of a mapping, in the order written.
pub(super) struct Entries<'de> {
    rest: slice::Iter<'de, (Node, Node)>,
    format: &'static dyn Format,
    key: Option<&'de Node>,
    value: Option<&'de Node>,
}

impl<'de> Entries<'de> {
    pub(super) fn new(entries: &'de [(Node, Node)], format: &'static dyn Format) -> Self {
        Self {
            rest: entries.iter(),
            format,
            key: None,
            value: None,
        }
    }

    /// The key read last, which a field written twice is placed on.
    pub(super) fn last_key(&self) -> Option<&'de Node> {
        self.key
    }
}

impl<'de> MapAccess<'de> for Entries<'de> {
    type Error = Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Error> {
        let Some((key, value)) = self.rest.next() else {
            return Ok(None);
        };
        self.key = Some(key);
        self.value = Some(value);
        NodeDe::new(key, self.format, Parent::Item)
            .seeded(seed)
            .map(Some)
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value, Error> {
        let (Some(key), Some(value)) = (self.key, self.value.take()) else {
            return Err(de::Error::custom("a value was read before its key"));
        };
        NodeDe::new(value, self.format, Parent::Key(key)).seeded(seed)
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.rest.len())
    }
}

/// The items of a sequence, in the order written.
pub(super) struct Items<'de> {
    rest: slice::Iter<'de, Node>,
    format: &'static dyn Format,
    total: usize,
    length: Option<usize>,
}

impl<'de> Items<'de> {
    /// The items of a sequence, which a tuple of `length` items reads, or
    /// else any number.
    pub(super) fn new(
        items: &'de [Node],
        format: &'static dyn Format,
        length: Option<usize>,
    ) -> Self {
        Self {
            rest: items.iter(),
            format,
            total: items.len(),
            length,
        }
    }

    /// Refuses a sequence the visitor left items of, as a tuple shorter than
    /// the sequence does.
    pub(super) fn finish(&self) -> Result<(), Error> {
        if self.rest.as_slice().is_empty() {
            return Ok(());
        }
        Err(match self.length {
            Some(length) => {
                de::Error::invalid_length(self.total, &format!("{length} items").as_str())
            }
            None => de::Error::invalid_length(self.total, &"a shorter sequence"),
        })
    }
}

impl<'de> SeqAccess<'de> for Items<'de> {
    type Error = Error;

    fn next_element_seed<T: DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, Error> {
        match self.rest.next() {
            Some(item) => NodeDe::new(item, self.format, Parent::Item)
                .seeded(seed)
                .map(Some),
            None => Ok(None),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.rest.len())
    }
}

/// The variant an enum is written as: a name, or a name with its content.
pub(super) struct Variant<'de> {
    name: NodeDe<'de>,
    content: Option<NodeDe<'de>>,
}

impl<'de> Variant<'de> {
    /// A variant written as its name alone.
    pub(super) fn unit(name: NodeDe<'de>) -> Self {
        Self {
            name,
            content: None,
        }
    }

    /// A variant written as a one-entry mapping: its name, then its content.
    pub(super) fn new(name: NodeDe<'de>, content: NodeDe<'de>) -> Self {
        Self {
            name,
            content: Some(content),
        }
    }
}

impl<'de> EnumAccess<'de> for Variant<'de> {
    type Error = Error;
    type Variant = Content<'de>;

    fn variant_seed<V: DeserializeSeed<'de>>(
        self,
        seed: V,
    ) -> Result<(V::Value, Content<'de>), Error> {
        let name = self.name.seeded(seed)?;
        Ok((name, Content(self.content)))
    }
}

/// What a variant carries, if anything.
pub(super) struct Content<'de>(Option<NodeDe<'de>>);

impl<'de> VariantAccess<'de> for Content<'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Error> {
        match self.0 {
            Some(content) => content.seeded(PhantomData),
            None => Ok(()),
        }
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value, Error> {
        match self.0 {
            Some(content) => content.seeded(seed),
            None => Err(de::Error::invalid_type(
                Unexpected::UnitVariant,
                &"newtype variant",
            )),
        }
    }

    fn tuple_variant<V: Visitor<'de>>(self, length: usize, visitor: V) -> Result<V::Value, Error> {
        match self.0 {
            Some(content) => de::Deserializer::deserialize_tuple(content, length, visitor),
            None => Err(de::Error::invalid_type(
                Unexpected::UnitVariant,
                &"tuple variant",
            )),
        }
    }

    fn struct_variant<V: Visitor<'de>>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        match self.0 {
            Some(content) => de::Deserializer::deserialize_struct(content, "", fields, visitor),
            None => Err(de::Error::invalid_type(
                Unexpected::UnitVariant,
                &"struct variant",
            )),
        }
    }
}
