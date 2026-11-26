//! The input document: a file of values for the arguments of one command.
//!
//! The document becomes argument tokens that Clap parses with the command
//! line, once, so every check a typed flag gets applies to a document value.
//! The command line wins over the environment, the environment over the
//! document, and the document over the default.

use std::ffi::OsString;
use std::path::PathBuf;

use clap::{Arg, ArgAction, Command, Id, value_parser};

use self::encode::{Encoded, On, Refusal};
use crate::flags::{Placed, levels, placed};
use crate::input::format::Format;
use crate::input::tree::{Node, NodeKind};
use crate::input::{self, FieldPath, Input, InputError, Problem};

mod blame;
mod encode;
mod flags;
mod position;
mod precedence;
mod source;

pub(super) use flags::{add as add_flags, clashes};

/// The arguments ctl-core's own flags declare, which the output view reads
/// from the command line before any document is read.
const OUTPUT_IDS: [&str; 5] = ["format", "json", "stream", "color", "no_color"];

/// What a run does with a document key that the command line or the
/// environment decides already, see
/// [`App::input_conflicts`](super::App::input_conflicts).
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum InputConflicts {
    /// Drop the key, so the command line wins over the document without a
    /// word. A caller overrides one run's value without editing the file.
    #[default]
    Drop,
    /// Refuse the key, with a problem on it that names the flag or the
    /// variable it lost to.
    Fail,
}

/// A document read and joined into the command line.
#[derive(Debug)]
pub(super) struct Joined {
    argv: Vec<OsString>,
    input: Input,
    /// The command as Clap built it to read the command line, whose arguments
    /// display as Clap writes them in an error.
    built: Command,
    path: Vec<String>,
    /// The arguments the document supplied, with the key each came from.
    supplied: Vec<(Id, String)>,
}

impl Joined {
    /// The command line with the document's tokens in it.
    pub(super) fn argv(&self) -> &[OsString] {
        &self.argv
    }

    /// The problem on the key or value that `error` is about, when Clap's
    /// error names an argument the document supplied. `None` for an error
    /// about the command line, or about no argument at all.
    pub(super) fn blame(&self, error: &clap::Error) -> Option<InputError> {
        let level = self
            .path
            .iter()
            .try_fold(&self.built, |level, name| level.find_subcommand(name))?;
        let supplied: Vec<&Arg> = self
            .supplied
            .iter()
            .filter_map(|(id, _)| level.get_arguments().find(|arg| arg.get_id() == id))
            .collect();
        let blamed = blame::blamed(error, &supplied)?;
        let key = &self
            .supplied
            .iter()
            .find(|(id, _)| id == blamed.arg.get_id())?
            .1;
        let path = FieldPath::root().key(key.as_str());
        let message = blame::message(error);
        let problem = if blamed.on_value {
            self.input.problem_at(&path, message)
        } else {
            key_problem(&self.input, &path, message)
        };
        InputError::try_new(self.input.name(), [problem])
    }
}

/// A problem on the key at `path`, which `Input::problem_at` places on the
/// value.
fn key_problem(input: &Input, path: &FieldPath, message: impl Into<String>) -> Problem {
    let place = input.place_key(path);
    Problem {
        line: place.map(|at| at.line),
        column: place.map(|at| at.column),
        field: path.to_string(),
        message: message.into(),
    }
}

/// A [`Problem`] for a refusal of the value under `name`.
fn refused(input: &Input, name: &str, refusal: Refusal) -> Problem {
    let path = FieldPath::root().key(name);
    match refusal.on {
        On::Key => key_problem(input, &path, refusal.message),
        On::Value => input.problem_at(&path, refusal.message),
        On::Item(index) => input.problem_at(&path.index(index), refusal.message),
    }
}

/// One key of the document that fills an argument.
struct Planned<'a> {
    name: &'a str,
    arg: &'a Arg,
    encoded: Encoded,
}

/// What reading one document into one command line needs to know.
struct Reading<'a> {
    input: &'a Input,
    format: &'static dyn Format,
    /// The selected command, as Clap built it to read the command line.
    level: &'a Command,
    raw: &'a [OsString],
    policy: InputConflicts,
    /// The output shorthand flags the registry added, which are no keys.
    reserved: &'a [&'a str],
    placed: &'a [Placed<'a>],
    /// The depth of the selected command among the subcommand path.
    deepest: usize,
    /// The selected command's positionals, in index order.
    positionals: Vec<&'a Arg>,
}

impl<'a> Reading<'a> {
    /// The rank of `arg` among the positionals.
    fn rank(&self, arg: &Arg) -> Option<usize> {
        self.positionals
            .iter()
            .position(|positional| positional.get_id() == arg.get_id())
    }

