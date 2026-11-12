## ADDED Requirements

### Requirement: Input lists the top-level entries of a text

`Input::top_level` SHALL list the entries of the root mapping or table in the order written.
Each entry SHALL hold the key, the byte range of the value in the text, how the value is written and, when the parser reads it as a string, its text.
A key that is not a string SHALL be left out, and a root that is no mapping SHALL give no entries.
It SHALL exist with `input-yaml` or `input-toml`, and not need `validate`.

#### Scenario: Each way a YAML value is written

- **WHEN** `plain: a plain value`, `single: 'it''s'`, a folded and a literal scalar, `number: 12`, `flag: true`, `nothing: null`, `yes_text: yes`, a list and a mapping are the entries
- **THEN** the written styles are plain, single-quoted, folded, literal, plain, plain, plain, plain, collection and collection, and only the strings and `yes` hold a string

#### Scenario: A key that is no string and a root that is no mapping

- **WHEN** a YAML text has the keys `1`, `'two'` and `z`, or a root that is a sequence, a scalar or empty
- **THEN** the entries are `two` and `z` for the first, and none for the others

#### Scenario: A TOML text

- **WHEN** a TOML text has a basic string, a literal string, a multi-line basic and a multi-line literal string, a number, a date, an array and a table
- **THEN** the written styles are double-quoted, single-quoted, multi-line basic, multi-line literal, plain, plain, collection and collection, and a string under the table is not listed

### Requirement: A listed value range excludes what is not the value

The value range SHALL exclude an anchor and a tag before a YAML value, SHALL include the quotes of a quoted string and the header of a block scalar, and SHALL be `*name` for an alias.

#### Scenario: An anchor, an alias and a tag

- **WHEN** the entries are `a: &x text`, `b: *x`, `c: !!str 12` and `d: &y !!str t`
- **THEN** the value ranges read `text`, `*x`, `12` and `t`, the alias is written as an alias, and every entry holds its string

### Requirement: Listing follows the limits of parsing

The text SHALL pass the limits in force, and a refusal SHALL be the error that `Input::parse` gives, with the same wording.
A YAML input SHALL share the yamled document with `Input::place`.

#### Scenario: A refusal is the parse refusal

- **WHEN** a YAML text holds a duplicate key or a merge key, or goes past `max_depth`
- **THEN** `top_level` gives the error `parse` gives, and for the depth it names `max_depth`

#### Scenario: One document for placing and listing

- **WHEN** a YAML input lists its entries and then places a path
- **THEN** the yamled document is built once
