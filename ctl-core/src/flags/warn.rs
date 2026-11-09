//! Warn on repeated or contradictory shared flags. The last value still wins.

use std::fmt;
use std::io::Write as _;

use crate::{formatdoc, writedoc};

/// The type of clash between flags that a [`FlagWarning`] reports. The last
/// value still wins in every case.
///
/// The set is closed, and the enum is deliberately not `#[non_exhaustive]`.
/// Two flags given together are the same flag, two names of one flag, or
/// opposites; no other relation between them exists.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FlagClash {
    /// Same flag twice (`--format json --format pretty`).
    Repeated,
    /// Opposite flags (`--color always --no-color`).
    Contradictory,
    /// Aliases of the same flag (`--dry-run --preview`).
    Redundant,
}

/// One flag clash. Its `Display` text is the sentence a warning line holds
/// after the `{bin}: warning: ` prefix.
///
/// [`chassis_warnings`] builds them. A field can join the struct in a minor
/// release.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct FlagWarning {
    /// The type of clash.
    pub clash: FlagClash,
    /// The flags involved, spelled as in a warning (`--color`, `--no-color`).
    pub names: Vec<String>,
}

impl FlagWarning {
    /// The full warning line for stderr, `{bin}: warning: …`, without a
    /// trailing newline.
    #[must_use]
    pub fn line(&self, bin: &str) -> String {
        formatdoc!("{bin}: warning: {self}")
    }
}

impl fmt::Display for FlagWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let names = match self.clash {
            FlagClash::Repeated => self.names.join("/"),
            FlagClash::Contradictory | FlagClash::Redundant => self.names.join(" and "),
        };
        match self.clash {
            FlagClash::Repeated => {
                writedoc!(f, "{names} repeated; last value wins")
            }
            FlagClash::Contradictory => {
                writedoc!(f, "{names} both set; last wins")
            }
            FlagClash::Redundant => {
                writedoc!(f, "{names} are the same flag")
            }
        }
    }
}

/// Find clashes among ctl-core's shared flags in `args`.
///
/// `args` excludes the program name. Scanning stops at `--`. The result lists
/// a repeated `--color`, `--format`, `--quiet`, or `--no-color`, a `--color`
/// with `--no-color`, and a repeated or aliased `--dry-run`.
#[must_use]
pub fn chassis_warnings<'a, I>(args: I) -> Vec<FlagWarning>
where
    I: IntoIterator<Item = &'a str>,
{
    let seen = collect(args);
    let mut out = Vec::new();
    push_repeat(&mut out, &seen.color, "--color");
    push_repeat(&mut out, &seen.format, "--format");
    push_repeat(&mut out, &seen.quiet, "--quiet");
    push_repeat(&mut out, &seen.color_off, "--no-color");
    if !seen.color.is_empty() && !seen.color_off.is_empty() {
        out.push(FlagWarning {
            clash: FlagClash::Contradictory,
            names: vec!["--color".into(), "--no-color".into()],
        });
    }
    push_dry(&mut out, &seen.dry);
    out
}

/// Find a `--foo` / `--no-foo` pair given together in `args`.
///
/// `yes` and `no` list the spellings of each side. The result holds one
/// [`FlagClash::Contradictory`] warning, naming the first spelling of each
/// side, or is empty. Scanning stops at `--`. Pair it with Clap's
/// `overrides_with` on both sides, so the last flag wins and the clash is not
/// silent.
#[must_use]
pub fn warn_opposites<'a, I>(args: I, yes: &[&str], no: &[&str]) -> Vec<FlagWarning>
where
    I: IntoIterator<Item = &'a str>,
{
    let mut saw_yes = false;
    let mut saw_no = false;
    for arg in args {
        if arg == "--" {
            break;
        }
        if matches_any(arg, yes) {
            saw_yes = true;
        }
        if matches_any(arg, no) {
            saw_no = true;
        }
    }
    if saw_yes && saw_no {
        vec![FlagWarning {
            clash: FlagClash::Contradictory,
            names: vec![yes[0].to_owned(), no[0].to_owned()],
        }]
    } else {
        Vec::new()
    }
}

/// Print each warning to stderr as `{bin}: warning: …`, one per line.
pub fn emit_warnings<'a, I>(bin: &str, warnings: I)
where
    I: IntoIterator<Item = &'a FlagWarning>,
{
    for warning in warnings {
        let _ = writeln!(std::io::stderr().lock(), "{}", warning.line(bin));
    }
}

