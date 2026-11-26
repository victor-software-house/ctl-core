---
ctl-core: patch
---

Every command takes `--input <FILE>` and `--input-format <NAME>` when `app` and an input feature are both on.
They read `<PREFIX>_INPUT` and `<PREFIX>_INPUT_FORMAT`, with the prefix `--format` reads its variable with.
A YAML, JSON or TOML document, or stdin for `--input -`, fills the arguments of the command the command line selects.
Its keys are long names and positional ids, and each value passes the argument's own parser and rules.

The command line wins over the environment, the environment over the document, and the document over the default.
A key that loses is dropped.
`App::input_conflicts(InputConflicts::Fail)` refuses it instead, with a problem on its line.
`InputConflicts` is new.

`App::without_input` removes both flags.
A CLI that declares `input` or `input-format` itself must call it, or `App` panics.

Every problem in a document names its file, line and column.
Each input feature now turns on Clap's `error-context`, so a parse error names the argument and shows its usage.

`ctl-input` adds `Input::tree`, the strict read as a root node, and `default_formats`, the registry `Input::open` reads through.