    /// Why a positional cannot come from the document, if it cannot.
    /// The document supplies positionals only when the command line gives none
    /// at the selected level and holds no `--`.
    /// The command must have no last or optional-middle positional either.
    fn positional_refusal(&self) -> Option<&'static str> {
        let given = self
            .placed
            .iter()
            .any(|entry| entry.depth == self.deepest && entry.arg.is_positional() && entry.given);
        if given {
            Some("the command line already gives a positional argument, so give them all there")
        } else if self.raw.iter().any(|arg| arg == "--") {
            Some("the command line holds `--`, so give the positional arguments there")
        } else if self.level.is_allow_missing_positional_set()
            || self.positionals.iter().any(|arg| arg.is_last_set())
        {
            Some("this command has a last or skippable positional argument, so give them there")
        } else {
            None
        }
    }

    /// Encode each entry of the document's mapping.
    /// A key that names no argument, or holds a value no token can carry, is a
    /// problem.
    fn plan(&self, entries: &'a [(Node, Node)], problems: &mut Vec<Problem>) -> Vec<Planned<'a>> {
        let mut planned = Vec::new();
        for (key, value) in entries {
            let NodeKind::Scalar(scalar) = key.kind() else {
                problems.push(Problem {
                    line: None,
                    column: None,
                    field: String::new(),
                    message: "a key must be text".to_owned(),
                });
                continue;
            };
            let name = scalar.value();
            let Some(arg) = named(self.level, name, self.reserved) else {
                let message = format!(
                    "unknown key `{name}`; `{}` takes no argument of that name",
                    self.level.get_name()
                );
                problems.push(key_problem(
                    self.input,
                    &FieldPath::root().key(name),
                    message,
                ));
                continue;
            };
            match encode::encode(arg, value, self.format, self.level) {
                Ok(Encoded::Absent) => {}
                Ok(encoded) => planned.push(Planned { name, arg, encoded }),
                Err(refusal) => problems.push(refused(self.input, name, refusal)),
            }
        }
        planned
    }

    /// The keys that stay.
    /// A positional the document may not supply, or supplies with a gap before
    /// it, is a problem.
    /// So is a key that lost to the command line or the environment under
    /// [`InputConflicts::Fail`], which `Drop` leaves out without a word.
    fn vet(&self, planned: Vec<Planned<'a>>, problems: &mut Vec<Problem>) -> Vec<Planned<'a>> {
        let blocked = self.positional_refusal();
        let supplied: Vec<usize> = planned
            .iter()
            .filter(|entry| matches!(entry.encoded, Encoded::Positional(_)))
            .filter_map(|entry| self.rank(entry.arg))
            .collect();
        let mut kept = Vec::new();
        for entry in planned {
            let path = FieldPath::root().key(entry.name);
            if matches!(entry.encoded, Encoded::Positional(_)) {
                let gap = self
                    .rank(entry.arg)
                    .is_some_and(|rank| (0..rank).any(|lower| !supplied.contains(&lower)));
                let message = match (blocked, gap) {
                    (Some(reason), _) => Some(format!(
                        "`{}` is a positional argument, but {reason}",
                        entry.name
                    )),
                    (None, true) => Some(format!(
                        "`{}` is a positional argument, and the document gives none of the ones before it",
                        entry.name
                    )),
                    (None, false) => None,
                };
                if let Some(message) = message {
                    problems.push(key_problem(self.input, &path, message));
                    continue;
                }
            }
            let beaten = precedence::find(self.placed, entry.arg, self.deepest)
                .and_then(|key| precedence::lost(key, self.placed));
            match (beaten, self.policy) {
                (Some(_), InputConflicts::Drop) => {}
                (Some(loss), InputConflicts::Fail) => {
                    let message = format!(
                        "`{}` conflicts with {}; remove one of them",
                        entry.name,
                        loss.describe()
                    );
                    problems.push(key_problem(self.input, &path, message));
                }
                (None, _) => kept.push(entry),
            }
        }
        kept
    }

    /// The command line with the option tokens of `kept` at `at`, right after
    /// the selected command's name.
    /// The positional values go at the end after one `--`, in index order.
    fn argv(&self, kept: Vec<Planned<'a>>, at: usize) -> Vec<OsString> {
        let mut options = Vec::new();
        let mut values: Vec<(usize, Vec<String>)> = Vec::new();
        for entry in kept {
            match entry.encoded {
                Encoded::Option(tokens) => options.extend(tokens),
                Encoded::Positional(texts) => {
                    values.push((self.rank(entry.arg).unwrap_or_default(), texts));
                }
                Encoded::Absent => {}
            }
        }
        values.sort_by_key(|(rank, _)| *rank);
        let mut argv = self.raw.get(..at).unwrap_or(self.raw).to_vec();
        argv.extend(options.into_iter().map(OsString::from));
        argv.extend_from_slice(self.raw.get(at..).unwrap_or_default());
        if !values.is_empty() {
            argv.push(OsString::from("--"));
            argv.extend(
                values
                    .into_iter()
                    .flat_map(|(_, texts)| texts)
                    .map(OsString::from),
            );
        }
        argv
    }
}

/// The positionals of `level`, in index order.
fn ranked(level: &Command) -> Vec<&Arg> {
    let mut positionals: Vec<&Arg> = level.get_positionals().collect();
    positionals.sort_by_key(|arg| arg.get_index());
    positionals
}

/// The arguments a document key may name.
/// Hidden arguments, help, version, `--input` and `--input-format` are not
/// keys, and neither are ctl-core's output flags.
/// The view reads their value from the command line itself.
fn usable(arg: &Arg, reserved: &[&str]) -> bool {
    let id = arg.get_id().as_str();
    !arg.is_hide_set()
        && !matches!(
            arg.get_action(),
            ArgAction::Help | ArgAction::HelpShort | ArgAction::HelpLong | ArgAction::Version
        )
        && id != flags::INPUT
        && id != flags::INPUT_FORMAT
        && !OUTPUT_IDS.contains(&id)
        && !reserved.contains(&id)
}

/// The argument a key names at `level`: a long name, else a positional's id.
fn named<'a>(level: &'a Command, key: &str, reserved: &[&str]) -> Option<&'a Arg> {
    level
        .get_arguments()
        .filter(|arg| usable(arg, reserved))
        .find(|arg| arg.get_long() == Some(key))
        .or_else(|| {
            level
                .get_positionals()
                .filter(|arg| usable(arg, reserved))
                .find(|arg| arg.get_id() == key)
        })
}

