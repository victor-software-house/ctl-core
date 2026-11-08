# ctl-core workspace

The shared Rust CLI chassis for the `*ctl` CLIs (`verctl`, `qctl`, `forkctl`)
and `state-sync`, and the terminal render engine it draws with. Every crate is
a library; none is a command.

This repository's queue is [`tasks.yaml`](tasks.yaml) (`CTC-###`). This file
holds the rules for the whole workspace. Each crate's own `AGENTS.md` holds
what that crate owns, what it must not own, its public-API rules, and where
its tests live:

| Crate | Guide | Owns |
|:--|:--|:--|
| `ctl-text` | [`ctl-text/AGENTS.md`](ctl-text/AGENTS.md) | Styled text, cell widths, wrapping, and the `Renderable` protocol. No terminal I/O |
| `ctl-layout` | [`ctl-layout/AGENTS.md`](ctl-layout/AGENTS.md) | Tables, records, rules, stacks, and indents on `ctl-text`. Never writes an escape byte |
| `ctl-term` | [`ctl-term/AGENTS.md`](ctl-term/AGENTS.md) | Colour policy, depth, and width per stream, and the escape encoder |
| `ctl-core` | [`ctl-core/AGENTS.md`](ctl-core/AGENTS.md) | The CLI chassis: flags, `App`, help, `Document`, `View`, and the JSON envelope |

Read the guide of every crate a change touches before changing it.

## Workspace

The repository is a virtual Cargo workspace (`layout: workspace` in the
Copier answers). The root `Cargo.toml` holds the members, one version for
every crate in `[workspace.package]`, `[workspace.dependencies]`, and
`[workspace.lints]`. Each crate lives in a root folder named like its
package and inherits those tables with `workspace = true`. Profiles stay in
the root manifest, where Cargo reads them.

The crates depend in one direction: `ctl-layout` and `ctl-term` on
`ctl-text`, and `ctl-core` on all three. `ctl-layout` and `ctl-term` never
depend on each other. A rule changes in the crate that owns it, never in a
crate above it.

ctl-core requires each engine crate at exactly the workspace version
(`version = "=x.y.z"` in `[workspace.dependencies]`), and a `pins` entry in
`.ctl/ver.yaml` moves each requirement with the release. Keep the two in step
when a crate is renamed or added. `.ctl/ver.yaml` points at the virtual
manifest, so verctl bumps `workspace.package.version` and publishes each
member in dependency order: `ctl-text`, `ctl-layout`, `ctl-term`, then
`ctl-core`.

For a workspace the template drops the library-only `print_stdout` and
`print_stderr` lints and clippy's `allow-print-in-tests`; they stay here as
local edits, because every member is a library. `check:features` passes
`--package ctl-core`, since Cargo refuses `--features` at the root of a
virtual workspace.

Each crate's `README.md` is its crates.io page, named by `readme` in its
`Cargo.toml`, and its example runs as a doctest through a `ReadmeDoctests`
item in `src/lib.rs`. The root `README.md` is the workspace's GitHub page.
Each crate's `//!` docs are its docs.rs page.

## Strings

Multiline Rust is `indoc!` / `formatdoc!` / `writedoc!` / `printdoc!` /
`eprintdoc!` / `concatdoc!` (re-exported from this crate). **No `concat!`.**
**No escaped `\n` in a document.** Leave a raw `\n` only when that *is* the
test.

## Comments

Doc comments carry the why, on the item. Inline `//` prose inside a function
body is litter.

## Copier template

