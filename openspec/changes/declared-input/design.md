# Design: declared input

## Decisions

### 1. Two features, by what they pull

`validate` needs only garde. `input` adds `serde`, `serde-saphyr`, and
`yamled`. A CLI that validates template exports but reads no YAML takes
`validate` alone. Considered: one feature. It lost because it would pull a
YAML parser into consumers that never read YAML.

### 2. The reader settings are the family's contract

`Input::parse` sets strict booleans, refuses duplicate keys, and refuses merge
keys. These settings are why the family moved to `serde-saphyr`: with them a
typo'd `yes` or a repeated key fails with a line instead of loading a wrong
value. The public override is to call `serde-saphyr` with other options and
pass the value to `Input::check`; the disable path is not to call
`Input::parse`. Considered: an options parameter. It lost because it would put
`serde-saphyr`'s `Options` type into ctl-core's public API.

### 3. Placement goes through the yamled index

garde reports a field path (`outputs[1].path`). yamled's location index turns
it into a line and column, and a path that does not resolve lands on its
nearest written ancestor. A key holding `.` or `[` cannot be told apart from
nesting in garde's path text; such a problem lands on an ancestor. Considered:
garde's structured path components. They lost because garde exposes them only
through a hidden, unstable method.

### 4. Frontmatter is split here, not by a Markdown crate

The split is a first line of `---` and the next line of `---`. It keeps a
line offset, so a problem in the frontmatter reports the file's own line.
Considered: `gray_matter`. It lost because it brings a second YAML stack and
needed a workaround to stop treating `---` as an excerpt delimiter.