/// Read the document `--input` names into `raw`, or return `None` when the
/// run names none.
///
/// `command` is the one the parse reads, with the variables the command line
/// overrules already unbound. `reserved` holds the output shorthand flags
/// the registry added, which are no keys either.
///
/// Returns `None` too when `--input-format` names no registered format, so
/// the parse reports Clap's own error for it.
///
/// # Errors
///
/// An [`InputError`] with every problem ctl-core finds, each on its line.
/// A problem is a document that cannot be read, an unknown key, a value no
/// token can carry, or a positional that cannot come from a document.
/// Under [`InputConflicts::Fail`], each key that lost to the command line or
/// the environment is one too.
pub(super) fn join(
    command: &Command,
    raw: &[OsString],
    policy: InputConflicts,
    reserved: &[&str],
) -> Result<Option<Joined>, InputError> {
    let mut probe = command
        .clone()
        .mut_arg(flags::INPUT_FORMAT, |arg| {
            arg.value_parser(value_parser!(String))
        })
        .ignore_errors(true);
    let Ok(matches) = probe.try_get_matches_from_mut(raw) else {
        return Ok(None);
    };
    let levels = levels(&probe, &matches, command);
    let Some(&(level, at_level)) = levels.last() else {
        return Ok(None);
    };
    let Ok(Some(file)) = at_level.try_get_one::<PathBuf>(flags::INPUT) else {
        return Ok(None);
    };
    let format = match at_level.try_get_one::<String>(flags::INPUT_FORMAT) {
        Ok(Some(name)) => match input::default_formats().get(name) {
            Some(format) => Some(format),
            None => return Ok(None),
        },
        _ => None,
    };
    let (input, format) = source::read(file, format)?;
    let placed = placed(&levels);
    let reading = Reading {
        input: &input,
        format,
        level,
        raw,
        policy,
        reserved,
        placed: &placed,
        deepest: levels.len() - 1,
        positionals: ranked(level),
    };
    let (argv, supplied, path) = input.tree(|root| {
        let NodeKind::Mapping(entries) = root.kind() else {
            return Err(InputError::try_new(
                input.name(),
                [input.problem_at(
                    &FieldPath::root(),
                    "the document must be a mapping of argument names to values",
                )],
            )
            .unwrap_or_else(|| InputError::whole(input.name(), "the document is not a mapping")));
        };
        let Some(selected) = position::selected(command, raw) else {
            return Err(InputError::whole(
                input.name(),
                "cannot tell which command the document fills",
            ));
        };
        let mut problems = Vec::new();
        let planned = reading.plan(entries, &mut problems);
        let kept = reading.vet(planned, &mut problems);
        if let Some(failed) = InputError::try_new(input.name(), problems) {
            return Err(failed);
        }
        let supplied = kept
            .iter()
            .map(|entry| (entry.arg.get_id().clone(), entry.name.to_owned()))
            .collect();
        Ok((reading.argv(kept, selected.at), supplied, selected.path))
    })??;
    Ok(Some(Joined {
        argv,
        input,
        built: probe,
        path,
        supplied,
    }))
}
