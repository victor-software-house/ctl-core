# Design

## Context

`clap_complete` offers two ways to complete a Clap command
([research], section 1). A static script from `aot::generate` holds the
grammar and knows only what Clap knew at build time. A dynamic
registration from `CompleteEnv` is a short shim; the shell runs
`COMPLETE=<shell> <bin> -- <words…>` at each TAB and the binary prints the
candidates, including values an `ArgValueCompleter` computes then. forkctl
uses the dynamic form today, for patch names, refs, and remotes.

ctl-core's `App` already has an ordered `before_parse` hook list, which is
where forkctl runs `CompleteEnv`.

## Decisions

1. **Dynamic registration.** The verb prints `CompleteEnv`'s registration,
   not `aot::generate`'s script. A CLI can then complete values it knows
   only at run time, as forkctl does and qctl's row ids would.
   Alternative: the static script. It lost because it cannot list a
   computed value, and because Homebrew regenerates the dynamic script at
   every install and upgrade, which removes the drift `clap_complete` warns
   about for a saved script.
2. **An exact pin.** `clap_complete = "=4.6.11"` with `unstable-dynamic`.
   The feature is exempt from semver ([clap#3166] is open), so a caret
   range could change the shim's protocol in a patch release. Alternative:
   the static `aot` module, which needs no unstable feature. It lost for
   the reason in decision 1. The pin also turns on Clap's `unstable-ext`
   feature for every consumer that enables `completion`; it only adds
   extension points.
3. **One `Shell` type, three shells.** `Shell` lives in its own `shell`
   feature so the shell-integration change can use it without
   `clap_complete`. It holds bash, zsh, and fish: the set Homebrew installs
   by default and the set every surveyed CLI supports. Alternative: the
   five shells `CompleteEnv` registers, adding elvish and powershell. It
   lost because every verb that takes a `Shell` must handle every variant,
   and the shell-integration hooks and quoting have no tested elvish or
   powershell form.
4. **The verb is a flattened subcommand.** `CompletionCommand` is a Clap
   subcommand enum with one variant, flattened into a CLI's own enum with
   `#[command(flatten)]`. ctl-core then owns the verb's name, its about
   line, and its argument, and the verb still appears in the CLI's Clap
   type, so help, the Usage spec, and `Surface` documents see it.
   Alternative: `App` adds the subcommand to the command at run time. It
   lost because help, the Usage spec, and `Surface` build from the CLI's
   type and would not list it.
5. **The script calls the binary by name.** The completer is `bin`, never
   `argv[0]`. Homebrew runs the binary from its versioned keg, so the
   default would bake that keg's path into the installed file
   ([research], section 4).
6. **ctl-core appends the zsh first-TAB fix.** Three lines run the
   completer when zsh autoloads the file as `_<bin>`, and do nothing when
   the file is sourced ([research], section 6). Alternative: wait for
   `clap_complete` to fix its template. It lost because Homebrew is the
   main install path for the ctl CLIs. The fix stays until a pinned
   `clap_complete` writes its own; the test that asserts it shows when.
7. **`App::completion()` prints the registration itself.** With
   `COMPLETE=<shell>` and no words after `--`, the hook prints
   `Shell::script(bin)`, so `COMPLETE=zsh qctl` and `qctl completion zsh`
   write the same bytes, fix included. With words after `--` it hands the
   request to `CompleteEnv`, restricted to the same three shells.
   Alternative: let `CompleteEnv` answer both. It lost because its
   registration has no zsh fix and would accept elvish and powershell.
8. **The variable is `COMPLETE`.** It is `clap_complete`'s default, and
   jj and Homebrew's `:clap` format use it. The shim and the binary must
   agree, and both come from ctl-core, so the name is a constant, not a
   setting. Disable paths: a CLI that never calls `App::completion()` reads
   no variable, and `COMPLETE=` or `COMPLETE=0` leaves a run alone.
   Alternative: a per-CLI name such as `QCTL_COMPLETE`. It lost because
   Homebrew's `:clap` format sets only `COMPLETE`.
9. **The verb's view ignores quiet.** `completion::view(format)` keeps the
   format, turns colour off, and never sets quiet, because
   `source <(qctl completion zsh)` must load something even when
   `QCTL_QUIET=1` is set. JSON keeps working: `--format json` prints
   `{"shell": "zsh", "text": "…"}`. Alternative: always pretty, as forkctl
   does. It lost because it breaks the one-model-every-mode rule for no
   gain.
10. **No `--install`.** mise and usage offer one. Writing into shell
    directories is a second behaviour, and Homebrew already installs the
    files. The help and rustdoc name the line to add instead.

## Risks

1. A `clap_complete` bump can change the shim protocol. The exact pin and
   the script tests make every bump a reviewed change.
2. A shell that runs a script from an older binary against a newer one can
   complete wrongly. Loading the script at shell start, or through
   Homebrew, avoids that; the rustdoc says so.

[clap#3166]: https://github.com/clap-rs/clap/issues/3166
[research]: ../../../docs/research/shell-completion.md
