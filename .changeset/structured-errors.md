---
ctl-core: patch
---

The JSON error envelope keeps a field per part of a failure, so a caller no longer parses prose.
`ErrorBody` gains `causes`, `problems`, `usage`, and `parse`, each left out of JSON when empty and described by the schema.
`causes` lists each further error of an `anyhow` chain, outermost first.
`problems` holds one `InputProblem` per problem of a refused input, with its `input`, `line`, `column`, `field`, and `message`.
`usage` is the usage line of the selected command, without the `Usage:` label, on every parse error.
`parse` is a `ParseFailure`: Clap's error kind in snake case as `reason`, and a field for each kind of Clap's error context: `argument`, `missing`, `value`, `valid_values`, `subcommand`, `valid_subcommands`, `suggested_arguments`, `suggested_values`, `suggested_subcommands`, `suggested_commands`, `conflicts_with`, `actual_values`, `expected_values`, `min_values`, `trailing_argument`, `tips`, and `notes`.
The new `error-context` feature turns on Clap's, so `parse` holds those parts; without it, `parse` holds only `reason`.
`ErrorBody::text` is the text a person reads, which `pretty` and every other non-JSON format write after `{bin}: `.
`App` sets it to what each failure printed before it had fields, so a non-JSON error reads as it did; JSON and the schema leave it out.

Build an `ErrorBody` with `ErrorBody::new` and the builders `kind`, `causes`, `problems`, `usage`, `parse`, and `with_text`.
`InputProblem` and `ParseFailure` are `#[non_exhaustive]` too, built with `InputProblem::new` and `ParseFailure::new` and their builders.

`message` is one sentence for every failure, so a script that read it for a cause or a problem reads `causes` or `problems`:

1. A handler error's `message` is its outermost error, and the rest of its chain is in `causes`, where it was joined with `: `.
2. A refused input's `message` is `<input> has <n> problems`, and the lines it carried are in `problems`.
3. A parse error's `message` is Clap's first line, so the tip, the usage, and the pointer to `--help` are no longer in it.
