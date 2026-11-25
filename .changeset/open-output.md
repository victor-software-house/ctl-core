---
ctl-core: minor
---

Output formats are an open interface.
`format::Format` defines how a result becomes output, `format::FormatName` names one, and `format::Formats` is the set a CLI registers with `App::formats`.
`pretty` and `json` are the built-in implementations, `format::PRETTY` and `format::JSON`, and their output is unchanged.
A CLI adds a format, replaces a built-in of the same name, or removes one.
It also sets the default for a terminal and for a pipe.
The terminal default is `pretty` and the piped default is `tsv`, the break that the tabular formats entry describes.
Output on a terminal does not change, except where the breaks below say so.

Breaking changes:

1. `format::OutputFormat` and `format::ParseFormatError` are removed. `FormatName::PRETTY` and `FormatName::JSON` replace its variants, and `view.format() == FormatName::JSON` replaces `is_json()`.
2. `View::new` takes a `&'static dyn Format`, such as `&format::JSON`, in place of an `OutputFormat`. `View::requested` takes an optional name that `View::resolve` turns into a format.
3. `View::format()` returns a `FormatName`, and `View::is_machine()` says whether the format never writes ANSI. `--quiet` hides `pretty` and a default format, and never a machine format that a flag, a shorthand, or a variable named.
4. The `format` field of `OutputArgs`, `FormatArgs`, and `FormatLong` is an `Option<FormatName>`, and is `None` when no flag or variable named a format. `--format` has no default in help.
5. `shell::view` takes an `Option<FormatName>`. Every format but `json` prints the script as text, and quiet never hides it.
6. `App::new` requests no format, so an `App` without `.view(...)` now honours `--format`, `--json`, and their variables.
7. Two variables that name different formats, such as `TOY_FORMAT=csv` and `TOY_JSON=1`, are a usage error that names both. Before, `TOY_JSON` won.
8. `--json` moves after `--quiet` in help and in the Usage spec, and `--format` no longer shows `[default: pretty]`.
9. `FormatName` has no `FromStr`, `Default`, `Deserialize` or `JsonSchema`.
10. `View::format()` is no longer `const`.
11. The `view` feature depends on `erased-serde`. It appears in `Output::data`, and `format` re-exports it.

Migration:

1. Replace `OutputFormat::Json` with `FormatName::JSON`, and `format.is_json()` with `view.format() == FormatName::JSON` where the branch writes JSON. Use `view.is_machine()` only where a check guards ANSI or `--quiet`.
2. Replace `View::new(OutputFormat::X, color)` with `View::new(&format::X, color)`.
3. Compare the `format` field with `Some(FormatName::JSON)`, or call `view()`.
4. Pass an `Option<FormatName>` to `shell::view`, which `cli.output.format` already is.
5. Replace `View::new(cli.output.format, color)` with `View::requested(cli.output.format, color)`.
6. Match the text of a name against `formats.names()` where `FormatName::from_str` or `Deserialize` read one.
7. Register any format of the CLI's own through `App::formats`. Build a `Surface` or an operator document from `formats.augment(Cli::command())`, with the same `Formats` the `App` gets.
