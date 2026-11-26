//! Where the tokens of a document go in the command line.

use std::ffi::OsString;

use clap::Command;

use crate::flags::levels;

/// The subcommand the command line selects, and where the document's option
/// tokens go in it.
/// That is right after the name of the subcommand, or right after the binary
/// name at the root.
#[derive(Debug, Eq, PartialEq)]
pub(super) struct Selected {
    /// The names of the selected subcommands, outermost first, as the command
    /// declares them. An alias or an inferred prefix reads as its name.
    pub(super) path: Vec<String>,
    /// The index of the first token the document's tokens come before.
    pub(super) at: usize,
}

/// The subcommand path that Clap selects from `raw`.
/// The read is lenient, so a missing value or a required argument that is not
/// there yet does not end it.
///
/// `None` when Clap stops for help or version.
fn path(command: &Command, raw: &[OsString]) -> Option<Vec<String>> {
    let mut probe = command.clone().ignore_errors(true);
    let matches = probe.try_get_matches_from_mut(raw).ok()?;
    Some(
        levels(&probe, &matches, command)
            .iter()
            .skip(1)
            .map(|(level, _)| level.get_name().to_owned())
            .collect(),
    )
}

/// Find the selected subcommand and where its name ends, by probing growing
/// prefixes of `raw` with Clap itself.
///
/// The first prefix whose lenient read selects the whole path ends at the last
/// name.
/// Clap alone knows whether a token is a subcommand name, an inferred prefix
/// of one, a flag that selects one, or the value of an option.
/// So no other reading of the command line is written here.
///
/// `None` when Clap stops for help or version, or no prefix selects the path.
pub(super) fn selected(command: &Command, raw: &[OsString]) -> Option<Selected> {
    let path = path(command, raw)?;
    let at = (1..=raw.len()).find(|end| {
        raw.get(..*end)
            .and_then(|prefix| self::path(command, prefix))
            .is_some_and(|found| found == path)
    })?;
    Some(Selected { path, at })
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use clap::{Arg, Command};

    use super::{Selected, selected};

    fn toy() -> Command {
        Command::new("toy")
            .infer_subcommands(true)
            .arg(Arg::new("name").long("name").global(true))
            .subcommand(
                Command::new("add")
                    .infer_subcommands(true)
                    .arg(Arg::new("priority").long("priority"))
                    .subcommand(
                        Command::new("label")
                            .infer_subcommands(true)
                            .arg(Arg::new("title").long("title"))
                            .subcommand(Command::new("remove")),
                    ),
            )
            .subcommand(Command::new("sync").short_flag('S').long_flag("sync"))
    }

    fn raw(args: &[&str]) -> Vec<OsString> {
        args.iter().map(OsString::from).collect()
    }

    fn found(args: &[&str]) -> Option<(Vec<String>, usize)> {
        selected(&toy(), &raw(args)).map(|Selected { path, at }| (path, at))
    }

    fn names(path: &[&str]) -> Vec<String> {
        path.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn the_root_takes_its_tokens_after_the_binary_name() {
        assert_eq!(found(&["toy"]), Some((names(&[]), 1)));
        assert_eq!(found(&["toy", "--name", "x"]), Some((names(&[]), 1)));
    }

    #[test]
    fn a_subcommand_takes_its_tokens_after_its_name() {
        assert_eq!(found(&["toy", "add"]), Some((names(&["add"]), 2)));
        assert_eq!(
            found(&["toy", "add", "--priority", "2"]),
            Some((names(&["add"]), 2))
        );
    }

    #[test]
    fn a_subcommand_two_levels_deep_takes_them_after_the_innermost_name() {
        assert_eq!(
            found(&["toy", "add", "label", "--title", "x"]),
            Some((names(&["add", "label"]), 3))
        );
        assert_eq!(
            found(&["toy", "add", "label", "remove"]),
            Some((names(&["add", "label", "remove"]), 4))
        );
    }

    #[test]
    fn root_options_before_the_subcommand_name_come_first() {
        assert_eq!(
            found(&["toy", "--name", "x", "add", "label"]),
            Some((names(&["add", "label"]), 5))
        );
        assert_eq!(
            found(&["toy", "--name=x", "add"]),
            Some((names(&["add"]), 3))
        );
    }

    #[test]
    fn an_option_value_that_spells_a_subcommand_is_a_value() {
        assert_eq!(
            found(&["toy", "--name", "add", "add"]),
            Some((names(&["add"]), 4))
        );
    }

    #[test]
    fn an_inferred_subcommand_reads_as_its_name() {
        assert_eq!(found(&["toy", "ad"]), Some((names(&["add"]), 2)));
        assert_eq!(
            found(&["toy", "ad", "lab"]),
            Some((names(&["add", "label"]), 3))
        );
    }

    #[test]
    fn a_flag_subcommand_reads_as_its_name() {
        assert_eq!(found(&["toy", "-S"]), Some((names(&["sync"]), 2)));
        assert_eq!(found(&["toy", "--sync"]), Some((names(&["sync"]), 2)));
        assert_eq!(
            found(&["toy", "--name", "x", "--sync"]),
            Some((names(&["sync"]), 4))
        );
    }

    #[test]
    fn tokens_after_a_double_dash_select_nothing() {
        assert_eq!(
            found(&["toy", "add", "--", "label"]),
            Some((names(&["add"]), 2))
        );
    }
}
