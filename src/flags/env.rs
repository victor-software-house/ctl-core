//! Bind every long option to an environment variable.

use std::ffi::OsString;

use clap::error::ErrorKind;
use clap::parser::ValueSource;
use clap::{Arg, ArgAction, ArgMatches, Command, Id};

/// Binds each long option of a [`Command`] and of every subcommand to
/// `<PREFIX>_<LONG>`, so the environment can set any flag a CLI declares.
///
/// Put it on the root parser as a derive attribute, with the trait in scope
/// (the prelude exports it): `#[command(env_prefix = "VERCTL")]`. Clap's
/// derive applies a root attribute after it has added every argument and
/// subcommand, so `C::command()` returns the bound graph to help, the Usage
/// spec, the pre-parse view, and the parse alike.
///
/// A command-line flag wins over its variable, which wins over the default,
/// once [`env_under_flags`] has run before the parse. A CLI that also reads a
/// config file applies a file value only where neither was given.
///
/// An option is left unbound when it already declares its own `env` (the
/// override), has no long name, is hidden, is the `--no-*` side of a pair,
/// or is help or version. Not calling `env_prefix` disables the binding.
pub trait EnvPrefix {
    /// Bind every eligible long option under `prefix`.
    #[must_use]
    fn env_prefix(self, prefix: &str) -> Self;
}

impl EnvPrefix for Command {
    fn env_prefix(self, prefix: &str) -> Self {
        self.mut_args(|arg| bind(arg, prefix))
            .mut_subcommands(|subcommand| subcommand.env_prefix(prefix))
    }
}

/// The variable a long option reads: `dry-run` under `VERCTL` is
/// `VERCTL_DRY_RUN`.
#[must_use]
fn env_name(prefix: &str, long: &str) -> String {
    format!("{prefix}_{}", long.to_ascii_uppercase().replace('-', "_"))
}

fn bind(arg: Arg, prefix: &str) -> Arg {
    let name = match arg.get_long() {
        Some(long)
            if arg.get_env().is_none()
                && !arg.is_hide_set()
                && !long.starts_with("no-")
                && !matches!(
                    arg.get_action(),
                    ArgAction::Help
                        | ArgAction::HelpShort
                        | ArgAction::HelpLong
                        | ArgAction::Version
                ) =>
        {
            env_name(prefix, long)
        }
        _ => return arg,
    };
    arg.env(name)
}

/// Removes each variable that the command line overrules, so the command line
/// wins, and refuses two variables that conflict.
///
/// Clap treats a variable as given by the user: a variable that conflicts
/// with a flag on the command line, or with its `--no-*` side, fails the
/// parse ([clap#5591](https://github.com/clap-rs/clap/issues/5591)). This
/// parses `raw` once leniently to learn which options the command line set
/// at each subcommand level, then unbinds each variable that:
///
/// 1. belongs to an option the command line set;
/// 2. belongs to the other side of a `--x`/`--no-x` pair the command line set;
/// 3. belongs to an option that conflicts, either way, with one the command
///    line set: a `conflicts_with` on either side, a single-choice group they
///    share, or either side being `exclusive`;
/// 4. is empty, as a composite action sets an input it was not given;
/// 5. reads as a boolean flag's default, `false` for a flag that sets `true`,
///    so it changes nothing and cannot conflict.
///
/// Clap keeps `overrides_with` and a group's own `conflicts_with` private, so
/// this sees overrides only between `--x`/`--no-x` pairs, and no conflict
/// that a group declares. Call this on the command just before the parse,
/// and parse the command it returns; `App` does.
///
/// # Errors
///
/// An `ArgumentConflict` error that names both variables when two remaining
/// variables belong to conflicting options. Clap's own error would name two
/// flags the user never typed.
pub fn env_under_flags(command: Command, raw: &[OsString]) -> Result<Command, clap::Error> {
    let mut probe = command.clone().ignore_errors(true);
    let Ok(matches) = probe.try_get_matches_from_mut(raw) else {
        return Ok(command);
    };
    let levels = levels(&probe, &matches, &command);
    let placed = placed(&levels);
    let mut unset = Vec::new();
    let mut kept = Vec::<&Placed<'_>>::new();
    for option in placed
        .iter()
        .filter(|option| option.arg.get_env().is_some())
    {
        let overruled = !option.is_set()
            || option.given
            || placed
                .iter()
                .any(|given| given.given && (option.negates(given) || option.conflicts(given)));
        if overruled {
            unset.push((option.depth, option.arg.get_id().clone()));
        } else if let Some(other) = kept.iter().find(|other| option.conflicts(other)) {
            let mut command = levels
                .last()
                .map_or_else(|| probe.clone(), |(level, _)| (*level).clone());
            return Err(clap::Error::raw(
                ErrorKind::ArgumentConflict,
                format!(
                    "the variables `{}` and `{}` cannot both be set\n",
                    other.env_name(),
                    option.env_name()
                ),
            )
            .format(&mut command));
        } else {
            kept.push(option);
        }
    }
    let path = levels
        .iter()
        .skip(1)
        .map(|(level, _)| level.get_name())
        .collect::<Vec<_>>();
    Ok(unbind(command, &path, 0, &unset))
}