#[derive(Default)]
struct Seen {
    color: Vec<String>,
    color_off: Vec<String>,
    format: Vec<String>,
    quiet: Vec<String>,
    dry: Vec<String>,
}

fn collect<'a, I>(args: I) -> Seen
where
    I: IntoIterator<Item = &'a str>,
{
    let mut seen = Seen::default();
    let mut args = args.into_iter().peekable();
    while let Some(arg) = args.next() {
        if arg == "--" {
            break;
        }
        if arg == "--no-color" {
            seen.color_off.push(arg.to_owned());
            continue;
        }
        if matches_any(arg, &["-q", "--quiet"]) {
            seen.quiet.push("--quiet".into());
            continue;
        }
        if matches_any(arg, &["-n", "--dry-run", "--preview"]) {
            seen.dry.push(arg.to_owned());
            continue;
        }
        if take_value(&mut args, arg, &["-c", "--color"]) {
            seen.color.push("--color".into());
            continue;
        }
        if take_value(&mut args, arg, &["-f", "--format"]) {
            seen.format.push("--format".into());
        }
    }
    seen
}

fn take_value<'a, I>(args: &mut std::iter::Peekable<I>, arg: &str, names: &[&str]) -> bool
where
    I: Iterator<Item = &'a str>,
{
    if names.contains(&arg) {
        if matches!(args.peek(), Some(next) if !next.starts_with('-')) {
            args.next();
        }
        return true;
    }
    names.iter().any(|name| {
        name.starts_with("--")
            && arg
                .strip_prefix(name)
                .is_some_and(|rest| rest.starts_with('='))
    })
}

fn matches_any(arg: &str, names: &[&str]) -> bool {
    names.contains(&arg)
}

fn push_repeat(out: &mut Vec<FlagWarning>, hits: &[String], name: &str) {
    if hits.len() > 1 {
        out.push(repeat(name));
    }
}

fn push_dry(out: &mut Vec<FlagWarning>, hits: &[String]) {
    if hits.len() < 2 {
        return;
    }
    let aliases = hits.iter().any(|hit| hit == "--preview")
        && hits.iter().any(|hit| hit == "--dry-run" || hit == "-n");
    if aliases {
        out.push(FlagWarning {
            clash: FlagClash::Redundant,
            names: vec!["--dry-run".into(), "--preview".into()],
        });
    } else {
        out.push(repeat("--dry-run"));
    }
}

fn repeat(name: &str) -> FlagWarning {
    FlagWarning {
        clash: FlagClash::Repeated,
        names: vec![name.to_owned()],
    }
}

#[cfg(test)]
mod tests {
    use super::{FlagClash, FlagWarning, chassis_warnings, warn_opposites};

    fn clashes(args: &[&str]) -> Vec<FlagClash> {
        chassis_warnings(args.iter().copied())
            .into_iter()
            .map(|warning| warning.clash)
            .collect()
    }

    #[test]
    fn equals_form_is_an_occurrence() {
        assert_eq!(
            clashes(&["--format=json", "--format=pretty"]),
            [FlagClash::Repeated]
        );
        assert_eq!(
            clashes(&["--color=always", "--color=never"]),
            [FlagClash::Repeated]
        );
    }

    #[test]
    fn peek_does_not_eat_the_following_flag() {
        let hits = clashes(&["--color", "--no-color"]);
        assert!(hits.contains(&FlagClash::Contradictory), "{hits:?}");
    }

    #[test]
    fn stops_at_double_dash() {
        assert_eq!(
            clashes(&["--format", "json", "--", "--format", "pretty"]),
            Vec::<FlagClash>::new()
        );
        assert_eq!(
            warn_opposites(["--pr", "--", "--no-pr"], &["--pr"], &["--no-pr"]),
            Vec::<FlagWarning>::new()
        );
    }

    #[test]
    fn repeated_no_color() {
        assert_eq!(
            clashes(&["--no-color", "--no-color"]),
            [FlagClash::Repeated]
        );
    }

    #[test]
    fn short_and_long_color_is_repeated() {
        assert_eq!(
            clashes(&["-c", "always", "--color", "never"]),
            [FlagClash::Repeated]
        );
    }
}
