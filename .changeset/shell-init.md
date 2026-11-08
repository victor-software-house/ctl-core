---
ctl-core: patch
---

Add a shared shell integration verb. The new `shell-init` feature gives a ctl CLI `<bin> init <SHELL>` for bash, zsh, and fish: declare `shell_init::Hook::new("<bin>", "<subcommand>", "<VARIABLE>")`, flatten `shell_init::InitCommand` into the CLI's subcommands, route the verb to `shell::view(format)`, and return `command.script(&hook)`. The script runs `<bin> <subcommand>` once, in the foreground, before the shell's first prompt, when the variable is non-empty and the binary is on `PATH`, then removes itself; loading it twice installs it once, and bash keeps the `$?` its prompt shows. `Script` and `view` move from `completion` to `shell`, which both verbs share.
