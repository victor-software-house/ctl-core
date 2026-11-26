---
ctl-core: patch
---

Every command takes `--input <FILE>` and `--input-format <NAME>` with the new `input-document` feature and an input format (`input-yaml`, `input-json`, `input-toml` or `input`).
`input-document` turns on `app` and `error-context`, and fails to compile without an input format.
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
Under `--format json`, a refused document carries each problem in `problems`, and a Clap error placed on a key adds its `parse` parts and usage line.

`ctl-input` adds `Input::tree`, which lends the strict read's root node to a closure, and `default_formats`, the registry `Input::open` reads through.
