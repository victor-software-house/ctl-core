//! The JSON reader: one loop over the scanner's tokens that builds the located
//! tree, with an explicit stack of the arrays and objects still open.
//!
//! The loop checks `max_depth` when a container opens and `max_nodes` when a
//! node is added, so a refusal comes before the next allocation. It never
//! recurses, so the reader keeps no call stack while it reads. A deep tree
//! still recurses when it is dropped or deserialized, which `max_depth` bounds.

mod scan;

use std::collections::HashSet;
use std::ops::Range;

use scan::{Failure, Kind, Scanner, Token};

use super::exceeded::Exceeded;
use super::format::{Fault, Raise, Reading, Refusal};
use super::limits::Bounds;
use super::tree::{Node, Scalar, ScalarStyle};

/// Reads `text` into the tree it writes, under `bounds`. A strict `reading`
/// refuses a key written twice in one object.
pub(super) fn read(text: &str, bounds: Bounds, reading: Reading) -> Result<Node, Refusal> {
    Reader {
        scanner: Scanner::new(text),
        bounds,
        strict: reading == Reading::Strict,
        stack: Vec::new(),
        nodes: 0,
    }
    .run()
}

/// What the loop needs next.
#[derive(Clone, Copy, Debug)]
enum Want {
    /// A value.
    Value,
    /// A value, or the end of the array that has just opened.
    ValueOrClose,
    /// A key, or the end of the object that has just opened.
    KeyOrClose,
    /// A key, after a comma.
    Key,
    /// The colon after a key.
    Colon,
    /// A comma, or the end of the open container.
    Next,
}

/// An array or an object, still open.
struct Frame {
    start: usize,
    open: Open,
}

enum Open {
    Array(Vec<Node>),
    Object(Object),
}

/// The entries of an object read so far, and the key whose value is next.
struct Object {
    entries: Vec<(Node, Node)>,
    key: Option<Node>,
    /// The decoded keys of `entries`, kept only by a strict reading.
    seen: HashSet<String>,
}

/// What a token did to the loop.
enum Step {
    Want(Want),
    Done(Node),
}

struct Reader<'a> {
    scanner: Scanner<'a>,
    bounds: Bounds,
    strict: bool,
    stack: Vec<Frame>,
    nodes: usize,
}

