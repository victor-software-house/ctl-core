//! A document value as command-line tokens, by the action of the argument
//! it fills.

use clap::{Arg, ArgAction, Command};

use crate::input::format::{Format, Resolved, Wanted};
use crate::input::tree::{Node, NodeKind, Scalar};

/// What a value turns into.
#[derive(Debug, Eq, PartialEq)]
pub(super) enum Encoded {
    /// The value gives no token: a null, an empty list, or a `false` with no
    /// `--no-` side.
    Absent,
    /// Tokens for the part of the command line before the positionals.
    Option(Vec<String>),
    /// The values of a positional argument, in order.
    Positional(Vec<String>),
}

/// Which part of the document a refusal is placed on.
#[derive(Debug, Eq, PartialEq)]
pub(super) enum On {
    /// The key.
    Key,
    /// The value.
    Value,
    /// One item of the value, by index.
    Item(usize),
}

/// A value no token can carry, and why.
#[derive(Debug, Eq, PartialEq)]
pub(super) struct Refusal {
    pub(super) on: On,
    pub(super) message: String,
}

fn refuse<T>(on: On, message: String) -> Result<T, Refusal> {
    Err(Refusal { on, message })
}

/// How a person names `arg` on the command line.
pub(super) fn name(arg: &Arg) -> String {
    arg.get_long()
        .map_or_else(|| format!("<{}>", arg.get_id()), |long| format!("--{long}"))
}

/// The scalar `value` is, if it is one.
fn scalar(value: &Node) -> Option<&Scalar> {
    if let NodeKind::Scalar(scalar) = value.kind() {
        Some(scalar)
    } else {
        None
    }
}

/// Whether `format` reads the node as a null.
fn is_null(node: &Node, format: &dyn Format) -> bool {
    scalar(node).is_some_and(|scalar| format.resolve(scalar, node.tag(), Wanted::Null).is_ok())
}

/// The tokens `value` gives for `arg`, a flag, a counted flag, an option or a
/// positional, read by `format`. `level` is the command that declares `arg`
/// or that it reaches, which holds the `--no-` side of a flag.
///
/// # Errors
///
/// A [`Refusal`] for a value that argument cannot take, placed on the key,
/// the value or an item.
pub(super) fn encode(
    arg: &Arg,
    value: &Node,
    format: &dyn Format,
    level: &Command,
) -> Result<Encoded, Refusal> {
    if is_null(value, format) {
        return Ok(Encoded::Absent);
    }
    let action = arg.get_action();
    if matches!(action, ArgAction::SetTrue) {
        flag(arg, value, format, level)
    } else if matches!(action, ArgAction::Count) {
        count(arg, value, format)
    } else if matches!(action, ArgAction::Set | ArgAction::Append) {
        values(arg, value, format)
    } else {
        refuse(
            On::Key,
            format!("`{}` is a flag a document cannot fill", name(arg)),
        )
    }
}

/// `true` as the flag, `false` as its `--no-` side when it has one.
fn flag(arg: &Arg, value: &Node, format: &dyn Format, level: &Command) -> Result<Encoded, Refusal> {
    let long = arg.get_long().unwrap_or_default();
    let negated = format!("no-{long}");
    let tokens = if boolean(arg, value, format)? {
        vec![format!("--{long}")]
    } else if level
        .get_arguments()
        .any(|other| other.get_long() == Some(negated.as_str()))
    {
        vec![format!("--{negated}")]
    } else {
        return Ok(Encoded::Absent);
    };
    Ok(Encoded::Option(tokens))
}

fn boolean(arg: &Arg, value: &Node, format: &dyn Format) -> Result<bool, Refusal> {
    let Some(scalar) = scalar(value) else {
        return refuse(On::Value, format!("`{}` takes true or false", name(arg)));
    };
    match format.resolve(scalar, value.tag(), Wanted::Bool) {
        Ok(Resolved::Bool(given)) => Ok(given),
        Ok(_) => refuse(On::Value, format!("`{}` takes true or false", name(arg))),
        Err(mismatch) => refuse(On::Value, mismatch.to_string()),
    }
}

/// An integer from 0 to 255 as that many repeats of the flag.
fn count(arg: &Arg, value: &Node, format: &dyn Format) -> Result<Encoded, Refusal> {
    let Some(scalar) = scalar(value) else {
        return refuse(
            On::Value,
            format!("`{}` takes a count from 0 to 255", name(arg)),
        );
    };
    let repeats = match format.resolve(scalar, value.tag(), Wanted::Int) {
        Ok(Resolved::Int(number)) => u8::try_from(number).ok(),
        Ok(Resolved::Uint(_)) => None,
        Ok(_) => return refuse(On::Value, "expected an integer".to_owned()),
        Err(mismatch) => return refuse(On::Value, mismatch.to_string()),
    };
    let Some(repeats) = repeats else {
        return refuse(
            On::Key,
            format!("`{}` takes a count from 0 to 255", name(arg)),
        );
    };
    Ok(Encoded::Option(vec![
        format!(
            "--{}",
            arg.get_long().unwrap_or_default()
        );
        usize::from(repeats)
    ]))
}

/// A scalar as one value, a sequence as its items.
fn values(arg: &Arg, value: &Node, format: &dyn Format) -> Result<Encoded, Refusal> {
    let delimiter = arg.get_value_delimiter();
    let appends = matches!(arg.get_action(), ArgAction::Append);
    let per_use = arg.get_num_args().map_or(1, |range| range.max_values());
    let several = appends || delimiter.is_some() || per_use > 1;
    let listed = matches!(value.kind(), NodeKind::Sequence(_));
    let texts = if let NodeKind::Scalar(single) = value.kind() {
        vec![single.value().to_owned()]
    } else if let NodeKind::Sequence(items) = value.kind() {
        if !several {
            return refuse(
                On::Value,
                format!("`{}` takes one value, not a list", name(arg)),
            );
        }
        items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                scalar(item)
                    .filter(|_| !is_null(item, format))
                    .map(|text| text.value().to_owned())
                    .ok_or_else(|| Refusal {
                        on: On::Item(index),
                        message: format!(
                            "`{}` takes text in a list, not a null, a list or a mapping",
                            name(arg)
                        ),
                    })
            })
            .collect::<Result<Vec<_>, _>>()?
    } else {
        return refuse(
            On::Value,
            format!("`{}` takes text, not a mapping", name(arg)),
        );
    };
    if texts.is_empty() {
        return Ok(Encoded::Absent);
    }
    if arg.is_positional() {
        return Ok(Encoded::Positional(texts));
    }
    let long = arg.get_long().unwrap_or_default();
    let tokens = match delimiter {
        Some(delimiter) if listed => {
            if let Some(index) = texts.iter().position(|text| text.contains(delimiter)) {
                return refuse(
                    On::Item(index),
                    format!(
                        "`{}` splits its values on `{delimiter}`, so an item cannot hold it",
                        name(arg)
                    ),
                );
            }
            vec![format!("--{long}={}", texts.join(&delimiter.to_string()))]
        }
        None if listed && !appends => {
            return refuse(
                On::Value,
                format!(
                    "`{}` takes several values with no delimiter, so a list cannot fill it",
                    name(arg)
                ),
            );
        }
        None if listed && per_use > 1 => {
            return refuse(
                On::Value,
                format!(
                    "`{}` takes several values at each use, so a list cannot fill it",
                    name(arg)
                ),
            );
        }
        _ => texts
            .iter()
            .map(|text| format!("--{long}={text}"))
            .collect(),
    };
    Ok(Encoded::Option(tokens))
}
