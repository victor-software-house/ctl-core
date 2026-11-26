---
ctl-core: patch
---

`input-json` reads JSON through `Input::open`, with the built-in format `input::JSON` and the extension `.json`.
`input` does not turn it on, so a CLI that reads JSON names `input-json` beside it.
The reader is ctl-input's own and links no crate.

`ScalarSyntax::Null` joins the tree for `null`; the enum is non-exhaustive, so a `match` on it already has a wildcard arm.
`Formats::builtin()` holds `yaml`, `json` and `toml`, in that order, as their features allow.

A JSON text is read by RFC 8259 with no option that loosens it:

1. A comment, a trailing comma, a single-quoted string, an unquoted key and a number such as `01`, `+1`, `.5` or `NaN` are refused.
2. Whitespace is space, tab, line feed and carriage return only.
3. A raw control character in a string must be escaped, and a lone or inverted surrogate escape is refused.
4. The text holds one value: an empty text, or text after the value, is refused.
5. A leading byte-order mark is skipped and counts as no column.
6. A strict reading refuses a key written twice in one object, comparing keys after their escapes are decoded, and a listing keeps both entries.
7. A string fills only a string field, a number only a number field, and `null` only an optional or unit field.
   A number keeps its text, so an integer too large for `u128` reads as a float, and a float that overflows is refused.
8. `max_reader_input_bytes`, `max_depth` and `max_nodes` apply as the text is read, and a file that is not UTF-8 is refused when it is opened.
