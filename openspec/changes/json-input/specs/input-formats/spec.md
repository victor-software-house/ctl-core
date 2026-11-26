## ADDED Requirements

### Requirement: JSON is a built-in input format

ctl-input SHALL provide a `JSON` format named `json` that reads files with the extension `json`.
`Formats::builtin()` SHALL hold it when ctl-input's `json` feature is on.
ctl-core's `input-json` feature SHALL turn that feature on, and ctl-core's `input` feature SHALL NOT.

#### Scenario: A JSON config file

- **WHEN** a CLI built with `input-json` calls `Input::open("app.json")` on a file that holds `{"port": 8080}`, and loads it into a type with a `port: u16` field
- **THEN** the field reads 8080

#### Scenario: JSON is off

- **WHEN** a CLI built with `input` and not `input-json` calls `Input::open("app.json")`
- **THEN** it fails with one problem that names `.json` as unsupported

### Requirement: JSON reads by the strict RFC 8259 grammar

The JSON format SHALL read only text that RFC 8259 allows, with no option that loosens it.
It SHALL refuse comments, trailing commas, single quotes, unquoted keys, whitespace other than space, tab, line feed and carriage return, a raw control character inside a string, a lone surrogate escape, and any number RFC 8259 does not allow.
A syntax error SHALL be one problem at the token that broke the grammar, worded with no parser names.

#### Scenario: A trailing comma

- **WHEN** `app.json` holds `{"a": 1,}` on line 1
- **THEN** reading it fails with one problem on line 1, column 9

#### Scenario: A leading zero

- **WHEN** `app.json` holds `{"port": 08080}` on line 1
- **THEN** reading it fails with one problem on line 1, column 10

### Requirement: A JSON text holds one value

The JSON format SHALL refuse an empty text and any text after the root value.
It SHALL skip a leading byte-order mark, and the mark SHALL count as no column.

#### Scenario: Text after the value

- **WHEN** `app.json` holds `{} {}` on line 1
- **THEN** reading it fails with one problem on line 1, column 4, that reads `text after the value`

#### Scenario: A byte-order mark

- **WHEN** `app.json` starts with U+FEFF and then holds `{"port": "x"}` on line 1, and the field is `u16`
- **THEN** loading fails with one problem on line 1, column 10

### Requirement: Each JSON scalar keeps its span and syntax

Every JSON value and every key SHALL be a node with its byte span, a string's span including its quotes.
An object SHALL keep its entries in written order.
A number SHALL keep its source text as its value, with the syntax `Integer` when it has no fraction or exponent and `Float` otherwise.
`null` SHALL have the syntax `Null`.

#### Scenario: A number keeps its text

- **WHEN** a JSON text holds `{"id": 18446744073709551616}` and the field is `u128`
- **THEN** the field reads 18446744073709551616

### Requirement: JSON scalars resolve by their syntax

The JSON format SHALL resolve a scalar by its syntax and never by its text.
A string SHALL fill only a string field.
An integer SHALL fill an integer field that holds it, and a float field.
A float SHALL fill a float field when `f64` holds it finitely.
`true` and `false` SHALL fill only a bool field.
`null` SHALL fill a unit field, and an optional field SHALL read it as none.

#### Scenario: A quoted number

- **WHEN** a JSON text holds `{"port": "8080"}` on line 1 and the field is `u16`
- **THEN** loading fails with one problem on line 1, column 10, that reads `expected an integer`

#### Scenario: A null for an optional field

- **WHEN** a JSON text holds `{"name": null}` and the field is `Option<String>`
- **THEN** the field reads none

#### Scenario: A float too large

- **WHEN** a JSON text holds `{"ratio": 1e400}` and the field is `f64`
- **THEN** loading fails with one problem for `ratio` that reads `expected a float`

### Requirement: A strict JSON reading refuses a duplicate key

Keys SHALL compare after their escapes are decoded.
A strict reading SHALL refuse the second of two equal keys in one object, with one problem on the second key that names it.
A listing reading SHALL keep both entries.

#### Scenario: An escaped duplicate

- **WHEN** `app.json` holds `{"name": "a",` on line 1 and `"\u006eame": "b"}` on line 2
- **THEN** loading fails with one problem on line 2, column 1, that reads ``duplicate key `name` ``

#### Scenario: Listing keeps both

- **WHEN** the same text is listed with `top_level`
- **THEN** it lists two entries named `name`, on lines 1 and 2

### Requirement: JSON reads under the shared limits

The JSON format SHALL apply `max_reader_input_bytes` before it reads the first token, `max_depth` when it opens an array or object, and `max_nodes` when it adds a node.
Each SHALL be set through the same tiers as for YAML and TOML, and a refusal SHALL name the field.

#### Scenario: Nesting past the depth limit

- **WHEN** `max_depth` is 8 and a JSON text nests nine arrays
- **THEN** reading it fails with one problem at the ninth `[` that names `max_depth`

### Requirement: The JSON reader passes JSONTestSuite

The JSON format SHALL read every `y_` file of JSONTestSuite at commit 1ef36fa except the two duplicate-key files, which a strict reading SHALL refuse.
It SHALL refuse every `n_` file.
Each `i_` file SHALL have one recorded verdict that a test checks.

#### Scenario: The suite

- **WHEN** the conformance test reads each of the 318 files of `test_parsing` through `Input`
- **THEN** every outcome matches its verdict, and a file without a verdict fails the test
