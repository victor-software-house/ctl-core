//! Helpers the integration tests share.

use ctl_input::Limits;
use ctl_input::format::{Format, Reading};
use ctl_input::tree::{Node, NodeKind};

/// `text` read strictly under the default limits.
pub(crate) fn read(format: &dyn Format, text: &str) -> Node {
    format
        .read(text, &Limits::NONE, Reading::Strict)
        .unwrap_or_else(|refusal| panic!("{refusal}"))
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
