//! The TOML half of [`Input`]: parse with the `toml` crate's span-carrying
//! tree, hold the tree to the limits, and place each problem on the value it
//! names.
//!
//! Every position here is a byte span in the text, turned into a 1-based line
//! and column once, in [`position`].

use std::ops::Range;

use serde::de::DeserializeOwned;
use serde_saphyr::Budget;
use toml::Spanned;
use toml::de::{DeTable, DeValue, Deserializer};

use super::limits::{Exceeded, Raise};
use super::{Input, InputError, Problem, Step, field_steps};

/// [`Input::parse`] for a TOML text: the tree, then the limits, then the
/// typed value. The first problem found is the one reported.
pub(super) fn parse<T: DeserializeOwned>(
    input: &Input,
    budget: &Budget,
    raise: Raise,
) -> Result<T, InputError> {
    let root = DeTable::parse(&input.text)
        .map_err(|error| refusal(input, error.span(), error.message()))?;
    let mut measure = Measure {
        max_depth: budget.max_depth,
        max_nodes: budget.max_nodes,
        nodes: 0,
        raise,
    };
    measure
        .table(&root.span(), root.get_ref(), 1)
        .map_err(|refused| refusal(input, Some(refused.at), refused.exceeded.to_string()))?;
    T::deserialize(Deserializer::from(root))
        .map_err(|error| refusal(input, error.span(), error.message()))
}

/// The 1-based line and column of each garde field in `text`, or of its
/// nearest written ancestor. `None` for every field when `text` does not
/// parse.
pub(super) fn places(text: &str, fields: &[&str]) -> Vec<Option<(usize, usize)>> {
    let root = DeTable::parse(text)
        .ok()
        .map(|table| Spanned::new(table.span(), DeValue::Table(table.into_inner())));
    fields
        .iter()
        .map(|field| {
            root.as_ref()
                .map(|root| position(text, nearest(root, &field_steps(field)).start))
        })
        .collect()
}

/// The span of the value `steps` reach, or of the last one that exists.
fn nearest(root: &Spanned<DeValue<'_>>, steps: &[Step<'_>]) -> Range<usize> {
    let mut found = root;
    for step in steps {
        let next = match step {
            Step::Key(key) => found.get_ref().get(*key),
            Step::Index(index) => found.get_ref().get(*index),
        };
        let Some(next) = next else { break };
        found = next;
    }
    found.span()
}

/// A problem with the whole input, placed at `span` when the parser gave one.
fn refusal(input: &Input, span: Option<Range<usize>>, message: impl Into<String>) -> InputError {
    let place = span.map(|span| position(&input.text, span.start));
    InputError {
        name: input.name.clone(),
        problems: vec![Problem {
            line: place.map(|(line, _)| line + input.lines_before),
            column: place.map(|(_, column)| column),
            field: String::new(),
            message: message.into(),
        }],
    }
}

/// The 1-based line and column, counted in characters, of byte `offset` in
/// `text`. An offset past the end, as at an unclosed array, is the end.
fn position(text: &str, offset: usize) -> (usize, usize) {
    let before = String::from_utf8_lossy(&text.as_bytes()[..offset.min(text.len())]);
    let line = before.split('\n').count();
    let column = 1 + before
        .rsplit('\n')
        .next()
        .map_or(0, |last| last.chars().count());
    (line, column)
}

/// A limit a tree went past, and where in the text.
struct Refused {
    exceeded: Exceeded,
    at: Range<usize>,
}

/// Walks a tree in the order it was written, counting depth and nodes
/// against the limits in force. `max_depth` is YAML's rule too: a table or
/// an array is one level, and the root table is the first.
struct Measure {
    max_depth: usize,
    max_nodes: usize,
    nodes: usize,
    raise: Raise,
}

impl Measure {
    /// A table at `depth`, then its entries in the order they were written:
    /// a parsed table iterates by key, not by position.
    fn table(
        &mut self,
        span: &Range<usize>,
        table: &DeTable<'_>,
        depth: usize,
    ) -> Result<(), Refused> {
        self.container(span, depth)?;
        let mut entries: Vec<_> = table.iter().collect();
        entries.sort_by_key(|(key, _)| key.span().start);
        entries
            .into_iter()
            .try_for_each(|(_, value)| self.value(value, depth + 1))
    }

    fn value(&mut self, value: &Spanned<DeValue<'_>>, depth: usize) -> Result<(), Refused> {
        let span = value.span();
        match value.get_ref() {
            DeValue::Table(table) => self.table(&span, table, depth),
            DeValue::Array(items) => {
                self.container(&span, depth)?;
                items
                    .iter()
                    .try_for_each(|item| self.value(item, depth + 1))
            }
            DeValue::String(_)
            | DeValue::Integer(_)
            | DeValue::Float(_)
            | DeValue::Boolean(_)
            | DeValue::Datetime(_) => self.node(&span),
        }
    }

    /// A table or an array: one level deeper, and one node.
    fn container(&mut self, span: &Range<usize>, depth: usize) -> Result<(), Refused> {
        if depth > self.max_depth {
            return Err(self.refuse("max_depth", self.max_depth, span));
        }
        self.node(span)
    }

    fn node(&mut self, span: &Range<usize>) -> Result<(), Refused> {
        self.nodes += 1;
        if self.nodes > self.max_nodes {
            return Err(self.refuse("max_nodes", self.max_nodes, span));
        }
        Ok(())
    }

    fn refuse(&self, field: &'static str, limit: usize, span: &Range<usize>) -> Refused {
        Refused {
            exceeded: Exceeded::new(field, limit, self.raise),
            at: span.clone(),
        }
    }
}