impl Reader<'_> {
    fn run(mut self) -> Result<Node, Refusal> {
        let mut want = Want::Value;
        loop {
            let token = self.scanner.next().map_err(syntax)?;
            match self.step(want, token)? {
                Step::Want(next) => want = next,
                Step::Done(root) => {
                    return match self.scanner.trailing() {
                        Some(span) => Err(syntax(Failure {
                            span,
                            detail: "text after the value".to_owned(),
                        })),
                        None => Ok(root),
                    };
                }
            }
        }
    }

    fn step(&mut self, want: Want, token: Token) -> Result<Step, Refusal> {
        let Token { kind, span } = token;
        match (want, kind) {
            (Want::Value | Want::ValueOrClose, Kind::Scalar(scalar)) => {
                self.count(&span)?;
                self.place(Node::scalar(span, scalar))
            }
            (Want::Value | Want::ValueOrClose, Kind::BeginArray) => {
                self.open(&span, Open::Array(Vec::new()))?;
                Ok(Step::Want(Want::ValueOrClose))
            }
            (Want::Value | Want::ValueOrClose, Kind::BeginObject) => {
                let object = Object {
                    entries: Vec::new(),
                    key: None,
                    seen: HashSet::new(),
                };
                self.open(&span, Open::Object(object))?;
                Ok(Step::Want(Want::KeyOrClose))
            }
            (Want::ValueOrClose, Kind::EndArray) | (Want::KeyOrClose, Kind::EndObject) => {
                self.close(span.end)
            }
            (Want::KeyOrClose | Want::Key, Kind::Scalar(scalar))
                if scalar.style() == ScalarStyle::DoubleQuoted =>
            {
                self.key(span, scalar)?;
                Ok(Step::Want(Want::Colon))
            }
            (Want::Colon, Kind::Colon) => Ok(Step::Want(Want::Value)),
            (Want::Next, Kind::Comma) => Ok(Step::Want(if self.in_object() {
                Want::Key
            } else {
                Want::Value
            })),
            (Want::Next, Kind::EndArray) if !self.in_object() => self.close(span.end),
            (Want::Next, Kind::EndObject) if self.in_object() => self.close(span.end),
            (want, _) => Err(syntax(Failure {
                span,
                detail: self.expected(want),
            })),
        }
    }

    fn in_object(&self) -> bool {
        self.stack
            .last()
            .is_some_and(|frame| matches!(frame.open, Open::Object(_)))
    }

    /// What the loop needed when the token it got was no way to go on.
    fn expected(&self, want: Want) -> String {
        match want {
            Want::Value | Want::ValueOrClose => "expected a value",
            Want::KeyOrClose | Want::Key => "expected a key in double quotes",
            Want::Colon => "expected `:`",
            Want::Next if self.in_object() => "expected `,` or `}`",
            Want::Next => "expected `,` or `]`",
        }
        .to_owned()
    }

    /// Adds a node, and refuses a text that goes past `max_nodes`.
    fn count(&mut self, span: &Range<usize>) -> Result<(), Refusal> {
        self.nodes += 1;
        if self.nodes > self.bounds.nodes {
            return Err(refused("max_nodes", self.bounds.nodes, span));
        }
        Ok(())
    }

    /// Opens a container at `span`, and refuses one that nests past
    /// `max_depth`. The root container is the first level.
    fn open(&mut self, span: &Range<usize>, open: Open) -> Result<(), Refusal> {
        if self.stack.len() >= self.bounds.depth {
            return Err(refused("max_depth", self.bounds.depth, span));
        }
        self.count(span)?;
        self.stack.push(Frame {
            start: span.start,
            open,
        });
        Ok(())
    }

    /// Closes the open container, which ends at `end`, into a node.
    fn close(&mut self, end: usize) -> Result<Step, Refusal> {
        let Some(Frame { start, open }) = self.stack.pop() else {
            return Err(syntax(Failure {
                span: end..end,
                detail: "expected a value".to_owned(),
            }));
        };
        let node = match open {
            Open::Array(items) => Node::sequence(start..end, items),
            Open::Object(object) => Node::mapping(start..end, object.entries),
        };
        self.place(node)
    }

    /// Records the key at `span` for the object being read, and refuses it
    /// when a strict reading has seen it already.
    fn key(&mut self, span: Range<usize>, scalar: Scalar) -> Result<(), Refusal> {
        let Some(Frame {
            open: Open::Object(object),
            ..
        }) = self.stack.last_mut()
        else {
            return Err(syntax(Failure {
                span,
                detail: "expected a value".to_owned(),
            }));
        };
        if self.strict && !object.seen.insert(scalar.value().to_owned()) {
            let key = scalar.value().to_owned();
            return Err(Refusal::new(Fault::DuplicateKey { key }, Some(span), None));
        }
        object.key = Some(Node::scalar(span, scalar));
        Ok(())
    }

    /// Hands a finished node to the open container, or ends the read when it
    /// is the root.
    fn place(&mut self, node: Node) -> Result<Step, Refusal> {
        let Some(frame) = self.stack.last_mut() else {
            return Ok(Step::Done(node));
        };
        match &mut frame.open {
            Open::Array(items) => items.push(node),
            Open::Object(object) => {
                let Some(key) = object.key.take() else {
                    return Err(syntax(Failure {
                        span: node.span(),
                        detail: "expected a key in double quotes".to_owned(),
                    }));
                };
                object.entries.push((key, node));
            }
        }
        Ok(Step::Want(Want::Next))
    }
}

fn syntax(failure: Failure) -> Refusal {
    let fault = Fault::Syntax {
        detail: failure.detail,
    };
    Refusal::new(fault, Some(failure.span), None)
}

fn refused(field: &'static str, limit: usize, span: &Range<usize>) -> Refusal {
    let exceeded = Exceeded::new(field, limit, Raise::FOR_TEXT);
    Refusal::new(exceeded.fault(), Some(span.clone()), None)
}
