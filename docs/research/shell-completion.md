# Shell completion: how current CLIs ship it, and what the ctl CLIs should do

Surveyed 2026-11-07 from source at the pinned tags below. The question is
which `completion` verb ctl-core should give every ctl CLI, and how
Homebrew and mise installs should pick it up.

**Answer.** Ship one verb, `<bin> completion <SHELL>`, that prints
`clap_complete`'s dynamic registration for bash, zsh, and fish. The script is a short shim; the binary answers each TAB itself
through `CompleteEnv`. Pin `clap_complete` exactly, because the dynamic
engine is behind its `unstable-dynamic` feature. Append three zsh lines to
the script, because the zsh registration completes nothing on the first TAB when
it is installed in `fpath`, which is where Homebrew puts it (section 6).

## 1. The two kinds of script

1. **Static.** The script holds the whole command grammar. `clap_complete`
   writes it with `aot::generate`, which needs no feature
   ([clap_complete `lib.rs`][cc-lib]). It completes only what Clap knows at
   build time: subcommands, flags, and `ValueEnum` values. It cannot list a
   value the program computes, such as a patch name or a queue row.
2. **Dynamic.** The script is a shim. At each TAB the shell runs the program
   again, here as `COMPLETE=<shell> <bin> -- <words…>`, and the program prints
   the candidates ([`env/mod.rs`][cc-env]). `ArgValueCompleter` can compute
   values at that moment. `CompleteEnv` is gated behind `unstable-dynamic`
   ([`lib.rs` L62-66][cc-lib-gate], [`Cargo.toml` L54-58][cc-cargo]), and
   its tracking issue, [clap#3166], is still open with shell protocol,
   quoting, and space-handling items unchecked.

`CompleteEnv` has two modes ([`try_complete_`][cc-try]). With nothing after
`--` it prints the registration; with words after `--` it prints candidates.
It removes `COMPLETE` from its own environment first, so a value completer's
child process never answers a request meant for its parent. `COMPLETE=` or
`COMPLETE=0` turns it off.

The shim and the binary share a protocol that `clap_complete` does not keep
stable between versions. Its docs recommend regenerating the script at shell
start instead of saving it to a file ([`env/mod.rs` L23-33][cc-env]).

## 2. What current CLIs do

| CLI | Verb | Kind | Shells | Homebrew formula |
|:--|:--|:--|:--|:--|
| [jj] | `jj util completion <shell>`; `COMPLETE=<shell> jj` | both | bash, elvish, fish, nushell, powershell, zsh | `shell_parameter_format: :clap`, dynamic ([jj.rb L29][brew-jj]) |
| [mise] | `mise completion [SHELL]`, `--install` | dynamic shim | bash, fish, nu, powershell, zsh | committed static files ([mise.rb L50-53][brew-mise]) |
| [usage] | `usage generate completion <SHELL> <BIN>` | dynamic shim, needs `usage` on `PATH` | bash, fish, nu, powershell, zsh | not applicable |
| [rustup] | `rustup completions <shell> [rustup\|cargo]` | static | bash, elvish, fish, powershell, zsh | `"completions"` plus `pwsh` ([rustup.rb L59-74][brew-rustup]) |
| [gh] | `gh completion -s <shell>` | dynamic shim (cobra `__complete`) | bash, fish, powershell, zsh | `"completion", "-s"` ([gh.rb L48][brew-gh]) |
| [uv] | `uv generate-shell-completion <shell>`, hidden | static | clap's set | `"generate-shell-completion"` ([uv.rb L38-39][brew-uv]) |
| [ripgrep] | `rg --generate complete-<shell>` | static, hand-built | bash, fish, powershell, zsh | `"--generate", shell_parameter_format: "complete-"` ([ripgrep.rb L40][brew-rg]) |
| [starship] | `starship completions <shell>` | static | bash, elvish, fish, nushell, powershell, zsh | `"completions"` ([starship.rb L35][brew-starship]) |

Notes on the rows:

1. jj's docs recommend the dynamic form, and keep static as the default only
   "since the underlying engine is still labeled unstable"
   ([install-and-setup.md][jj-docs]). Dynamic completion there lists
   bookmarks, revisions, and files, and respects `--repository`.
2. mise's own scripts no longer need the `usage` CLI at TAB time; they call
   a hidden `mise __complete_word__` request ([shell-setup.md][mise-docs],
   [`completions/mise.bash`][mise-bash]). Older releases did need `usage`
   ([installing-mise.md L384][mise-old]).
3. usage's Rust binding mounts `tool completion <shell> [--install]` whose
   shim calls the binary back, the closest prior art to this design
   ([rust/completions.md][usage-rust]).
4. gh is cobra, whose generated scripts call a hidden `__complete` command,
   so its "script" is a shim too ([cobra completions][cobra-docs]).

Every CLI in the table except ripgrep names its verb after completion and
takes the shell as one argument. The positional form (`completion zsh`) is
the most common; gh's `-s` flag is the exception.

## 3. Shell-start cost

No primary source measures a static script against a shim. The mechanics
decide the cost:

1. **Installed into a completion directory, both kinds cost nothing at
   start.** zsh's `compinit` reads only the `#compdef` first line of each
   `fpath` file and autoloads the body on first use, caching the scan in
   `.zcompdump` ([zsh compsys][zsh-compsys]). bash-completion 2 loads files
   in its `completions` directory on demand by command name; its
   `bash_completion.d` compat directory loads eagerly
   ([bash-completion README][bash-completion]). fish loads
   `<command>.fish` from `$fish_complete_path` when that command is first
   completed ([fish completions][fish-docs]).
2. **Sourced from a startup file, the cost is one process start.**
   `source <(toy completion zsh)` runs the binary on every new shell. For a
   Rust binary that builds a Clap command and writes a few hundred bytes,
   that is one fork and exec. A static script sourced the same way costs the
   same process start plus parsing a larger script.
3. **At TAB, the shim starts the binary once per keypress.** That is the
   price of dynamic values. jj, gh, and mise all accept it.

## 4. How installs pick the script up

1. **Homebrew.** `generate_completions_from_executable(*commands,
   base_name:, shell_parameter_format:, shells:)` runs the installed binary
   at build time with `SHELL=<sh>` and writes each script
   ([formula.rb L2697][brew-gen]). Default shells are bash, zsh, and fish;
   the `:cobra` and `:typer` formats add `pwsh`
   ([shell_completion.rb][brew-shells]). With no format the shell name is
   the last argument, so `generate_completions_from_executable(bin/"qctl",
   "completion")` runs `qctl completion bash` and so on. `:clap` sets
   `COMPLETE=<sh>` and passes no argument. Files land in
   `etc/bash_completion.d/<name>`, `share/zsh/site-functions/_<name>`, and
   `share/fish/vendor_completions.d/<name>.fish`. Homebrew regenerates them
   at every install and upgrade, so clap's warning about a saved script
   drifting from its binary does not bite a Homebrew install. zsh users run
   `compinit` after `brew shellenv` puts `site-functions` on `FPATH`; fish
   finds the vendor directory itself ([Homebrew shell completion][brew-doc]).
2. **The completer path.** By default `CompleteEnv` bakes `argv[0]` into the
   script, made absolute when it has a directory part ([`env/mod.rs`
   L271-280][cc-reg]). Homebrew runs the binary from its versioned keg, so a
   default script names that keg's path. Passing the bare binary name as the
   completer makes the script find the binary on `PATH` instead, which
   survives an upgrade and a `brew cleanup`.
3. **mise.** The `github:` and `ubi:` backend docs describe where the
   binary goes (`bin_path`) and a `postinstall` command, and say nothing
   about completion files ([github backend][mise-github],
   [ubi backend][mise-ubi]). An operator who installs a ctl CLI that way
   loads the script from a startup file. Inside a repository, `mise run q`
   completes through the Usage spec ctl-core already serves
   ([`src/usage.rs`](../../ctl-core/src/usage.rs)), which is a separate path this
   change leaves alone.

## 5. Which shells

Homebrew installs bash, zsh, and fish by default, and every surveyed CLI
supports at least those three. `clap_complete`'s dynamic registration
also covers elvish and powershell at no extra cost
([`Shells::builtins`][cc-env]). It has no dynamic nushell; forkctl reaches
nushell today through the Usage library instead.

Support bash, zsh, and fish. The shell-integration verb shares the same
`Shell` type, and every verb that takes a `Shell` must handle each one, so a
shell joins only when every verb has a tested form for it. elvish and
powershell have none for shell integration. forkctl's elvish, powershell,
and nushell scripts stay its own decision when it adopts the shared verb.

## 6. zsh completes nothing on the first TAB from `fpath`

`clap_complete` 4.6.11's zsh registration defines
`_clap_dynamic_completer_<name>` and then runs `compdef` to bind it
([`env/shells.rs`][cc-shells]). Sourced, that is right. Installed as
`site-functions/_<name>`, `compinit` autoloads the whole file as the body of
`_<name>`, so the first TAB only defines the function and rebinds the
command, and produces no candidates.

Reproduced 2026-11-07 on macOS with zsh 5.9, no build involved: a shell
script stood in for the binary and printed `status` and `sync` when
`COMPLETE=zsh`; the registration, filled in by hand from the 4.6.11
template, went into an `fpath` directory; `zsh -f -i` under `zpty` ran
`compinit` and typed `toy`, a space, and TAB twice. The first TAB rang the bell and
the second completed `s`. With three lines appended, the first TAB completed
`s` and the second listed `status  sync`, and sourcing the same file still
registered the completer without running it:

```zsh
if [[ ${funcstack[1]} == _toy ]]; then
    _clap_dynamic_completer_toy "$@"
fi
```

`funcstack[1]` is `_toy` only when zsh runs the file as the autoloaded
function, so the sourced path is unchanged.

## 7. The verb's shape

1. **Name and argument.** `completion <SHELL>`, one positional value from a
   closed set. That matches rustup, starship, jj, and uv, keeps Homebrew's
   default format, and matches forkctl today.
2. **Help.** One line, as the surveyed CLIs do: rustup "Generate
   tab-completion scripts for your shell", jj "Print a command-line-completion
   script", gh "Generate shell completion scripts". ctl-core's help renders
   only the short about and the argument help, so the shells are named in the
   argument's help text.
3. **Output.** The script on stdout, with no colour and nothing else. Quiet
   must not hide it: `source <(qctl completion zsh)` would load nothing.
4. **No `--install`.** mise and usage offer one, but writing into a user's
   shell directories is a second behaviour with its own failure modes, and
   Homebrew already installs the files. Leave it out until someone asks.

## 8. forkctl today

forkctl already ships the dynamic design
([`src/main.rs` L64-78][forkctl-hook], [L263-302][forkctl-text]):

1. `App::before_parse` runs `CompleteEnv::with_factory(Cli::command)`.
2. `completion <shell>` writes each shell's `write_registration` with
   `COMPLETE`, the name `forkctl`, and the completer `forkctl`, so the
   script calls the binary through `PATH`.
3. `CompletionShell` adds nushell through the Usage library
   ([`src/cli.rs` L414-421][forkctl-shells]).
4. Its `select_view` forces pretty, colourless output for the verb.
5. It pins `clap_complete = "=4.6.8"` with `unstable-dynamic`.

[Output rendering comparison item 5](output-rendering-comparison.md) found
that qctl and verctl have no `completion` verb at all, and recommended
copying forkctl's. Moving the verb into ctl-core gives every consumer the
same verb in one change, instead of four copies that drift.

## 9. Recommendation

1. A `completion` cargo feature in ctl-core, with `clap_complete` pinned
   exactly and `unstable-dynamic` on, because that feature carries no semver
   promise.
2. A flattenable `completion <SHELL>` subcommand for bash, zsh, and fish,
   printing the dynamic registration with the bare
   binary name as the completer.
3. An `App` builder that answers `COMPLETE=<shell>` before parsing.
4. The zsh first-TAB lines from section 6, until `clap_complete` fixes its
   template.
5. A view for the verb that keeps the script visible under `--quiet`.

[bash-completion]: https://github.com/scop/bash-completion/blob/2.17.0/README.md
[brew-doc]: https://docs.brew.sh/Shell-Completion
[brew-gen]: https://github.com/Homebrew/brew/blob/bf464f57275cae0be106b43d190c98564ac59b0e/Library/Homebrew/formula.rb#L2697
[brew-gh]: https://github.com/Homebrew/homebrew-core/blob/0320d5a7ac3d7470022ac0bd0a855f485adab24f/Formula/g/gh.rb#L48
[brew-jj]: https://github.com/Homebrew/homebrew-core/blob/0320d5a7ac3d7470022ac0bd0a855f485adab24f/Formula/j/jj.rb#L29
[brew-mise]: https://github.com/Homebrew/homebrew-core/blob/0320d5a7ac3d7470022ac0bd0a855f485adab24f/Formula/m/mise.rb#L50-L53
[brew-rg]: https://github.com/Homebrew/homebrew-core/blob/0320d5a7ac3d7470022ac0bd0a855f485adab24f/Formula/r/ripgrep.rb#L40
[brew-rustup]: https://github.com/Homebrew/homebrew-core/blob/0320d5a7ac3d7470022ac0bd0a855f485adab24f/Formula/r/rustup.rb#L59-L74
[brew-shells]: https://github.com/Homebrew/brew/blob/bf464f57275cae0be106b43d190c98564ac59b0e/Library/Homebrew/utils/shell_completion.rb
[brew-starship]: https://github.com/Homebrew/homebrew-core/blob/0320d5a7ac3d7470022ac0bd0a855f485adab24f/Formula/s/starship.rb#L35
[brew-uv]: https://github.com/Homebrew/homebrew-core/blob/0320d5a7ac3d7470022ac0bd0a855f485adab24f/Formula/u/uv.rb#L38-L39
[cc-cargo]: https://github.com/clap-rs/clap/blob/2cb76fdf385396f86f37af49784504da560b6e47/clap_complete/Cargo.toml#L54-L58
[cc-env]: https://github.com/clap-rs/clap/blob/2cb76fdf385396f86f37af49784504da560b6e47/clap_complete/src/env/mod.rs
[cc-lib]: https://github.com/clap-rs/clap/blob/2cb76fdf385396f86f37af49784504da560b6e47/clap_complete/src/lib.rs
[cc-lib-gate]: https://github.com/clap-rs/clap/blob/2cb76fdf385396f86f37af49784504da560b6e47/clap_complete/src/lib.rs#L62-L66
[cc-reg]: https://github.com/clap-rs/clap/blob/2cb76fdf385396f86f37af49784504da560b6e47/clap_complete/src/env/mod.rs#L271-L280
[cc-shells]: https://github.com/clap-rs/clap/blob/2cb76fdf385396f86f37af49784504da560b6e47/clap_complete/src/env/shells.rs
[cc-try]: https://github.com/clap-rs/clap/blob/2cb76fdf385396f86f37af49784504da560b6e47/clap_complete/src/env/mod.rs#L205-L254
[clap#3166]: https://github.com/clap-rs/clap/issues/3166
[cobra-docs]: https://github.com/spf13/cobra/blob/7da941c3547e93b8c9f70bbd3befca79c6335388/site/content/completions/_index.md
[fish-docs]: https://fishshell.com/docs/current/completions.html
[forkctl-hook]: https://github.com/victor-software-house/forkctl/blob/dd51c3c925dc97ad768a5586a322b3127085e765/src/main.rs#L64-L78
[forkctl-shells]: https://github.com/victor-software-house/forkctl/blob/dd51c3c925dc97ad768a5586a322b3127085e765/src/cli.rs#L414-L421
[forkctl-text]: https://github.com/victor-software-house/forkctl/blob/dd51c3c925dc97ad768a5586a322b3127085e765/src/main.rs#L263-L302
[gh]: https://github.com/cli/cli/blob/63837fb778d6a39f02be0ce76ff89f2bc8d74bdc/pkg/cmd/completion/completion.go
[jj]: https://github.com/jj-vcs/jj/blob/d16ee1fc88a72c54142241b079a34bbf3b033bc0/cli/src/commands/util/completion.rs
[jj-docs]: https://github.com/jj-vcs/jj/blob/d16ee1fc88a72c54142241b079a34bbf3b033bc0/docs/install-and-setup.md#L230-L330
[mise]: https://github.com/jdx/mise/blob/4dc53ec645558e448f7f36a7b1f02ccbd69446d1/docs/shell-setup.md#L189-L250
[mise-bash]: https://github.com/jdx/mise/blob/4dc53ec645558e448f7f36a7b1f02ccbd69446d1/completions/mise.bash
[mise-docs]: https://github.com/jdx/mise/blob/4dc53ec645558e448f7f36a7b1f02ccbd69446d1/docs/shell-setup.md#L189-L250
[mise-github]: https://github.com/jdx/mise/blob/4dc53ec645558e448f7f36a7b1f02ccbd69446d1/docs/dev-tools/backends/github.md
[mise-old]: https://github.com/jdx/mise/blob/5a4686a917f222fbc504ff43418da594a3e46e37/docs/installing-mise.md#L384
[mise-ubi]: https://github.com/jdx/mise/blob/4dc53ec645558e448f7f36a7b1f02ccbd69446d1/docs/dev-tools/backends/ubi.md
[ripgrep]: https://github.com/BurntSushi/ripgrep/blob/e89fff89ac9af12e8d4ce9d5fd07beb408ca730f/crates/core/flags/defs.rs#L2397-L2450
[rustup]: https://github.com/rust-lang/rustup/blob/edbc831cec44dc2858a4a4dce768c74361d027dc/src/cli/rustup_mode.rs#L292-L298
[starship]: https://github.com/starship/starship/blob/f8e4e2155542cc84f49b342122d371f74e0ad253/src/main.rs
[usage]: https://github.com/jdx/usage/blob/facfe1a78b411582e1fd8f2598222685ab92637e/docs/cli/reference/generate/completion.md
[usage-rust]: https://github.com/jdx/usage/blob/facfe1a78b411582e1fd8f2598222685ab92637e/docs/rust/completions.md
[uv]: https://github.com/astral-sh/uv/blob/316c827e058c16b71d5c507a20729c2995a7bcce/crates/uv-cli/src/lib.rs#L488-L491
[zsh-compsys]: https://zsh.sourceforge.io/Doc/Release/Completion-System.html
