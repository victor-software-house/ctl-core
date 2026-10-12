# Distributing the Rust baseline files

Researched 2026-10-11. The ctl repositories share one Rust baseline:
`rust-toolchain.toml`, `rustfmt.toml`, `clippy.toml`, `deny.toml`, the Cargo
`[lints]` tables, and the mise `[tools]` pins. ctl-core renders them
(`baseline::render`, feature `baseline`), and VER-045 planned for verctl to
write and check them in each repository. VER-045 is paused until one way of
distributing them is chosen. This document lists the ways, what each tool
supports natively, and prior art. It proposes no code.

The shared mise tasks are already distributed by a native include
(`tasks/rust.toml`, CTC-016), so they are out of scope here.

## What each tool supports natively

None of the five Rust configuration files has an `extends` or remote include.
Each tool finds its file by walking from a start directory up through the
parents, and the first file found wins. An override flag or variable can point
at one other location.

| File | Lookup | Include or extends | Override |
|:--|:--|:--|:--|
| `rust-toolchain.toml` | Walks up from the current directory; the nearest file wins ([rustup overrides][rustup-overrides]) | None | `RUSTUP_TOOLCHAIN`, `rustup override`, `+toolchain` |
| `rustfmt.toml` | Walks up from the formatted file's directory for `.rustfmt.toml` or `rustfmt.toml` ([rustfmt config][rustfmt-config]) | None | `--config-path` |
| `clippy.toml` | Walks up from `CLIPPY_CONF_DIR`, else `CARGO_MANIFEST_DIR`, else `.` ([clippy conf.rs][clippy-conf]) | None | `CLIPPY_CONF_DIR` (a directory) |
| `deny.toml` | Walks up from the manifest directory, trying `deny.toml`, `.deny.toml`, `.cargo/deny.toml`, `.config/deny.toml` ([cargo-deny config][deny-cfg]) | None. A `deny.exceptions.toml`, found the same way, adds local exceptions ([common.rs][deny-common]) | `--config` |
| Cargo `[lints]` | Inline in each `Cargo.toml` | `[workspace.lints]` inheritance, within one workspace only. Sharing across workspaces, [cargo#14236][cargo-14236], was closed as not planned on 2024-10-29 | None |
| mise `[tools]` | `mise.toml` and the `mise.<env>.toml` files | Top-level `include` takes local paths, `git::` URLs, and `oci::` artifacts ([mise configuration][mise-include]) | Local config overrides included entries |

Two more native mechanisms are close but do not cover the gap:

1. **`.cargo/config.toml` `include`.** Stable since Cargo 1.93.0
   ([unstable.md][cargo-unstable]). It accepts only local `.toml` paths,
   relative to the including file ([config.md][cargo-config]), so the
   included file must still be present in the repository.
2. **mise top-level `include`.** An included file may carry `[tools]`,
   `[env]`, `[vars]`, `[hooks]`, `[plugins]`, and a few more, but not
   `[settings]` or `[tasks]`. Its tools are locked in the including file's
   lockfile. A tag or branch ref is refreshed by `mise install` once the cache
   is older than `fetch_remote_versions_cache`; a commit SHA is never
   refetched. The mise docs mark no experimental gate for it, unlike `git::`
   task includes, which need `experimental = true` (verified in CTC-016). The
   top-level include was read in the docs and not tested here.

So mise `[tools]` is the only baseline item a repository can take from
ctl-core without holding a copy. The other five must exist as files in each
repository, because separate clones and CI checkouts share no parent
directory.

## Options

| # | Option | Covers | Drift detection | Update path | Cost |
|:--|:--|:--|:--|:--|:--|
| 1 | Hand copy from the `rust-projects` skill | All six | None | Manual per repository | The current state; qctl and forkctl already drifted |
| 2 | Generator writes, checker fails on drift (VER-045 as planned) | All six | `ver` check exits non-zero in `verify` and CI | Bump ctl-core in verctl, run the writer, commit | verctl links ctl-core's `baseline` feature; repos run verctl |
| 3 | Central repository opens sync PRs | Whole files only | The PR itself | Merge the bot PR | A GitHub App or token with write access to every repository |
| 4 | mise `include` for `[tools]` | mise `[tools]` only | None needed; the content is not copied | Bump the ref | `[tools]` leaves `mise.dev.toml`; the `tools` marker splice goes away |
| 5 | Shared directory through override variables | `clippy.toml`, `rustfmt.toml`, `deny.toml` | None | Bump the checkout | Every repository and CI job needs the baseline at a known path; editors and rust-analyzer do not read the variables |
| 6 | One Cargo workspace for all crates | All five files by parent walk | Not needed | One commit | Merges the repositories and changes the release model |
| 7 | One-shot scaffold | All six, once | None | None | No update path |

Notes per option:

1. The `rust-projects` skill's `baseline-config.md` is this today.
2. ctl-core already renders all seven targets, with marker-bounded splices
   for `Cargo.toml` (`lints`) and `mise.dev.toml` (`tools`), and a
   self-check test. verctl's served-file block would write and check them.
3. Whole-file sync cannot splice the `[lints]` block into a `Cargo.toml`
   that also holds each repository's own package data.
4. Combines with option 2: the generator then serves five files, and
   `mise.dev.toml` loses its `tools` block. When the ctl-core ref for
   `[tools]` and for `tasks/rust.toml` is the same tag, the rule that the task
   include and the tool pins move together reduces to one bump.
5. Needs a submodule or a fetched copy in every clone. This is option 2's
   problem, with less tooling support.
6. Ruled out in practice: the ctl crates release independently.

## Prior art

| Project | Mechanism | Drift handling |
|:--|:--|:--|
| [projen][projen] | Synthesizes configuration from code; synthesized files are read-only | Refuses manual edits; needs Node |
| [cruft][cruft] | Tracks the template commit in `.cruft.json` | `cruft check` exits 1 when the project is behind the template |
| [copier][copier] | Records answers in `.copier-answers.yml` | `copier update` re-renders and merges the template's changes |
| [repo-file-sync-action][repo-file-sync] | A GitHub Action in the source repository | Opens a PR in each target repository |
| [Renovate presets][renovate-presets] | `extends` in Renovate's configuration | Applies only to Renovate's own configuration |
| [cargo-generate][cargo-generate] | One-shot scaffold from a template repository | None after generation |
| [Embark lints.toml][embark-lints] | A `rustflags` block pasted into `.cargo/config.toml` | A versioned marker comment; the repository is archived |
| [release-plz `version_group`][release-plz-version-group] | One version across packages of one workspace | Same workspace only |

Option 2 is the cruft and projen pattern: a pinned source, a generator, and
a check that fails. None of the prior art distributes Rust tool
configuration across separate repositories natively.

## Recommendation for the decision

1. Take mise `[tools]` through a top-level `include` from ctl-core at the
   same tag as `tasks/rust.toml` (option 4), after a test that confirms it
   needs no experimental gate and locks into the including lockfile.
2. Distribute the other five through verctl's served-file block (option 2),
   so drift fails `verify` and CI.
3. Keep option 1 only until option 2 ships.

[rustup-overrides]: https://github.com/rust-lang/rustup/blob/61271d00a979e8fcc13a366d9515f4bf69b88ac2/doc/user-guide/src/overrides.md
[rustfmt-config]: https://github.com/rust-lang/rustfmt/blob/ff020519c12b1c8183ee91950e9507bafe801136/src/config/mod.rs
[clippy-conf]: https://github.com/rust-lang/rust-clippy/blob/51a5a0b9d43e120e59094aabc631ad1f253c5d4c/clippy_config/src/conf.rs
[deny-cfg]: https://github.com/EmbarkStudios/cargo-deny/blob/735416424c5629429d7d59a6fcfc863eebe4e21f/docs/src/checks/cfg.md
[deny-common]: https://github.com/EmbarkStudios/cargo-deny/blob/735416424c5629429d7d59a6fcfc863eebe4e21f/src/cargo-deny/common.rs
[cargo-14236]: https://github.com/rust-lang/cargo/issues/14236
[cargo-unstable]: https://github.com/rust-lang/cargo/blob/1ef25ea3d391374fe3bed4dcbe24f0d3be3ab8f9/doc/book/src/reference/unstable.md
[cargo-config]: https://github.com/rust-lang/cargo/blob/1ef25ea3d391374fe3bed4dcbe24f0d3be3ab8f9/doc/book/src/reference/config.md
[mise-include]: https://github.com/jdx/mise/blob/f17a47e6cc2d230544edd4e7032bccf916cd9836/docs/configuration.md
[projen]: https://github.com/projen/projen
[cruft]: https://github.com/cruft/cruft
[copier]: https://github.com/copier-org/copier
[repo-file-sync]: https://github.com/BetaHuhn/repo-file-sync-action
[renovate-presets]: https://docs.renovatebot.com/config-presets/
[cargo-generate]: https://github.com/cargo-generate/cargo-generate
[embark-lints]: https://github.com/EmbarkStudios/rust-ecosystem/blob/bc9f6c527108a8b79b74d44121aed574e7b5e526/lints.toml
[release-plz-version-group]: https://release-plz.dev/docs/config#the-version_group-field
