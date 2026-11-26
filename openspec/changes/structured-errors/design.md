# Design

## Context

`ErrorBody` has `bin`, `message` and `kind` ([model.rs][model]).
`App` builds it in three places: a parse error, a handler error, and a failure to settle the view ([app.rs][app]).
`json` serializes the envelope, and every other format writes `{bin}: {message}` through the default `Format::write_error` ([interface.rs][interface]).

## Goals and non-goals

Goals:

1. Every part of a failure is its own JSON field.
2. A person reading `pretty` sees what they see today.

Non-goals:

1. Changing `kind`, which stays the CLI's word for scripts to match.
2. Bumping `SCHEMA_VERSION`: every new field is optional and left out when empty.

## Decisions

### 1. The fields

| Field | Type | Holds |
|:--|:--|:--|
| `message` | string | one sentence: clap's first line, the input's summary, or the outermost error |
| `kind` | string, optional | as today |
| `causes` | list of strings | each further error of an `anyhow` chain, outermost first |
| `problems` | list of objects | one per input problem: `input`, `line`, `column`, `field`, `message`; `line`, `column` and `field` are left out when unknown |
| `usage` | string, optional | the selected command's usage line, without the `Usage:` label |
| `parse` | object, optional | clap's parts, below |

`parse` holds `reason`, clap's `ErrorKind` in snake case such as `unknown_argument`, and these when clap gives them: `argument`, `value`, `valid_values`, `suggestions`, `conflicts_with`, `tips`.
Each is a string or a list of strings, with no ANSI.

### 2. Where each part comes from

1. A parse error: `reason` from `Error::kind`; the rest from `Error::get` with `ContextKind`; `message` from clap's rendered first line.
   ctl-core cannot see a dependency's feature, so it reads the context under its own `error-context` feature, which turns on clap's; `input-document` turns it on, and a consumer that wants the parts turns on ctl-core's `error-context` rather than clap's.
   Without it, `parse` holds only `reason`, even when the consumer turns on clap's `error-context` itself.
2. `usage`: ctl-core renders the selected command's usage with clap's `usage` feature, which `help` turns on, for every parse error.
3. An `InputError` found by ctl-core, such as a refused input document: `problems` from its problems, `message` as `<input> has <n> problems`, or the one problem's message when it has one.
4. A handler error: `message` from the outermost error, `causes` from `Error::chain` after it.
   A `CommandError`'s exit code and `kind` are found as today.

### 3. Pretty output

`Format::write_error`'s default renders `{bin}: {message}` with each cause after it as `: {cause}`, as `{error:#}` reads today.
Then it writes one line per problem as `{input}:{line}:{column}: {field}: {message}`, the line `InputError` prints today, then the tips and `Usage: {usage}` as clap lays them out.
An `InputError` in a handler's chain becomes `problems` and adds no cause of its own.
A format that overrides `write_error` gets the same `ErrorBody`.

### 4. A non-exhaustive body

`ErrorBody` becomes `#[non_exhaustive]` and gains builders for the new fields, so a later field is no break.
A CLI builds one with `ErrorBody::new` and the builders.

Alternatives considered:

1. Keeping clap's whole text in `message` keeps one field, which is what the operator rejected (operator, 2026-11-26).
2. A second envelope type for parse errors would give a caller two shapes to tell apart.

## Risks

1. A script that read `message` for a problem or a cause gets one sentence now; the changeset names the fields that replace it.

[model]: https://github.com/victor-software-house/ctl-core/blob/55cbfb6c5249df822535ff206dbb93af80e111ca/ctl-core/src/model.rs
[app]: https://github.com/victor-software-house/ctl-core/blob/55cbfb6c5249df822535ff206dbb93af80e111ca/ctl-core/src/app.rs
[interface]: https://github.com/victor-software-house/ctl-core/blob/55cbfb6c5249df822535ff206dbb93af80e111ca/ctl-core/src/format/interface.rs
