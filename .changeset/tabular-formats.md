---
ctl-core: minor
---

Piped output is now `tsv` for every CLI on ctl-core.
When stdout is not a terminal and no flag or variable names a format, a CLI writes tab-separated lines.
Before, it wrote the pretty table.
`--quiet` hides that default output, as it hides `pretty`.

`tsv`, `csv`, and `markdown` are new built-in formats: `format::TSV`, `format::CSV`, and `format::MARKDOWN`.
Each reads the result's document, so a script gets the columns a person sees.
`Formats::builtin()` registers all five, and `--format` accepts their names.

Migration:

1. A test that reads piped output passes `--format pretty`, or reads `tsv`.
2. A CLI that needs the old piped output sets the piped default to pretty: `Formats::builtin().piped_default(FormatName::PRETTY)`.