/// The probe's command and matches at each level of the subcommand path,
/// as far as the declared command has that subcommand.
fn levels<'a>(
    mut probe: &'a Command,
    mut matches: &'a ArgMatches,
    mut declared: &Command,
) -> Vec<(&'a Command, &'a ArgMatches)> {
    let mut levels = vec![(probe, matches)];
    while let Some((name, sub)) = matches.subcommand() {
        let (Some(next), Some(next_declared)) =
            (probe.find_subcommand(name), declared.find_subcommand(name))
        else {
            break;
        };
        probe = next;
        matches = sub;
        declared = next_declared;
        levels.push((probe, matches));
    }
    levels
}

/// One option on the subcommand path, at the level that declares it.
struct Placed<'a> {
    depth: usize,
    arg: &'a Arg,
    conflicts: Vec<&'a Id>,
    given: bool,
}

/// Every option on the path once. A global option appears at the level that
/// declares it, not at the levels clap copies it into.
fn placed<'a>(levels: &[(&'a Command, &'a ArgMatches)]) -> Vec<Placed<'a>> {
    let mut options = Vec::new();
    for (depth, (level, matches)) in levels.iter().enumerate() {
        let parent = depth.checked_sub(1).and_then(|up| levels.get(up));
        for arg in level.get_arguments() {
            let copied = arg.is_global_set()
                && parent.is_some_and(|(up, _)| {
                    up.get_arguments()
                        .any(|other| other.get_id() == arg.get_id())
                });
            if copied {
                continue;
            }
            let mut conflicts = level
                .get_arg_conflicts_with(arg)
                .into_iter()
                .map(Arg::get_id)
                .collect::<Vec<_>>();
            for group in level.get_groups() {
                if group.get_args().any(|id| id == arg.get_id()) && !group.clone().is_multiple() {
                    conflicts.extend(group.get_args().filter(|id| *id != arg.get_id()));
                }
            }
            options.push(Placed {
                depth,
                arg,
                conflicts,
                given: matches.value_source(arg.get_id().as_str())
                    == Some(ValueSource::CommandLine),
            });
        }
    }
    options
}

impl Placed<'_> {
    fn env_name(&self) -> String {
        self.arg
            .get_env()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    /// Whether the variable holds a value that changes the parse.
    fn is_set(&self) -> bool {
        let Some(value) = self.arg.get_env().and_then(std::env::var_os) else {
            return false;
        };
        let action = self.arg.get_action();
        let default = if matches!(action, ArgAction::SetTrue) {
            Some("false")
        } else if matches!(action, ArgAction::SetFalse) {
            Some("true")
        } else {
            None
        };
        !value.is_empty() && default.is_none_or(|default| value != default)
    }

    /// Whether the two options can meet: at one level, or one is a global
    /// declared above the other.
    fn meets(&self, other: &Self) -> bool {
        let upper = if self.depth <= other.depth {
            self
        } else {
            other
        };
        self.depth == other.depth || upper.arg.is_global_set()
    }

    /// Whether clap would refuse the two options together.
    fn conflicts(&self, other: &Self) -> bool {
        let exclusive = self.arg.is_exclusive_set() || other.arg.is_exclusive_set();
        self.meets(other)
            && self.arg.get_id() != other.arg.get_id()
            && (exclusive
                || self.conflicts.contains(&other.arg.get_id())
                || other.conflicts.contains(&self.arg.get_id()))
    }

    /// Whether the two options are the two sides of a `--x`/`--no-x` pair.
    fn negates(&self, other: &Self) -> bool {
        let (Some(long), Some(other_long)) = (self.arg.get_long(), other.arg.get_long()) else {
            return false;
        };
        self.meets(other)
            && (long.strip_prefix("no-") == Some(other_long)
                || other_long.strip_prefix("no-") == Some(long))
    }
}

