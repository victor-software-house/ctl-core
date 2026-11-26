//! Reads a typed value from the located tree a [format](crate::format::Format)
//! read.
//!
//! One serde Deserializer serves every format: it walks the [tree](crate::tree)
//! and asks the format to resolve each scalar for the type the field needs, so
//! each format keeps its own scalar rules. A problem comes back as a
//! [`Refusal`] with a [`Fault`](crate::format::Fault)
//! and the bytes it sits at; [`Messages`](crate::Messages) words the fault.
//!
//! [`Tagged`] reads a node's tag beside its value.
//!
//! [`Input`](crate::Input) does not read through this module yet: it still
//! reads with the `Format` enum of the crate root.

mod access;
#[cfg(all(test, feature = "yaml", feature = "toml"))]
mod differential;
mod error;
mod node;
mod tagged;

use std::marker::PhantomData;

use node::{NodeDe, Parent};
use serde::de::DeserializeOwned;
pub use tagged::Tagged;

use crate::format::{Format, Refusal};
use crate::tree::Node;

/// Reads a `T` from `node`, a tree `format` read, resolving each scalar by
/// `format`'s rules.
///
/// The mapping and sequence rules are the same for every format. A struct or
/// a map needs a mapping, and a sequence, a tuple or a tuple struct needs a
/// sequence. An enum is a scalar naming a unit variant, or a mapping of one
/// entry from the variant to its content. An `Option` is `None` for a scalar
/// that resolves as null and `Some` for any other node.
///
/// # Errors
///
/// A [`Refusal`] naming the fault and the bytes it sits at: the node the
/// fault arose in, or the alias that replayed it with
/// [`Refusal::defined_at`] naming where its value is written. A missing field
/// sits on the key whose value is the mapping that lacks it, or at the start
/// of the text for the root mapping.
///
/// ```
/// # #[cfg(feature = "yaml")]
/// # {
/// use ctl_input::Limits;
/// use ctl_input::de::deserialize;
/// use ctl_input::format::{Format, Reading, YAML};
/// use serde::Deserialize;
///
/// #[derive(Debug, Deserialize, PartialEq)]
/// struct Config {
///     port: u16,
/// }
///
/// let tree = YAML.read("port: '8080'\n", &Limits::NONE, Reading::Strict)?;
/// let config: Config = deserialize(&tree, &YAML)?;
/// assert_eq!(config, Config { port: 8080 });
/// # }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn deserialize<T: DeserializeOwned>(
    node: &Node,
    format: &'static dyn Format,
) -> Result<T, Refusal> {
    NodeDe::new(node, format, Parent::Root)
        .seeded(PhantomData::<T>)
        .map_err(error::Error::into_refusal)
}
