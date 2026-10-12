//! The served Rust baseline: ctl-core matches its own render, and what a
//! repository declares reaches its files.
#![cfg(all(feature = "baseline", feature = "fixture"))]

use std::path::Path;

use ctl_core::baseline::{self, Config, File, REPO_FILE, Rendered};
use ctl_core::fixture::{Root, Suite};
use ctl_core::indoc;
use ctl_core::input::Input;

const CARGO: &str = indoc! {r#"
    [package]
    name = "demo"
    version = "1.2.3"

    # ctl-core baseline: begin lints
    [lints.rust]
    stale = "deny"
    # ctl-core baseline: end lints

    [dependencies]
    serde = "1"
"#};

const MISE_DEV: &str = indoc! {r#"
    min_version = "2026.10.3"

    # ctl-core baseline: begin tools
    [tools]
    # ctl-core baseline: end tools
    "github:demo/extra" = "1.0.0"
"#};

/// A repository with `rust_yaml` and both block files.
fn repo<'suite>(suite: &'suite Suite, name: &str, rust_yaml: &str) -> Root<'suite> {
    let root = suite.scenario(name);
    root.file(REPO_FILE, rust_yaml);
    root.file("Cargo.toml", CARGO);
    root.file("mise.dev.toml", MISE_DEV);
    root
}

/// The rendered body of `name`, if it was served.
fn body(rendered: &Rendered, name: &str) -> Option<String> {
    rendered
        .served
        .iter()
        .find(|served| served.target() == Path::new(name))
        .map(|served| served.body().to_owned())
}

#[test]
fn ctl_core_matches_its_own_baseline() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let config = baseline::load(root).unwrap();
    let rendered = baseline::render(root, &config).unwrap();
    assert_eq!(rendered.served.len(), 7);
    let drifted: Vec<String> = rendered
        .served
        .iter()
        .filter(|served| {
            std::fs::read_to_string(root.join(served.target()))
                .ok()
                .as_deref()
                != Some(served.body())
        })
        .map(|served| served.target().display().to_string())
        .collect();
    assert!(drifted.is_empty(), "differs from its render: {drifted:?}");
}

#[test]
fn a_published_crate_gets_every_file_and_the_semver_tool() {
    let suite = Suite::new("baseline-crate");
    let root = repo(&suite, "crate", "published: true\n");
    let config = baseline::load(root.path()).unwrap();
    let rendered = baseline::render(root.path(), &config).unwrap();
    assert_eq!(rendered.served.len(), 7);
    assert!(
        body(&rendered, "rust-toolchain.toml")
            .unwrap()
            .contains("channel = \"1.99.0\"")
    );
    assert!(
        body(&rendered, "mise.dev.toml")
            .unwrap()
            .contains("cargo-semver-checks")
    );

    let unpublished = repo(&suite, "unpublished", "published: false\n");
    let config = baseline::load(unpublished.path()).unwrap();
    let rendered = baseline::render(unpublished.path(), &config).unwrap();
    assert!(
        !body(&rendered, "mise.dev.toml")
            .unwrap()
            .contains("cargo-semver-checks")
    );
}

#[test]
fn a_tokio_crate_gets_the_lock_guards_and_the_channel_ban() {
    let suite = Suite::new("baseline-tokio");
    let root = repo(&suite, "tokio", "tokio: true\n");
    let config = baseline::load(root.path()).unwrap();
    let clippy = body(
        &baseline::render(root.path(), &config).unwrap(),
        "clippy.toml",
    )
    .unwrap();
    assert!(clippy.contains("\"tokio::sync::MutexGuard\","), "{clippy}");
    assert!(
        clippy.contains(
            "{ path = \"tokio::sync::mpsc::unbounded_channel\", reason = \"no backpressure; use a bounded channel\" },"
        ),
        "{clippy}"
    );
}