/// Unbind each listed option at its depth along the subcommand path.
fn unbind(command: Command, path: &[&str], depth: usize, unset: &[(usize, Id)]) -> Command {
    let command = command.mut_args(|arg| {
        if unset
            .iter()
            .any(|(at, id)| *at == depth && id == arg.get_id())
        {
            arg.env(None::<&'static str>)
        } else {
            arg
        }
    });
    match path.split_first() {
        Some((name, rest)) => command.mut_subcommands(|sub| {
            if sub.get_name() == *name {
                unbind(sub, rest, depth + 1, unset)
            } else {
                sub
            }
        }),
        None => command,
    }
}

#[cfg(test)]
mod tests {
    use clap::{Args, CommandFactory, Parser, Subcommand};

    use super::{EnvPrefix, env_name};

    #[derive(Parser)]
    #[command(env_prefix = "TOYCTL_UNIT")]
    struct Toy {
        #[arg(long, global = true)]
        format: Option<String>,
        #[arg(long)]
        no_color: bool,
        #[arg(long, hide = true)]
        secret: bool,
        #[arg(long, env = "OWN_NAME")]
        token: Option<String>,
        #[command(subcommand)]
        command: Cmd,
    }

    #[derive(Subcommand)]
    enum Cmd {
        Prepare(PrepareArgs),
        Group {
            #[command(subcommand)]
            command: Nested,
        },
    }

    #[derive(Args)]
    struct PrepareArgs {
        #[arg(short = 'c', long)]
        config: Option<String>,
        path: Option<String>,
    }

    #[derive(Subcommand)]
    enum Nested {
        Show {
            #[arg(long)]
            dry_run: bool,
        },
    }

    fn env_of(command: &clap::Command, id: &str) -> Option<String> {
        command
            .get_arguments()
            .find(|arg| arg.get_id() == id)
            .and_then(|arg| arg.get_env())
            .map(|name| name.to_string_lossy().into_owned())
    }

    #[test]
    fn a_long_name_becomes_the_prefixed_variable() {
        assert_eq!(env_name("VERCTL", "dry-run"), "VERCTL_DRY_RUN");
        assert_eq!(env_name("QCTL", "config"), "QCTL_CONFIG");
    }

    #[test]
    fn every_eligible_long_option_is_bound_at_every_depth() {
        let command = Toy::command();
        assert_eq!(
            env_of(&command, "format").as_deref(),
            Some("TOYCTL_UNIT_FORMAT")
        );
        let prepare = command.find_subcommand("prepare").unwrap();
        assert_eq!(
            env_of(prepare, "config").as_deref(),
            Some("TOYCTL_UNIT_CONFIG")
        );
        let show = command
            .find_subcommand("group")
            .and_then(|group| group.find_subcommand("show"))
            .unwrap();
        assert_eq!(
            env_of(show, "dry_run").as_deref(),
            Some("TOYCTL_UNIT_DRY_RUN")
        );
    }

    #[test]
    fn negations_hidden_positional_and_declared_options_keep_their_shape() {
        let command = Toy::command();
        assert_eq!(env_of(&command, "no_color"), None);
        assert_eq!(env_of(&command, "secret"), None);
        assert_eq!(env_of(&command, "token").as_deref(), Some("OWN_NAME"));
        let prepare = command.find_subcommand("prepare").unwrap();
        assert_eq!(env_of(prepare, "path"), None);
    }

    #[test]
    fn a_command_without_the_attribute_binds_nothing() {
        let command = clap::Command::new("bare").arg(clap::Arg::new("format").long("format"));
        assert_eq!(env_of(&command, "format"), None);
        let bound = command.env_prefix("BARE");
        assert_eq!(env_of(&bound, "format").as_deref(), Some("BARE_FORMAT"));
    }
}