The repository setup comes from the shared Copier template, the private
`victor-software-house/templates` repository (project `rust-crate`): the whole
files `.miserc.toml`, `.envrc`, `mise.dev.toml`, `mise.ci.toml`,
`lefthook.yml`, `committed.toml`, `rust-toolchain.toml`, `rustfmt.toml`,
`clippy.toml`, `deny.toml`, `.rumdl.toml`, `.taplo.toml`, `.typos.toml`,
`.github/workflows/ci.yml`, `.github/workflows/template.yml`,
`.github/actionlint.yaml`,
the path-named file tasks in `mise-tasks/` (`check/repo.sh`,
`commit-msg.sh`, `commits.sh`, `specs.sh`, `format/`, `lint/`, and
`hooks/secrets/`), plus the settings, toolchain, and `[task_config]` blocks
of `mise.toml` and the `[workspace.lints]` tables of the root `Cargo.toml`. The starters
`mise-tasks/check/local.sh` and `openspec/config.yaml` are this repository's
own, and so is `mise-tasks/check/features.sh`, which holds this crate's
feature sets.
Local edits survive an update: `.rumdl.toml` allows the README banner's
HTML, and `.typos.toml` ignores commit hashes cited in prose. Change a shared rule in the template, then take
its new tag here.
`.copier-answers.yml` records its tag and this repository's answers; never
edit it by hand. Take a new tag with
`mise x uv@0.13.1 python@3.14.8 -- uvx copier@<pin> update --defaults --vcs-ref <tag>`,
where `<pin>` is the `"pypi:copier"` version in the template's `mise.toml` at
that tag. Review the diff, and fix any conflict markers (`git diff --check`)
before committing.

`ci.yml` calls the shared `rust-ci.yml`, and `template.yml` calls the shared
`template-check.yml`, both from the public `victor-software-house/actions`
repository at the commit the template pins. Their checks report as
`verify / Verify` and `template / Template`. The drift check runs
`template:check`, which fails only a pull request that changes
`.copier-answers.yml` without reaching the newest tag; any other pull request
gets a notice. A push to `main` and a daily run keep one issue, "Template
update available", open while the newest tag would change this repository.
The drift check is skipped for pull requests from forks.

