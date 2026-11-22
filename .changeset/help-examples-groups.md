---
ctl-core: patch
---

Help renders more of what Clap declares.
`after_help` shows as an Examples section on `-h` and `--help`, and `after_long_help` replaces it on `--help`.
`--help` and the `help` subcommand show `long_about` and each argument's `long_help`, falling back to `about` and `help`.
A command's visible aliases follow its name, as in `ls, list`.

`App::command_group(heading, commands)` lists some of the root's commands under a heading of their own, in the order given.
Each call adds one group, and the root's other commands follow under Commands.
