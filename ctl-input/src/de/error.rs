//! The error a read of the tree raises, and where it sits.

use std::ops::Range;
use std::{error, fmt};

use serde::de::{self, Expected, Unexpected};

use crate::format::{Fault, Refusal};
use crate::messages::messages;
use crate::tree::Node;

/// Where an error sits: the bytes, and for a node an alias replayed, where its
/// value is defined.
#[derive(Clone, Debug)]
pub(super) struct Place {
    span: Range<usize>,
    defined_at: Option<Range<usize>>,
}

impl Place {
    /// The start of the text, where a fault of the whole root sits.
    pub(super) fn start() -> Self {
        Self {
            span: 0..0,
            defined_at: None,
        }
    }

    /// This place, naming where `mapping` is defined when an alias replayed
    /// it.
    pub(super) fn defining(self, mapping: &Node) -> Self {
        match mapping.used_at() {
            Some(_) => Self {
                defined_at: Some(mapping.span()),
                ..self
            },
            None => self,
        }
    }

    /// Where a fault of `node` sits: its own bytes, or the alias that
    /// replayed it, with the bytes it is defined at.
    pub(super) fn of(node: &Node) -> Self {
        match node.used_at() {
            Some(alias) => Self {
                span: alias,
                defined_at: Some(node.span()),
            },
            None => Self {
                span: node.span(),
                defined_at: None,
            },
        }
    }
}

/// A fault with the place it was found, once a node has claimed it.
#[derive(Debug)]
pub(super) struct Error {
    fault: Fault,
    place: Option<Place>,
}

impl Error {
    fn new(fault: Fault) -> Self {
        Self { fault, place: None }
    }

    pub(super) fn fault(&self) -> &Fault {
        &self.fault
    }

    pub(super) fn is_placed(&self) -> bool {
        self.place.is_some()
    }

    /// This error at `place`, unless an inner node already claimed it.
    pub(super) fn at(mut self, place: impl FnOnce() -> Place) -> Self {
        if self.place.is_none() {
            self.place = Some(place());
        }
        self
    }

    /// A node of another kind than the one the type needs.
    pub(super) fn expected_kind(expected: &str, found: String) -> Self {
        Self::new(Fault::ExpectedKind {
            expected: expected.to_owned(),
            found,
        })
    }

    /// A scalar the format cannot read as the type the field needs.
    pub(super) fn invalid(detail: String) -> Self {
        Self::new(Fault::InvalidValue { detail })
    }

    pub(super) fn into_refusal(self) -> Refusal {
        let (span, defined_at) = match self.place {
            Some(Place { span, defined_at }) => (Some(span), defined_at),
            None => (None, None),
        };
        Refusal::new(self.fault, span, defined_at)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&messages().message(&self.fault))
    }
}

impl error::Error for Error {}

impl de::Error for Error {
    fn custom<T: fmt::Display>(message: T) -> Self {
        Self::invalid(message.to_string())
    }

    fn invalid_type(unexpected: Unexpected<'_>, expected: &dyn Expected) -> Self {
        Self::new(Fault::ExpectedKind {
            expected: expected.to_string(),
            found: unexpected.to_string(),
        })
    }

    fn invalid_value(unexpected: Unexpected<'_>, expected: &dyn Expected) -> Self {
        Self::invalid(format!("expected {expected}, found {unexpected}"))
    }

    fn invalid_length(length: usize, expected: &dyn Expected) -> Self {
        Self::invalid(format!("expected {expected}, found {length} items"))
    }

    fn unknown_field(field: &str, expected: &'static [&'static str]) -> Self {
        Self::new(Fault::UnknownField {
            field: field.to_owned(),
            expected: expected.to_vec(),
        })
    }

    fn unknown_variant(variant: &str, expected: &'static [&'static str]) -> Self {
        Self::new(Fault::UnknownVariant {
            variant: variant.to_owned(),
            expected: expected.to_vec(),
        })
    }

    fn missing_field(field: &'static str) -> Self {
        Self::new(Fault::MissingField { field })
    }

    fn duplicate_field(field: &'static str) -> Self {
        Self::new(Fault::DuplicateKey {
            key: field.to_owned(),
        })
    }
}
