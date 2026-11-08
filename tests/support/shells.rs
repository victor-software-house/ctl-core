//! Runs a rendered shell script through its own shell's syntax check, and a
//! bash script through shellcheck too, so a snapshot that the shell cannot
//! read fails its test.
//!
//! The shells and shellcheck are lazy tools in `mise.toml`, which the test
//! task installs first, through `install:test-tools`. A test that cannot start
//! one fails and names the tool: a check that skips itself when a tool is
//! missing would pass without checking anything.

use std::path::Path;
use std::process::Command;

use ctl_core::fixture::Suite;
use ctl_core::shell::Shell;

/// Check `script` as `shell` reads it. `exclude` names the shellcheck codes
/// the bash script may break, each for a reason its caller states; the other
/// shells ignore it.
pub(crate) fn check(shell: Shell, script: &str, exclude: &[&str]) {
    let suite = Suite::new("shell-script");
    match shell {
        Shell::Bash => {
            let path = suite.root().file("script.bash", script);
            run("bash", &["-n"], &path);
            let excluded = format!("--exclude={}", exclude.join(","));
            let mut args = vec!["--shell=bash", "--enable=all"];
            if !exclude.is_empty() {
                args.push(&excluded);
            }
            run("shellcheck", &args, &path);
        }
        Shell::Zsh => run(
            "zsh",
            &["-f", "-n"],
            &suite.root().file("script.zsh", script),
        ),
        Shell::Fish => run(
            "fish",
            &["--no-config", "-n"],
            &suite.root().file("script.fish", script),
        ),
    }
}

/// Run `program` on `path` and fail with its output unless it exits 0.
fn run(program: &str, args: &[&str], path: &Path) {
    let output = Command::new(program)
        .args(args)
        .arg(path)
        .output()
        .unwrap_or_else(|error| {
            panic!("start {program}, a tool `mise run test` installs first: {error}")
        });
    assert!(
        output.status.success(),
        "{program} {args:?} rejected the script:\n{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}
