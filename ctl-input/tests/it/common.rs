//! Helpers the integration tests share.

use std::borrow::Cow;
use std::fmt::Debug;

use ctl_input::Limits;
use ctl_input::de::deserialize;
use ctl_input::format::{Format, Reading, Refusal, Resolved};
use ctl_input::tree::{Node, NodeKind};
use serde::de::DeserializeOwned;

/// `text` read strictly under the default limits.
pub(crate) fn read(format: &dyn Format, text: &str) -> Node {
    format
        .read(text, &Limits::NONE, Reading::Strict)
        .unwrap_or_else(|refusal| panic!("{refusal}"))
}

/// A string borrowed from the scalar it was read from.
pub(crate) fn borrowed(text: &str) -> Resolved<'_> {
    Resolved::Str(Cow::Borrowed(text))
}

/// The bytes of `text` that `node` spans.
pub(crate) fn source<'a>(text: &'a str, node: &Node) -> &'a str {
    &text[node.span()]
}

/// The entries of a mapping node.
pub(crate) fn entries(node: &Node) -> &[(Node, Node)] {
    let NodeKind::Mapping(entries) = node.kind() else {
        panic!("{:?} is not a mapping", node.kind());
    };
    entries
}

/// The items of a sequence node.
pub(crate) fn items(node: &Node) -> &[Node] {
    let NodeKind::Sequence(items) = node.kind() else {
        panic!("{:?} is not a sequence", node.kind());
    };
    items
}

/// The value of a scalar node.
pub(crate) fn value(node: &Node) -> &str {
    let NodeKind::Scalar(scalar) = node.kind() else {
        panic!("{:?} is not a scalar", node.kind());
    };
    scalar.value()
}

/// The first value of `mapping` written under `key`.
pub(crate) fn get<'a>(mapping: &'a Node, key: &str) -> &'a Node {
    entries(mapping)
        .iter()
        .find(|(name, _)| value(name) == key)
        .map_or_else(|| panic!("no key {key}"), |(_, found)| found)
}

/// `text` read strictly and then into a `T`.
pub(crate) fn typed<T: DeserializeOwned>(
    format: &'static dyn Format,
    text: &str,
) -> Result<T, Refusal> {
    let tree = format.read(text, &Limits::NONE, Reading::Strict)?;
    deserialize(&tree, format)
}

/// What reading `text` into a `T` refused.
pub(crate) fn refused<T: DeserializeOwned + Debug>(
    format: &'static dyn Format,
    text: &str,
) -> Refusal {
    match typed::<T>(format, text) {
        Ok(value) => panic!("read {value:?} from {text:?}"),
        Err(refusal) => refusal,
    }
}

/// The bytes of `text` a refusal sits at.
pub(crate) fn at<'a>(text: &'a str, refusal: &Refusal) -> &'a str {
    let span = refusal
        .span()
        .unwrap_or_else(|| panic!("{refusal} has no span"));
    &text[span]
}