Local work loads the `dev` and `test` profiles (`.miserc.toml`). `dev`
holds lefthook, committed, and gitleaks; `test` holds the tools the script
tests start (see [Snapshots](ctl-core/AGENTS.md#snapshots)). Every hook command runs with
`MISE_ENV=dev,hooks`, except the pre-push lock check, which runs with
`MISE_ENV=dev,test`, the list `.miserc.toml` loads. The `postinstall` hook of
`dev` runs `lefthook install` after every `mise install`, so a fresh clone
gets its git hooks from `mise install`. CI sets `MISE_ENV=ci` (`ci,test` for
the Build and test job, `ci,release` for release jobs) and never loads `dev`,
so no runner installs hooks.

## Release

Human writes `.changeset/*.md` on the same PR that ships the behaviour. Never
hand-edit versions or CHANGELOG. Declarations live in [`.ctl/ver.yaml`](.ctl/ver.yaml).
`prepare` runs `cargo update --workspace` and stages `Cargo.lock` because CI
packages with `--locked`. `.github/workflows/release.yml` calls verctl's
reusable release workflow on every push to `main`: it publishes each version
commit not yet fully released, then opens or updates the Version PR on
`verctl/release/main`. Only the operator merges the Version PR, so no release
happens on its own. Bump its pin together with the verctl
pin in `mise.release.toml` and the `ver` task include in `mise.toml`.

## Checks

```sh
mise run verify
```

`verify` is the toolchain pins, format, clippy, the feature sets, `test` (unit
and integration; targets with `test = false` are skipped), `test:doc`
(doc-tests), `doc` (rustdoc as docs.rs builds it), cargo-deny
licenses/bans/sources, cargo-machete, and `check:versions`, verctl's
hand-edit guard. Those tasks come from the actions repository's
`tasks/rust.toml`, which `mise.toml` includes by `git::` URL at the commit
the template pins, and `mise.toml` composes `verify` from them. This
repository's own tasks live in `mise-tasks/`:
`check:features` is the file task `mise-tasks/check/features.sh`. It denies
warnings, so an item gated on more features than use it fails the set that
compiles it unused.
`check:tree` fails when the `app` tier's normal dependency tree holds a
terminal crate the render engine replaced: comfy-table, crossterm, anstream,
anstyle-parse, or console.
`check:size` builds the `size` example at each feature tier with the release
profile and fails when a tier's `.text` grows more than the tolerance past
its budget in `[workspace.metadata.size]` of the root `Cargo.toml`. The
budgets are linux-x64 numbers, so the task runs on Linux x86-64 and skips
elsewhere. A change that grows a tier on purpose raises its budget in the
same pull request, measured with the pinned rustc.
`lint:prose` runs Vale with the prose-style package over the files
`.vale.ini` picks: `README.md`, `AGENTS.md`, `docs/` outside
`docs/research/`, every comment in the Rust files under each `src/`, and
the changesets.
`vale sync` fetches the package into the ignored `.vale/`, so the first run
needs the network.
`check:repo`, `commit-msg`, `commits`, and the secrets scans are the
template's files in `mise-tasks/`. `check:repo` runs taplo, rumdl, typos,
actionlint, and OpenSpec validation, then `check:local`, which
`mise-tasks/check/local.sh` leaves to this repository; ctl-core adds none.
`[task_config]` lists `mise-tasks` first, because an includes list replaces the
default task dirs. Fragment names are not namespaced by file, so each name
carries its prefix. `verify` depends on `check:features` and `check:repo` by name, so every
repository on the template defines `check:features` and takes `check:repo`
from the template. Change a shared task in the actions repository, never in
a consumer.
`check:versions` and `semver` run verctl through its served `ver` task, so
every including repository that declares releases in `.ctl/ver.yaml` (or
`VERCTL_CONFIG`) includes that task at a pinned verctl release in the
environment `verify` runs in, locks verctl there, and gives the CI Verify
step `GITHUB_TOKEN` with `pull-requests: read` (template v0.0.27 renders
both). The check reads the pull request only when a manifest version differs.
A repository without that file declares no releases, so `check:versions`
passes there with nothing to compare.
`lint`, `test`, and `test:doc` pass `--workspace`, so every member is checked,
not only the root package or the `default-members` list.
Every cargo command that resolves dependencies passes `--locked`, in the
shared tasks and in each repository's `check:features`, so a stale `Cargo.lock`
fails `verify` the way it fails a release build. cargo-machete has no
`--locked`, so `deps` runs a locked `cargo metadata` first: no task in the
parallel `verify` ever rewrites the lock before a sibling reads it.
Tests run under plain `cargo test`. Measured on Linux and macOS with doc-tests
included, cargo-nextest was 1.05 to 14 times slower (it starts one process per
test), and mbx test scheduling or a `RUST_TEST_THREADS` cap slowed concurrent
runs. Do not reintroduce them without new measurements.
`mise.toml` pins stable 1.99.0 for build, lint, and test, with
`mr_boxington = true`, which routes Cargo through mr-boxington so worktrees
share one build store. A dated nightly carries only rustfmt: `format` runs on
it for `rustfmt.toml`'s unstable options, and `doc` builds on it as docs.rs
does. The
mise pin, `rust-toolchain.toml`, and `rust-version` move together in one
commit; `check:toolchain` fails when they differ. CI uses the
same pins and caches through `jdx/mr-boxington-action`.
Do not `&&` those in a new task; `depends` is the mise form. Those cargo
invocations share `target/`; the package-cache lock serializes them. Do not
invent extra `CARGO_TARGET_DIR` trees to hide that. Advisories (and yanked
crates) are `mise run deny:advisories` on CI only, so a pre-push `verify`
does not need the network. CI also runs `mise run semver`
(`verctl semver`: cargo-semver-checks against the last release, at the
release type the pending fragments allow) before packaging. An unfixable RUSTSEC or yank is listed in
`deny.toml` `[advisories].ignore` with a reason, not by weakening `yanked`
or `unmaintained`.

## Git

Conventional commits. lefthook. No `--no-verify`. Branch `type/number-desc`.
Always open a PR — never push to `main`.

## Changes

Plan a behaviour or contract change as an OpenSpec change in
`openspec/changes/<name>/` before writing code. `openspec/config.yaml` holds
this repository's context and rules, and `openspec validate <name>` checks the
change. One change maps to one `tasks.yaml` row.