#[test]
fn an_appended_entry_reaches_its_file_with_its_reason() {
    let suite = Suite::new("baseline-append");
    let root = repo(
        &suite,
        "append",
        indoc! {"
            deny:
              licenses:
                - license: BSD-3-Clause
                  reason: encoding_rs needs it
              bans:
                - crate: reqwest
                  wrappers: [http]
                  reason: one crate owns HTTP
        "},
    );
    let config = baseline::load(root.path()).unwrap();
    let deny = body(
        &baseline::render(root.path(), &config).unwrap(),
        "deny.toml",
    )
    .unwrap();
    assert!(
        deny.contains("    # encoding_rs needs it\n    \"BSD-3-Clause\",\n"),
        "{deny}"
    );
    assert!(
        deny.contains(
            "{ crate = \"reqwest\", wrappers = [\"http\"], reason = \"one crate owns HTTP\" },"
        ),
        "{deny}"
    );
}

#[test]
fn an_unknown_field_is_refused_by_name() {
    let error = Input::new("rust.yaml", "layot: crate\n")
        .load::<Config>()
        .unwrap_err();
    assert!(error.problems[0].message.contains("layot"), "{error}");
}

#[test]
fn an_entry_without_a_reason_is_refused_on_its_line() {
    let text = indoc! {"
        deny:
          bans:
            - crate: git2
    "};
    let error = Input::new("rust.yaml", text).load::<Config>().unwrap_err();
    let problem = &error.problems[0];
    assert!(problem.message.contains("reason"), "{error}");
    assert!(problem.line.is_some(), "{error}");
}

#[test]
fn a_quote_in_a_reason_is_refused_by_field() {
    let text = indoc! {r#"
        clippy:
          disallowed_methods:
            - path: std::env::var
              reason: say "no"
    "#};
    let error = Input::new("rust.yaml", text).load::<Config>().unwrap_err();
    assert_eq!(
        error.problems[0].field, "clippy.disallowed_methods[0].reason",
        "{error}"
    );
}

#[test]
fn a_block_is_replaced_and_the_rest_of_the_file_kept() {
    let suite = Suite::new("baseline-block");
    let root = repo(&suite, "block", "{}\n");
    let cargo = body(
        &baseline::render(root.path(), &Config::default()).unwrap(),
        "Cargo.toml",
    )
    .unwrap();
    assert!(
        cargo.starts_with(indoc! {r#"
            [package]
            name = "demo"
            version = "1.2.3"

            # ctl-core baseline: begin lints
            [lints.rust]
            unsafe_code = "forbid"
        "#}),
        "{cargo}"
    );
    assert!(!cargo.contains("stale"), "{cargo}");
    assert!(
        cargo.ends_with(indoc! {r#"
            # ctl-core baseline: end lints

            [dependencies]
            serde = "1"
        "#}),
        "{cargo}"
    );
}

#[test]
fn a_workspace_gets_workspace_lints() {
    let suite = Suite::new("baseline-workspace");
    let root = repo(&suite, "workspace", "layout: workspace\n");
    let config = baseline::load(root.path()).unwrap();
    let cargo = body(
        &baseline::render(root.path(), &config).unwrap(),
        "Cargo.toml",
    )
    .unwrap();
    assert!(cargo.contains("[workspace.lints.rust]\n"), "{cargo}");
    assert!(cargo.contains("[workspace.lints.rustdoc]\n"), "{cargo}");
    assert!(cargo.contains("[workspace.lints.clippy]\n"), "{cargo}");
    assert!(!cargo.contains("\n[lints."), "{cargo}");
}

#[test]
fn a_file_without_its_markers_names_the_file_and_the_marker() {
    let suite = Suite::new("baseline-markers");
    let root = repo(&suite, "markers", "{}\n");
    root.file("mise.dev.toml", "[tools]\n");
    let error = baseline::render(root.path(), &Config::default()).unwrap_err();
    assert_eq!(error.file, "mise.dev.toml");
    assert!(
        error.message.contains("# ctl-core baseline: begin tools"),
        "{error}"
    );
}

#[test]
fn a_marker_is_a_whole_line_and_may_end_in_crlf() {
    let suite = Suite::new("baseline-marker-lines");
    let root = repo(&suite, "lines", "{}\n");
    root.file(
        "Cargo.toml",
        "[package]\r\n# ctl-core baseline: begin lints\r\nstale = 1\r\n# ctl-core baseline: end lints\r\n",
    );
    let cargo = body(
        &baseline::render(root.path(), &Config::default()).unwrap(),
        "Cargo.toml",
    )
    .unwrap();
    assert!(cargo.contains("[lints.rust]\n"), "{cargo}");
    assert!(!cargo.contains("stale"), "{cargo}");

    root.file(
        "mise.dev.toml",
        "# ctl-core baseline: begin tools\n# ctl-core baseline: end toolsX\n",
    );
    let error = baseline::render(root.path(), &Config::default()).unwrap_err();
    assert_eq!(error.file, "mise.dev.toml");
    assert!(
        error
            .message
            .contains("has no `# ctl-core baseline: end tools` line"),
        "{error}"
    );
}

#[test]
fn an_owned_file_is_skipped_and_reported() {
    let suite = Suite::new("baseline-owned");
    let root = repo(
        &suite,
        "owned",
        indoc! {"
            own:
              - file: rustfmt.toml
                reason: this repository formats on stable
        "},
    );
    let config = baseline::load(root.path()).unwrap();
    let rendered = baseline::render(root.path(), &config).unwrap();
    assert_eq!(body(&rendered, "rustfmt.toml"), None);
    assert_eq!(rendered.served.len(), 6);
    assert_eq!(rendered.owned.len(), 1);
    assert_eq!(rendered.owned[0].file, File::Rustfmt);
}
