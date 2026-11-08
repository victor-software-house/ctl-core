---
ctl-core: patch
---

Add a shared shell completion verb. The new `completion` feature gives a ctl CLI `<bin> completion <SHELL>` for bash, zsh, and fish. To use it:

- flatten `completion::CompletionCommand` into the CLI's subcommands;
- call `App::completion()`, so the binary answers `COMPLETE=<shell>` at each TAB;
- route the verb to `shell::view(format)` and return `command.script("<bin>")`.

The zsh script completes on the first TAB when Homebrew installs it in `fpath`. The `completion` feature enables `app`, which `App::completion` needs. The new `shell` feature holds the `Shell` value the verb takes.
