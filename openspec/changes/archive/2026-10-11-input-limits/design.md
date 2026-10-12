# Design: input limits

## Decisions

### 1. Every Budget field but one, generated from one list

`Limits` mirrors 20 of the 21 fields of serde-saphyr 1.3.0's `Budget`, and one
`macro_rules!` list generates the struct, the field-by-field merge, and the
mapping onto `Budget`, so the three cannot drift. A chosen subset was
considered. It lost because the operator asked for every field, and a subset
leaves the next field someone needs unreachable.

`enforce_alias_anchor_ratio` is left out, by operator decision. The ratio
check is one comparison after scanning, so turning it off saves nothing and
only removes a guard against alias expansion. Its multiplier and minimum alias
count stay overridable.

### 2. Own type, not a re-export of `Budget`

`Budget` has no `Option` per field, so it cannot say "leave this to the next
tier". A re-export would also tie ctl-core's public API to serde-saphyr's.

### 3. Precedence mirrors table width

Width takes an explicit option first, then the operator's environment, then
the built-in. Limits take the call first, then the repository the operator
owns, then the tool's process default, then serde-saphyr's. A tool default
above the repository was considered. It lost because a tool would then
silently override the operator's own config.

### 4. Repository and process tiers are process-wide

`Input` does not know the repository root, so a CLI installs the repository
tier once at startup with `use_repo_limits(root)`. Threading limits through
every call was considered. It lost because every reader, including
frontmatter, would need the root.

### 5. In-memory text gets the byte cap

serde-saphyr applies `max_reader_input_bytes` only to readers. `Input` holds
text already read, so `parse` checks its length against the same field.

### 6. A repository that loosens a limit is reported

A repository outranks the tool, so it can raise a limit past serde-saphyr's
default. `use_repo_limits` returns each such field with its value and the
default, and the CLI prints them as `{bin}: warning: …`, as `App` prints style
variable warnings. Printing inside ctl-core was considered. It lost because
the library does not know the binary name.

## Override and disable

A repository raises any limit except the alias ratio switch in
`.ctl/input.yaml`, and is warned about each one it raises above the default. A tool sets its default
with `set_default_limits`. Every field defaults to serde-saphyr's own value.
