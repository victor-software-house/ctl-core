# Design: shared Rust tasks

## Decisions

### 1. One TOML task file, included by `git::` URL

mise loads a `.toml` file named by a `git::` include as a TOML task file, in
which each task is written as under `[tasks]` without the prefix. The file
keeps the `run` lists, `shell`, `timeout`, and `depends` the tasks already
use. A directory of executable file tasks, the form `q` uses, was
considered. It lost because `check:toolchain` needs a `run` list and a
`shell`, and `test` needs a `timeout`, which `#MISE` header lines do not
accept.

### 2. The include sits in `mise.toml`, next to `q`

For each directory, mise uses the `includes` list of the highest-precedence
config file that sets one, and does not merge lists. A list in
`mise.dev.toml` would replace the `q` include for the `dev` environment.
The include therefore joins the existing list in `mise.toml`.

### 3. `verify` reaches the repository's own `check:features`

Included tasks share one namespace with the repository's tasks, so the
`depends` entry `check:features` resolves to the task the repository
defines. A repository without it fails `mise run verify` with a missing
task, which is what it should do. Making `check:features` optional was
considered. It lost because every ctl crate advertises feature sets.

### 4. Tasks run in the including repository

An included task's default `dir` is the including config root, so `cargo`
runs in the repository, not in mise's clone of ctl-core.

### 5. ctl-core reads its own copy by path

ctl-core includes `tasks/rust.toml` by path, so its tasks are the shared
tasks and cannot drift from them. Including its own release by URL was
considered. It lost because a change to the file could then only be tested
after it was released.

### 6. Pins move together

`format` names the formatting nightly, which the repository's
`mise.dev.toml` installs. A repository bumps the include ref and its tool
pins in one commit.

## Automatic behaviour, override, and disable

1. Override one task: define a task of the same name in a higher-precedence
   config, such as `mise.dev.toml`. Its command replaces the included one.
2. Disable all: remove the include.
3. mise loads `git::` includes only with `experimental = true`. Without it,
   mise warns that it skipped the include, and `mise run verify` fails with
   a missing task instead of passing.

## Known issue

A git hook on a fresh machine can fail while mise makes its first clone of a
`git::` include. An upstream gix bump in mise addresses it. The consumer
proof records what a fresh build runner does.
