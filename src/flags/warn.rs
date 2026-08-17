//! Warn on duplicate / contradictory chassis flags. Last value still wins.

use std::fmt;

use crate::{formatdoc, writedoc};

/// How two (or more) flags fight.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WarningKind {
    /// Same flag twice (`--format json --format pretty`).
    Repeated,
    /// Opposite flags (`--color always --no-color`).
    Contradictory,
    /// Aliases of the same flag (`--dry-run --preview`).
    Redundant,
}

/// One warning. Display is the stderr line after `{bin}: warning: `.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlagWarning {
    /// Kind of clash.
    pub kind: WarningKind,
    /// Tokens as written (`--color`, `--no-color`).
    pub names: Vec<String>,
}

impl FlagWarning {
    /// `{bin}: warning: …` for stderr.
    #[must_use]
    pub fn line(&self, bin: &str) -> String {
        formatdoc!("{bin}: warning: {self}")
    }
}

impl fmt::Display for FlagWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let names = match self.kind {
            WarningKind::Repeated => self.names.join("/"),
            WarningKind::Contradictory | WarningKind::Redundant => self.names.join(" and "),
        };
        match self.kind {
            WarningKind::Repeated => {
                writedoc!(f, "{names} repeated; last value wins")
            }
            WarningKind::Contradictory => {
                writedoc!(f, "{names} both set; last wins")
            }
            WarningKind::Redundant => {
                writedoc!(f, "{names} are the same flag")
            }
        }
    }
}

/// Chassis flags known to ctl-core.
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
            kind: WarningKind::Contradictory,
            names: vec!["--color".into(), "--no-color".into()],
        });
    }
    push_dry(&mut out, &seen.dry);
    out
}

/// `--foo` / `--no-foo` both present.
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
            kind: WarningKind::Contradictory,
            names: vec![yes[0].to_owned(), no[0].to_owned()],
        }]
    } else {
        Vec::new()
    }
}

/// Print `{bin}: warning: …` for each finding.
pub fn emit_warnings<'a, I>(bin: &str, warnings: I)
where
    I: IntoIterator<Item = &'a FlagWarning>,
{
    for warning in warnings {
        eprintln!("{}", warning.line(bin));
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
            kind: WarningKind::Redundant,
            names: vec!["--dry-run".into(), "--preview".into()],
        });
    } else {
        out.push(repeat("--dry-run"));
    }
}

fn repeat(name: &str) -> FlagWarning {
    FlagWarning {
        kind: WarningKind::Repeated,
        names: vec![name.to_owned()],
    }
}

#[cfg(test)]
mod tests {
    use super::{FlagWarning, WarningKind, chassis_warnings, warn_opposites};

    fn kinds(args: &[&str]) -> Vec<WarningKind> {
        chassis_warnings(args.iter().copied())
            .into_iter()
            .map(|warning| warning.kind)
            .collect()
    }

    #[test]
    fn equals_form_is_an_occurrence() {
        assert_eq!(
            kinds(&["--format=json", "--format=pretty"]),
            [WarningKind::Repeated]
        );
        assert_eq!(
            kinds(&["--color=always", "--color=never"]),
            [WarningKind::Repeated]
        );
    }

    #[test]
    fn peek_does_not_eat_the_following_flag() {
        let hits = kinds(&["--color", "--no-color"]);
        assert!(hits.contains(&WarningKind::Contradictory), "{hits:?}");
    }

    #[test]
    fn stops_at_double_dash() {
        assert_eq!(
            kinds(&["--format", "json", "--", "--format", "pretty"]),
            Vec::<WarningKind>::new()
        );
        assert_eq!(
            warn_opposites(["--pr", "--", "--no-pr"], &["--pr"], &["--no-pr"]),
            Vec::<FlagWarning>::new()
        );
    }

    #[test]
    fn repeated_no_color() {
        assert_eq!(
            kinds(&["--no-color", "--no-color"]),
            [WarningKind::Repeated]
        );
    }

    #[test]
    fn short_and_long_color_is_repeated() {
        assert_eq!(
            kinds(&["-c", "always", "--color", "never"]),
            [WarningKind::Repeated]
        );
    }
}
