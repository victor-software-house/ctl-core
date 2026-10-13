//! A served CLI-proxy task keeps piped output byte for byte what the CLI wrote,
//! even when the caller forces colour.
#![allow(
    missing_docs,
    reason = "an integration test crate has no public API to document"
)]
#![cfg(all(unix, feature = "usage", feature = "fixture"))]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

use ctl_core::fixture::Suite;
use ctl_core::{formatdoc, indoc, mount_directives};

/// mise relays a task's output itself unless the task is `raw`, and appends an
/// SGR reset to every relayed line whenever its own colour is on.
/// `CLICOLOR_FORCE=1` turns that on into a pipe. The task runs in a scrubbed
/// environment, so neither the caller's mise configuration nor its colour
/// settings reach it.
#[test]
fn a_mounted_task_pipes_json_without_an_escape_byte() {
    let suite = Suite::new("mount-raw");
    let root = suite.root();
    root.file(
        "mise.toml",
        indoc! {r#"
            [task_config]
            includes = ["tasks"]
        "#},
    );
    let task = root.file(
        "tasks/demo",
        formatdoc! {r#"
            #!/usr/bin/env bash
            #MISE description="Demo CLI proxy"
            {directives}

            printf '{{"ok":true}}\n'
        "#, directives = mount_directives("demo")},
    );
    fs::set_permissions(&task, fs::Permissions::from_mode(0o755))
        .unwrap_or_else(|error| panic!("chmod the task: {error}"));
    let output = Command::new("mise")
        .args(["run", "demo"])
        .current_dir(root.path())
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("HOME", std::env::var_os("HOME").unwrap_or_default())
        .env("MISE_CONFIG_DIR", root.path().join("mise-config"))
        .env("MISE_TRUSTED_CONFIG_PATHS", root.path())
        .env("CLICOLOR_FORCE", "1")
        .env("FORCE_COLOR", "3")
        .env("COLORTERM", "truecolor")
        .output()
        .unwrap_or_else(|error| panic!("this test needs mise on PATH: {error}"));
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "{\"ok\":true}\n",
        "stdout must be exactly what the task printed"
    );
}
