# Design

## Context

`App` parses a command in one clap pass ([app.rs][app]).
Before it, `env_under_flags` parses a clone leniently to learn what the command line set at each subcommand level, and unbinds each variable the command line overrules ([env.rs][env]).
Declared input reads YAML, JSON, TOML and a CLI's own formats into a located tree, so any key can be placed on its line ([ctl-input][input]).

[research.md](research.md) compares the ways to put a document between the environment and the default.

## Goals and non-goals

Goals:

1. One document per command, from a file or stdin, in any registered input format.
2. Clap's own parsers and rules check every document value.
3. Every problem in the document names its file, line and column.

Non-goals:

1. Nested documents that select a subcommand: the command line selects it.
2. Telling a handler which values came from the document.
3. The consumer adoptions and their golden tests, which land with each CLI's adoption of the release.

## Decisions

### 1. Synthesized arguments

The document becomes argument tokens that clap parses with the command line, once.
Clap then applies every check a typed flag gets: value parsers, possible values, `num_args`, conflicts, `exclusive`, `requires`, `required`, groups and `required_if_eq`.

Alternatives considered:

1. Default values set from the document keep the value parsers but skip every relational check, because clap does not count a default as given.
2. A synthetic variable per key sits exactly one layer below the real one, but needs `unsafe` `set_var`, loses `num_args` and lists without a delimiter, and confuses `env_under_flags` and help.
3. Editing `ArgMatches` has no public API.
4. Every crate surveyed that merges a file with clap deserializes the file with serde, so clap never checks it.

### 2. The flags

`App` adds `--input <FILE>` and `--input-format <NAME>` to every command, as it adds `--format`, under an "Input" help heading.
Neither has a short form.
When the command's `--format` reads `<PREFIX>_FORMAT`, they read `<PREFIX>_INPUT` and `<PREFIX>_INPUT_FORMAT`.
`--input-format` takes a name from the input format registry that `Input::open` uses.

A document from `<PREFIX>_INPUT` is checked like one from `--input`, so a key the selected command lacks fails there too.
The variable therefore suits a caller that runs one command, such as one step of a workflow; set in a shell profile, it fails every command whose keys the document does not match.
This follows the rule that every flag has a variable and reads it the same way.

They exist only with `app` and the opt-in `input-document` feature, which a CLI names beside the input formats it reads, so no other tier pays for them (operator, 2026-11-26).
Measured, the document code and `error-context` add about 45 KB to the `input` tier.
`App::without_input()` removes both.

### 3. Reading the document

1. `--input FILE` reads the file through the registry, by `--input-format` when given, else by the file's extension.
2. `--input -` reads stdin and needs `--input-format`; without it, the parse fails naming the flag.
3. The document is read under the input limits in force, as a config file is.
4. Its root must be a mapping.

### 4. Keys

A key is a long name, as written on the command line, or a positional's id, of the command the command line selected.
Global arguments visible at that level count.
`input`, `input-format`, help, version and hidden arguments are not keys.
Any other key is refused on its line, and every such problem is reported at once.

### 5. Encoding a value

| Argument | Document value | Tokens |
|:--|:--|:--|
| An option | a scalar | `--name=VALUE` |
| An option taking many values | a sequence | one `--name=VALUE` per item, or one joined by its delimiter |
| A flag that sets true | `true` | `--name` |
| A flag that sets true | `false` | `--no-name` when that side exists, else nothing |
| A counted flag | an integer from 0 to 255 | that many `--name`; a larger value is refused on its key |
| A positional | a scalar or sequence | each value after `--` |

The `=` form keeps a value that starts with `-` a value.
A scalar's text is its processed value, so a quoted YAML number stays text for the argument's own parser.
`null` leaves the key out.
A mapping, a sequence for a one-value option, and a sequence for an option with several values and no delimiter are refused on their key.

### 6. Where the tokens go

Option tokens go right after the selected subcommand's name, found by probing growing prefixes of the command line with clap itself, or right after the binary name at the root.
Positional values go at the end after one `--`, in the positionals' index order whatever the document's key order.
A positional key whose lower-index positionals the document does not all supply is refused on its line, since clap fills positionals by position.
The document supplies positionals only when the command line gives none at that level, holds no `--`, and the command has no `last(true)` positional and no `allow_missing_positional` (operator, 2026-11-26).
Otherwise a positional key is refused on its line.

### 7. Precedence and conflicts

The lenient probe that `env_under_flags` runs tells which arguments the command line set and which variables it keeps.
A document key is dropped when:

1. the command line set the same argument, or the other side of its `--x`/`--no-x` pair;
2. the command line set an argument it conflicts with;
3. its own variable is kept, the variable of the other side of its `--x`/`--no-x` pair is kept, or a kept variable conflicts with it.

So the command line wins over the environment, the environment over the document, and the document over the default.

`App::input_conflicts(InputConflicts::Fail)` turns each drop into a problem on the key that names the flag or variable it lost to.
The default is `InputConflicts::Drop` (operator, 2026-11-26).
The same policy covers a key against its own `--no-x` side.

Failing by default lost: an override for one run would mean editing the file.

### 8. Placing a problem

ctl-core's own refusals (unknown key, unencodable value, positional not allowed, a conflict under `Fail`) are collected and reported together, before clap runs.

Clap stops at its first error.
ctl-core records the key span behind each synthesized argument.
When clap's error names an argument that came from the document, the problem is placed on that key, or on its value for an invalid value.
Naming the argument needs clap's `error-context`, so `input-document` turns on ctl-core's `error-context` feature, which turns on clap's (operator, 2026-11-26).
`check:tree` keeps refusing it for `app,usage,input` without `input-document`.
Under `--format json`, the problems are separate objects in the error envelope, through the [structured-errors change](../structured-errors/design.md), never one joined message (operator, 2026-11-26).

A missing required argument has no key, so it reads as clap words it.

Leaving `error-context` off lost: a bad document value would name a flag the user never typed.

### 9. Value source

A document value reads as `ValueSource::CommandLine` to the handler.
A handler that must tell the difference is out of scope (non-goal 2).

## Risks

1. Finding the subcommand's position by probing prefixes is untested with `infer_subcommands` and flag subcommands.
   Tests cover both before release, and a failure there refuses the document rather than guessing.
2. `input-document` costs about 45 KB; its own size tier holds that budget, and no existing tier moves.

[app]: https://github.com/victor-software-house/ctl-core/blob/7d9f67c600108627bba08acca27bccf274205f0d/ctl-core/src/app.rs
[env]: https://github.com/victor-software-house/ctl-core/blob/7d9f67c600108627bba08acca27bccf274205f0d/ctl-core/src/flags/env.rs
[input]: https://github.com/victor-software-house/ctl-core/blob/7d9f67c600108627bba08acca27bccf274205f0d/ctl-input/src/lib.rs
