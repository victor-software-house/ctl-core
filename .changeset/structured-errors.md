---
ctl-core: patch!
---

The JSON error envelope keeps a field per part of a failure, so a caller no longer parses prose.
`ErrorBody` gains `causes`, `problems`, `usage`, and `parse`, each left out of JSON when empty and described by the schema.
`causes` lists each further error of an `anyhow` chain, outermost first.
`problems` holds one `InputProblem` per problem of a refused input, with its `input`, `line`, `column`, `field`, and `message`.
`usage` is the usage line of the selected command, without the `Usage:` label, on every parse error.
`parse` is a `ParseFailure`: Clap's error kind in snake case as `reason`, and the `argument`, `value`, `valid_values`, `suggestions`, `conflicts_with`, and `tips` Clap gives.
The new `error-context` feature turns on Clap's, so `parse` holds those parts; without it, `parse` holds only `reason`.
`pretty` writes the same parts: `{bin}: {message}` with each cause after it as `: {cause}`, a line per problem, each tip, then `Usage: {usage}`.
A handler error with no input problem prints what it printed before.

`ErrorBody` is `#[non_exhaustive]`, so a later field is no break: build one with `ErrorBody::new` and the builders `kind`, `causes`, `problems`, `usage`, and `parse`.
`InputProblem` and `ParseFailure` are `#[non_exhaustive]` too, built with `InputProblem::new` and `ParseFailure::new` and their builders.

Breaking changes:

1. `message` is one sentence for every failure.
   A handler error's `message` is its outermost error, and the rest of its chain is in `causes`, where it was joined with `: `.
   A refused input's `message` is its one problem's message, or `<input> has <n> problems`, and the lines it carried are in `problems`.
   A parse error's `message` is Clap's first line, so the tip, the usage, and the pointer to `--help` are no longer in it.
   A script that read `message` for a cause or a problem reads `causes` or `problems`.
2. A `pretty` parse error ends with `Usage: {usage}` and no longer ends with Clap's pointer to `--help`.
   With `error-context`, its first line names the argument or value, as Clap words it.
