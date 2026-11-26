//! The argument a Clap error is about, read from the error's context.

use clap::Arg;
use clap::error::{ContextKind, ContextValue, ErrorKind};

/// The argument a Clap error names, and whether the fault is its value.
#[derive(Debug)]
pub(super) struct Blamed<'a> {
    /// The document's argument that the error names.
    pub(super) arg: &'a Arg,
    /// The error is about the argument's value (`InvalidValue` or
    /// `ValueValidation`), not about the argument being there.
    pub(super) on_value: bool,
}

/// The arguments among `supplied` that `error` names under `kind`, matched
/// by the display form of the argument, which Clap writes into the context.
///
/// `supplied` must hold the arguments of a built command: Clap writes the
/// display form of a built argument, and a declared one can read differently.
fn named<'a>(error: &clap::Error, kind: ContextKind, supplied: &[&'a Arg]) -> Vec<&'a Arg> {
    let shown: Vec<&str> = match error.get(kind) {
        Some(ContextValue::String(shown)) => vec![shown.as_str()],
        Some(ContextValue::Strings(shown)) => shown.iter().map(String::as_str).collect(),
        _ => Vec::new(),
    };
    supplied
        .iter()
        .copied()
        .filter(|arg| shown.contains(&arg.to_string().as_str()))
        .collect()
}

/// The first of `supplied` that `error` names as its invalid argument, or
/// else as the prior argument it conflicts with.
pub(super) fn blamed<'a>(error: &clap::Error, supplied: &[&'a Arg]) -> Option<Blamed<'a>> {
    let on_value = matches!(
        error.kind(),
        ErrorKind::InvalidValue | ErrorKind::ValueValidation
    );
    [ContextKind::InvalidArg, ContextKind::PriorArg]
        .into_iter()
        .find_map(|kind| named(error, kind, supplied).into_iter().next())
        .map(|arg| Blamed { arg, on_value })
}

#[cfg(test)]
mod tests {
    use std::iter;

    use clap::error::{ContextKind, ErrorKind};
    use clap::{Arg, ArgAction, Command, value_parser};

    use super::{blamed, named};

    fn toy() -> Command {
        Command::new("toy")
            .arg(
                Arg::new("priority")
                    .long("priority")
                    .value_parser(value_parser!(u8)),
            )
            .arg(
                Arg::new("fast")
                    .long("fast")
                    .action(ArgAction::SetTrue)
                    .conflicts_with("slow"),
            )
            .arg(Arg::new("slow").long("slow").action(ArgAction::SetTrue))
            .arg(Arg::new("pair").long("pair").num_args(2))
            .arg(Arg::new("mode").long("mode").value_parser(["a", "b"]))
            .arg(Arg::new("id"))
    }

    fn built() -> Command {
        let mut command = toy();
        command.build();
        command
    }

    fn fail(args: &[&str]) -> clap::Error {
        toy()
            .try_get_matches_from(iter::once("toy").chain(args.iter().copied()))
            .unwrap_err()
    }

    fn id(arg: &Arg) -> &str {
        arg.get_id().as_str()
    }

    #[test]
    fn an_invalid_value_names_its_argument_and_blames_the_value() {
        let error = fail(&["--priority=high"]);
        assert_eq!(error.kind(), ErrorKind::ValueValidation);
        let command = built();
        let supplied = command.get_arguments().collect::<Vec<_>>();
        let blamed = blamed(&error, &supplied).unwrap();
        assert_eq!(id(blamed.arg), "priority");
        assert!(blamed.on_value);
    }

    #[test]
    fn a_value_outside_the_possible_values_names_its_argument_and_blames_the_value() {
        let error = fail(&["--mode=c"]);
        assert_eq!(error.kind(), ErrorKind::InvalidValue);
        let command = built();
        let supplied = command.get_arguments().collect::<Vec<_>>();
        let blamed = blamed(&error, &supplied).unwrap();
        assert_eq!(id(blamed.arg), "mode");
        assert!(blamed.on_value);
    }

    #[test]
    fn a_conflict_names_both_arguments() {
        let error = fail(&["--fast", "--slow"]);
        assert_eq!(error.kind(), ErrorKind::ArgumentConflict);
        let command = built();
        let supplied = command.get_arguments().collect::<Vec<_>>();
        let invalid = named(&error, ContextKind::InvalidArg, &supplied);
        let prior = named(&error, ContextKind::PriorArg, &supplied);
        assert_eq!(
            invalid.iter().map(|arg| id(arg)).collect::<Vec<_>>(),
            ["fast"]
        );
        assert_eq!(
            prior.iter().map(|arg| id(arg)).collect::<Vec<_>>(),
            ["slow"]
        );
        assert!(!blamed(&error, &supplied).unwrap().on_value);
    }

    #[test]
    fn a_value_under_num_args_names_its_argument() {
        let error = fail(&["--pair=a"]);
        assert_eq!(error.kind(), ErrorKind::WrongNumberOfValues);
        let command = built();
        let supplied = command.get_arguments().collect::<Vec<_>>();
        let blamed = blamed(&error, &supplied).unwrap();
        assert_eq!(id(blamed.arg), "pair");
        assert!(!blamed.on_value);
    }
}
