# Research: a JSON reader for ctl-input

Retrieved 2026-11-26 (UTC).
Each crate version below is the latest stable release on crates.io on that date.
Each code link is pinned to the commit in that release's `.cargo_vcs_info.json`.
Sizes are estimates; nothing was compiled.

## 1. Answer

Write a small strict reader inside ctl-input.
It is a byte scanner that yields spanned tokens, driven by one loop with an explicit stack.
Expect 450 to 650 lines without tests, checked against JSONTestSuite.
The close alternative is to depend on `json_scanner` 0.1.0 and write only the loop.

Duplicate refusal and the limits live in ctl-input's tree-building loop in either design.
The other needs must come from the reader.
`json_scanner` is the only crate found that gives spans on every event (keys included), numbers as source text and a strict grammar.
It has a pull API, no recursion, no `unsafe` and one dependency, `memchr`.

Owning the reader adds about 300 to 400 lines of scanner and string code, plus conformance work.
Depending on `json_scanner` saves them but ties ctl-input to a crate with one release (2025-10-20), no commits since, zero GitHub stars and a closed `ErrorKind` enum.
This report recommends owning the reader, with `json_scanner` as the fallback.

`serde_json` gives spans only through `raw_value` and pointer arithmetic, which re-scans every byte once per nesting level.

## 2. What the tree needs

1. A byte span for each value and each key.
2. Each scalar's value and syntax class.
   For numbers that means the source text, because [RFC 8259 section 6][rfc6] sets no limit on range or precision.
3. A refusal of a duplicate key, with the repeated key's span.
   [Section 4][rfc4] says names SHOULD be unique and that behaviour on duplicates is unpredictable, so refusing is allowed.
4. Depth and node-count limits enforced while parsing, and syntax errors with a byte offset.
   [Section 9][rfc9] lets a parser limit size, depth, number range and string length.
5. Strict RFC 8259.
   Whitespace is only space, tab, LF and CR ([section 2][rfc2]).
   Control characters U+0000 to U+001F must be escaped ([section 7][rfc7]).
   A parser MAY ignore a leading BOM ([section 8.1][rfc81]).
   Unpaired surrogate escapes are legal but unpredictable ([section 8.2][rfc82]).

A crate that builds its own tree first allocates the whole document before ctl-input can apply its node limit.

## 3. Comparison

| Candidate | Value spans | Key spans | Duplicate keys | Depth limit | Number source text | Strict RFC 8259 | `unsafe` | Runtime deps |
|:--|:--|:--|:--|:--|:--|:--|:--|:--|
| serde_json 1.0.154 | Only via `raw_value` | Only via `raw_value` | Visitor can detect | 128, or off | Via `RawValue` | Yes | 16 matches | 5 ([list][sj-deps]) |
| json_scanner 0.1.0 | Yes | Yes | No, adapter can | None; adapter enforces | Yes, `&str` | Yes, runs JSONTestSuite | None | memchr |
| nojson 0.3.15 | Yes | Yes | No | Fixed 128 | Yes | Strict mode (partial read) | Forbidden | None |
| json-event-parser 0.3.0 | Derivable | Derivable | No | 65,536, configurable | Yes | Yes (partial read); skips BOM | 2 matches | None |
| struson 0.8.0 | Derivable | Derivable (not run) | Built in, opt-in, with position | 128, configurable | Yes | Yes; rejects BOM | None found | thiserror |
| json-syntax 0.13.1 | Yes | Yes | Preserved | None | Yes | Yes | 17 matches | 9 ([list][jy-deps]) |
| jsonc-parser 0.34.0 | Yes | Yes | No | Fixed 512 | Yes | No, lenient | None found | None |
| jiter 0.19.0 | Derivable | Derivable | Only in `python` feature | 200 in its builder | Yes | Partial read | 60 matches | 5 |
| json-spanned-value 0.2.2 | Yes | Yes | Refused, no span | serde_json's 128 | serde_json `Number` | serde_json's | Forbidden | serde, serde_json |
| Own reader | Yes | Yes | Yes, with span | Configurable | Yes | By construction | None | None |

`unsafe` counts are `rg -c -w unsafe` over `src/`, so they include comments and inline tests.
"Partial read" means the cell rests on error types and string code only.
Links to issues and third-party pages cannot be pinned to a commit.

## 4. serde_json

### 4.1 No public position on valid input

The `Read` trait is [sealed][sj-sealed], and its `byte_offset` is [`doc(hidden)`][sj-offset].
The only public offset counts bytes [between top-level values][sj-stream].
`Error` exposes only [`line()`][sj-line] and [`column()`][sj-col].
Upstream issues for spans and error offsets are open: [#637][sj-637] (opened 2020-03-27) and [#1023][sj-1023] (opened 2023-06-12).

### 4.2 Spans through raw_value

With `raw_value`, `deserialize_raw_value` [buffers exactly the value's bytes][sj-raw].
For `&str` input the text is a [subslice of the input][sj-borrow].
A borrowed `&RawValue` gives a span as its pointer minus the input pointer, plus its length.
[Map keys][sj-key] take the same path, quotes included.

Costs:

1. Each container is taken as raw text and parsed again, so a byte at depth d is scanned about d+1 times; each key is scanned raw and decoded again.
2. Duplicate detection and order are free.
3. The [depth limit of 128][sj-depth] applies per `Deserializer`, so each nested parse starts a fresh count and ctl-input must count depth itself.

### 4.3 Spans through an io::Read wrapper

json-spanned-value [records the position of every byte][jsv-reader] serde_json pulls.
It relies on `IoRead` reading [one byte at a time][sj-iobytes], an implementation detail, and forces [`from_reader`][jsv-lib], which cannot borrow.

### 4.4 Numbers

A `RawValue` of a number is its source text.
Without `raw_value` or `arbitrary_precision`, an integer above u64 [becomes an f64][sj-overflow] and loses precision.
The [`arbitrary_precision`][sj-arb] feature hands a foreign visitor a [one-entry map][sj-tok] keyed by a [private token][sj-num].
Cargo features unify across a build, so enabling it changes every serde_json user in the binary.

### 4.5 Verdict

serde_json works only through `raw_value` and re-parsing, which costs more code than a small reader.
ctl-core already declares serde_json as an optional dependency for its `json` output feature, so reuse adds no new crate.

## 5. Crates that report positions

### 5.1 json_scanner 0.1.0

Every `next_event` returns a [`SpannedEvent`][js-ev] with [`start` and `end`][js-ev2].
Key spans include the quotes ([docs][js-doc]).
Numbers are [unparsed `&str`][js-num].
Nesting uses a [`Vec` stack][js-stack] with no limit, so the adapter enforces depth.
Whitespace is [exactly the four RFC bytes][js-ws], so a BOM is rejected.
Leading zeros, raw control characters and unmatched surrogates are errors ([zeros][js-zero], [control][js-ctl], [surrogates][js-surrogate]).
Errors carry a [`byte_offset`][js-err] and a [closed `ErrorKind`][js-kind].
Its [conformance test][js-tests] runs JSONTestSuite and expects only `y_` files to succeed.
[Nickel depends on it][nickel].

### 5.2 nojson 0.3.15

`RawJson::parse` builds an index of every value.
Each value exposes [`position()`][nj-pos] and [`as_raw_str()`][nj-raw], and [`to_object()`][nj-obj] yields keys as values.
It has no dependencies and [forbids `unsafe`][nj-unsafe].
Strict mode has [comments and trailing commas off][nj-strict], and it [rejects unpaired surrogates][nj-surrogate].
Depth is a [fixed 128][nj-depth].
The index is built before ctl-input sees a node, so a node limit runs too late.

### 5.3 json-event-parser 0.3.0

Events carry [numbers as text][je-num], and [control characters in strings are errors][je-ctl].
There is no per-event position.
`parse_next` returns [`consumed_bytes`][je-consumed], whose slice starts at the previous event's end and can begin with whitespace, `,` or `:`.
A leading UTF-8 BOM is [skipped][je-bom], which RFC 8259 allows.
The open-container limit defaults to [65,536][je-lim1] and is [configurable][je-lim2].

### 5.4 struson 0.8.0

Defaults are RFC-compliant: [depth 128][st-depth], and a [BOM is rejected][st-bom].
[Duplicate detection][st-dup] is built in but off by default.
Its [error][st-duperr] carries the name and the start position, with no end offset.
`restrict_number_values` is [on by default][st-restrict] and rejects numbers over 100 characters; ctl-input would turn it off and use [`next_number_as_str`][st-numstr].
`data_pos` is [documented][st-datapos] as the current or the next data unit.
The source [adds a peeked token's length only when consumed][st-peek], so after `peek()` it should be the value's first byte.

### 5.5 json-syntax 0.13.1

It [describes itself][jy-doc] as a strict RFC 8259 parser with numbers in lexical form, duplicates preserved and no stack overflow.
Parsing returns a [`JsonCodeMap`][jy-map] with a span per fragment, [one per key][jy-key].
It has no depth limit by design ([heap stack][jy-stack]) and builds its own [`JsonObject`][jy-obj] tree first.
It pulls nine runtime crates including `lexical`.

### 5.6 jsonc-parser 0.34.0

Every [AST node has a `Range` and numbers keep their text][jc-ast].
Defaults [accept comments, trailing commas, missing commas, single quotes and hex numbers][jc-lenient].
With every option off it still accepts [unescaped control characters][jc-str], and it skips [vertical tab, form feed and any Unicode whitespace][jc-ws].
No option turns those off, and depth is [fixed at 512][jc-depth].

### 5.7 jiter 0.19.0

`Jiter` exposes [`current_index()`][ji-idx] and [`next_number_bytes()`][ji-numb], so spans are derivable.
[Duplicate checking][ji-dup] exists only in the `python` feature, and its [value type keeps duplicates][ji-val].
It carries SIMD code, 60 `unsafe` matches and five default dependencies.

### 5.8 Rejected without deeper study

1. spanned_json_parser 0.2.0: line and column only, objects in a [`HashMap`][sp-val] so [order and duplicates are lost][sp-val2]; last release 2023.
2. json-spanned-value 0.2.2: last release 2020, relies on serde_json internals (section 4.3).
3. sonic-rs and simd-json: SIMD parsers with hundreds of `unsafe` matches.
   sonic-rs exposes [raw text of a `LazyValue`][sonic] but no key or offset API was found by grep.
   simd-json's [tape `Node`][simd] has no offset.
4. actson 2.1.0: offsets derivable from [`parsed_bytes()`][ac-bytes]; number text sits in the buffer [`current_str()` returns][ac-str], though that method is documented only for strings.
5. jstrict 0.16.2: strict with a code map, but [10 required dependencies][jstrict-deps] including `lexical`, `hashbrown` and `stacker`.
6. justjson 0.3.0: [offsets only in errors][justjson], last release 2023.

## 6. Writing our own reader

### 6.1 Size

Non-blank, non-comment lines before the test module, from the release tarballs: json_scanner about 570 (pull events, `Vec` stack), nojson about 380 (recursive descent into an index), miniserde 0.1.46 about 610 ([explicit stack][miniserde]).

The ctl-input estimate is inferred from these references and not measured.
The scanner takes 150 to 200 lines, and strings another 150 to 200, since `&str` already guarantees UTF-8.
The tree-building loop, with its stack, limits and duplicate check, takes 150 to 250.
That is 450 to 650 lines without tests.

### 6.2 RFC 8259 edge cases to get right

1. Whitespace: only `0x20`, `0x09`, `0x0A`, `0x0D` ([section 2][rfc2]); vertical tab, form feed and U+2060 are errors.
2. BOM: decide once; RFC 8259 allows ignoring it ([section 8.1][rfc81]), and JSONTestSuite marks it implementation-defined.
3. Numbers: `-? (0 | [1-9][0-9]*) (\.[0-9]+)? ([eE][+-]?[0-9]+)?`; no leading `+`, no leading zeros, no bare `.`, no `Inf` or `NaN` ([section 6][rfc6]).
   Keep the source slice; classify integer versus float by the presence of `.`, `e` or `E`; leave overflow to the typed deserializer.
4. Strings: reject raw U+0000 to U+001F; allow only `\" \\ \/ \b \f \n \r \t \uXXXX` ([section 7][rfc7]); accept U+007F, U+2028 and U+2029 raw.
5. Surrogates: combine a `\uD800-\uDBFF` with a following `\uDC00-\uDFFF`; reject a lone or inverted surrogate, since a Rust `String` cannot hold one ([section 8.2][rfc82] leaves this to the implementation).
6. Keys: compare after unescaping, so `"a"` and `"\u0061"` are duplicates; report the second key's span.
7. Literals: `true`, `false`, `null` only, lower case, followed by a delimiter.
8. Top level: any value, including a scalar; nothing but whitespace after it; an empty document is an error.
9. Limits: check input size before parsing; check depth when opening a container and node count when adding a node, so the error comes before any further allocation ([section 9][rfc9]).

### 6.3 Conformance suite

[JSONTestSuite at commit 1ef36fa][suite] (2024-11-22, MIT) has 318 files in `test_parsing`: 95 `y_`, 188 `n_` and 35 `i_`.

1. All 95 `y_` files must parse, with two deliberate exceptions: [y_object_duplicated_key.json][dupkey] and [y_object_duplicated_key_and_value.json][dupkv] must be refused as duplicate keys.
2. All 188 `n_` files must fail.
   The ones that catch most hand-written readers: `n_structure_100000_opening_arrays` (depth limit), `n_string_unescaped_ctrl_char`, `n_string_unescaped_tab`, `n_array_spaces_vertical_tab_formfeed`, `n_structure_whitespace_formfeed`, `n_structure_whitespace_U+2060_word_joiner`, `n_number_-01`, `n_number_with_leading_zero`, `n_number_2.e3`, `n_number_.2e-3`, `n_number_+1`, `n_number_NaN`, `n_string_1_surrogate_then_escape_u1x`, `n_string_incomplete_surrogate_escape_invalid`, `n_structure_UTF8_BOM_no_data`, `n_object_trailing_comma` and `n_structure_object_with_comment`.
3. The `i_` files need a chosen verdict each:
   1. `i_number_*` (10 files: huge exponents, overflowing reals, integers beyond i64 and u64): accept, since the reader keeps the source text and the typed deserializer decides.
   2. Lone or bad surrogate escapes (`i_object_key_lone_2nd_surrogate`, `i_string_1st_surrogate_but_2nd_missing`, `i_string_1st_valid_surrogate_2nd_invalid`, `i_string_incomplete_surrogate_and_escape_valid`, `i_string_incomplete_surrogate_pair`, `i_string_incomplete_surrogates_escape_valid`, `i_string_invalid_lonely_surrogate`, `i_string_invalid_surrogate`, `i_string_inverted_surrogates_U+1D11E`, `i_string_lone_second_surrogate`): reject.
   3. Invalid UTF-8 (`i_string_UTF-8_invalid_sequence`, `i_string_UTF8_surrogate_U+D800`, `i_string_invalid_utf-8`, `i_string_iso_latin_1`, `i_string_lone_utf8_continuation_byte`, `i_string_not_in_unicode_range`, `i_string_overlong_sequence_2_bytes`, `i_string_overlong_sequence_6_bytes`, `i_string_overlong_sequence_6_bytes_null`, `i_string_truncated-utf-8`): rejected before the reader runs if ctl-input reads the file into a `String`; if the reader takes bytes, it must validate UTF-8 itself.
   4. UTF-16 input (`i_string_UTF-16LE_with_BOM`, `i_string_utf16BE_no_BOM`, `i_string_utf16LE_no_BOM`): reject.
   5. `i_structure_UTF-8_BOM_empty_object`: the BOM decision in 6.2 item 2.
   6. `i_structure_500_nested_arrays`: accepted only if the configured depth is at least 500; it doubles as the depth-limit test.
4. The suite's MIT license permits vendoring the files into `tests/`, as json_scanner and json-syntax do.

## 7. Binary size

No crate was compiled; every figure is an estimate.

1. serde_json: a small derive-based program measured 23.8 KiB of `.text` with cargo-bloat ([source][rs4ts]).
   A WASM build measured +18.2 kB brotli for `serde_json::from_str` against +5.6 kB for miniserde ([leptos#5008][leptos]).
2. json_scanner, nojson and an own reader resemble miniserde's parser, so likely the low tens of KiB of `.text` or less (inferred from line counts).
3. json-event-parser and struson have more code than json_scanner, with no SIMD.
4. jiter, sonic-rs, simd-json, json-syntax and jstrict are larger, from SIMD paths, `lexical`, `num-bigint` or many dependencies.

## 8. What was not verified

1. No binary-size measurement of any candidate or of an own reader.
2. No candidate was run against JSONTestSuite; conformance claims come from each crate's own tests and source.
3. Span derivation for struson, json-event-parser and jiter, and serde_json's map key as `RawValue`, was read from source, not run.
4. json-syntax's per-key code-map entry was read from the builder used for printing; the parse side was not traced.
5. The "partial read" cells (json-event-parser, nojson, jiter) rest on error types and string code, not a full grammar review.
6. actson's strictness was not checked.
7. Whether ctl-input already links serde_json.
8. The two serde_json size figures come from third-party pages and were not reproduced.

[rfc2]: https://www.rfc-editor.org/rfc/rfc8259#section-2
[rfc4]: https://www.rfc-editor.org/rfc/rfc8259#section-4
[rfc6]: https://www.rfc-editor.org/rfc/rfc8259#section-6
[rfc7]: https://www.rfc-editor.org/rfc/rfc8259#section-7
[rfc81]: https://www.rfc-editor.org/rfc/rfc8259#section-8.1
[rfc82]: https://www.rfc-editor.org/rfc/rfc8259#section-8.2
[rfc9]: https://www.rfc-editor.org/rfc/rfc8259#section-9
[sj-sealed]: https://github.com/serde-rs/json/blob/20572a6d066958542d79cdd6cf1a7cd9f0c8e1f8/src/read.rs#L26-L28
[sj-offset]: https://github.com/serde-rs/json/blob/20572a6d066958542d79cdd6cf1a7cd9f0c8e1f8/src/read.rs#L58-L61
[sj-stream]: https://github.com/serde-rs/json/blob/20572a6d066958542d79cdd6cf1a7cd9f0c8e1f8/src/de.rs#L2424
[sj-line]: https://github.com/serde-rs/json/blob/20572a6d066958542d79cdd6cf1a7cd9f0c8e1f8/src/error.rs#L32
[sj-col]: https://github.com/serde-rs/json/blob/20572a6d066958542d79cdd6cf1a7cd9f0c8e1f8/src/error.rs#L44
[sj-637]: https://github.com/serde-rs/json/issues/637
[sj-1023]: https://github.com/serde-rs/json/issues/1023
[sj-raw]: https://github.com/serde-rs/json/blob/20572a6d066958542d79cdd6cf1a7cd9f0c8e1f8/src/de.rs#L1285-L1292
[sj-borrow]: https://github.com/serde-rs/json/blob/20572a6d066958542d79cdd6cf1a7cd9f0c8e1f8/src/read.rs#L665-L674
[sj-key]: https://github.com/serde-rs/json/blob/20572a6d066958542d79cdd6cf1a7cd9f0c8e1f8/src/de.rs#L2294-L2295
[sj-depth]: https://github.com/serde-rs/json/blob/20572a6d066958542d79cdd6cf1a7cd9f0c8e1f8/src/de.rs#L63
[sj-iobytes]: https://github.com/serde-rs/json/blob/20572a6d066958542d79cdd6cf1a7cd9f0c8e1f8/src/read.rs#L153
[sj-overflow]: https://github.com/serde-rs/json/blob/20572a6d066958542d79cdd6cf1a7cd9f0c8e1f8/src/de.rs#L462-L492
[sj-arb]: https://github.com/serde-rs/json/blob/20572a6d066958542d79cdd6cf1a7cd9f0c8e1f8/Cargo.toml#L82
[sj-tok]: https://github.com/serde-rs/json/blob/20572a6d066958542d79cdd6cf1a7cd9f0c8e1f8/src/de.rs#L127
[sj-num]: https://github.com/serde-rs/json/blob/20572a6d066958542d79cdd6cf1a7cd9f0c8e1f8/src/number.rs#L18
[sj-deps]: https://crates.io/api/v1/crates/serde_json/1.0.154/dependencies
[jsv-reader]: https://github.com/MaulingMonkey/json-spanned-value/blob/f68925d39408d4ca60ef96c9cf6ace576e8f9eff/src/reader.rs#L86-L90
[jsv-lib]: https://github.com/MaulingMonkey/json-spanned-value/blob/f68925d39408d4ca60ef96c9cf6ace576e8f9eff/src/lib.rs#L39-L43
[js-ev]: https://github.com/nickel-lang/json-scanner/blob/2f4ad81232f3af5c64c79c9c66f6dc6f15f6de15/src/lib.rs#L220-L225
[js-ev2]: https://github.com/nickel-lang/json-scanner/blob/2f4ad81232f3af5c64c79c9c66f6dc6f15f6de15/src/lib.rs#L261
[js-doc]: https://github.com/nickel-lang/json-scanner/blob/2f4ad81232f3af5c64c79c9c66f6dc6f15f6de15/src/lib.rs#L20-L29
[js-num]: https://github.com/nickel-lang/json-scanner/blob/2f4ad81232f3af5c64c79c9c66f6dc6f15f6de15/src/lib.rs#L208
[js-stack]: https://github.com/nickel-lang/json-scanner/blob/2f4ad81232f3af5c64c79c9c66f6dc6f15f6de15/src/lib.rs#L189
[js-ws]: https://github.com/nickel-lang/json-scanner/blob/2f4ad81232f3af5c64c79c9c66f6dc6f15f6de15/src/lib.rs#L458-L459
[js-zero]: https://github.com/nickel-lang/json-scanner/blob/2f4ad81232f3af5c64c79c9c66f6dc6f15f6de15/src/lib.rs#L755
[js-ctl]: https://github.com/nickel-lang/json-scanner/blob/2f4ad81232f3af5c64c79c9c66f6dc6f15f6de15/src/lib.rs#L606
[js-surrogate]: https://github.com/nickel-lang/json-scanner/blob/2f4ad81232f3af5c64c79c9c66f6dc6f15f6de15/src/lib.rs#L571
[js-err]: https://github.com/nickel-lang/json-scanner/blob/2f4ad81232f3af5c64c79c9c66f6dc6f15f6de15/src/lib.rs#L56
[js-kind]: https://github.com/nickel-lang/json-scanner/blob/2f4ad81232f3af5c64c79c9c66f6dc6f15f6de15/src/lib.rs#L121-L123
[js-tests]: https://github.com/nickel-lang/json-scanner/blob/2f4ad81232f3af5c64c79c9c66f6dc6f15f6de15/tests/inputs.rs
[nickel]: https://github.com/nickel-lang/nickel/blob/219562611995ef3082be5141b6abe3b58ff76830/Cargo.toml#L81
[nj-pos]: https://github.com/sile/nojson/blob/95a6b72b6cd19436611d5239f69cac847d0b2434/src/raw.rs#L709
[nj-raw]: https://github.com/sile/nojson/blob/95a6b72b6cd19436611d5239f69cac847d0b2434/src/raw.rs#L792
[nj-obj]: https://github.com/sile/nojson/blob/95a6b72b6cd19436611d5239f69cac847d0b2434/src/raw.rs#L1057
[nj-unsafe]: https://github.com/sile/nojson/blob/95a6b72b6cd19436611d5239f69cac847d0b2434/src/lib.rs#L219
[nj-strict]: https://github.com/sile/nojson/blob/95a6b72b6cd19436611d5239f69cac847d0b2434/src/parse.rs#L33-L37
[nj-surrogate]: https://github.com/sile/nojson/blob/95a6b72b6cd19436611d5239f69cac847d0b2434/src/parse.rs#L378-L401
[nj-depth]: https://github.com/sile/nojson/blob/95a6b72b6cd19436611d5239f69cac847d0b2434/src/parse.rs#L18
[je-num]: https://github.com/oxigraph/json-event-parser/blob/cb0ebe74098c7095a5176f136d51430a320f9e36/src/lib.rs#L30-L32
[je-ctl]: https://github.com/oxigraph/json-event-parser/blob/cb0ebe74098c7095a5176f136d51430a320f9e36/src/read.rs#L929
[je-consumed]: https://github.com/oxigraph/json-event-parser/blob/cb0ebe74098c7095a5176f136d51430a320f9e36/src/read.rs#L1171-L1176
[je-bom]: https://github.com/oxigraph/json-event-parser/blob/cb0ebe74098c7095a5176f136d51430a320f9e36/src/read.rs#L656-L662
[je-lim1]: https://github.com/oxigraph/json-event-parser/blob/cb0ebe74098c7095a5176f136d51430a320f9e36/src/read.rs#L11
[je-lim2]: https://github.com/oxigraph/json-event-parser/blob/cb0ebe74098c7095a5176f136d51430a320f9e36/src/read.rs#L396
[st-depth]: https://github.com/Marcono1234/struson/blob/e72a2a11d4cd49c441fb33a263f8d934663f67f6/src/reader/stream_reader.rs#L392
[st-bom]: https://github.com/Marcono1234/struson/blob/e72a2a11d4cd49c441fb33a263f8d934663f67f6/src/reader/stream_reader.rs#L4855
[st-dup]: https://github.com/Marcono1234/struson/blob/e72a2a11d4cd49c441fb33a263f8d934663f67f6/src/reader/stream_reader.rs#L350-L369
[st-duperr]: https://github.com/Marcono1234/struson/blob/e72a2a11d4cd49c441fb33a263f8d934663f67f6/src/reader/stream_reader.rs#L1318-L1340
[st-restrict]: https://github.com/Marcono1234/struson/blob/e72a2a11d4cd49c441fb33a263f8d934663f67f6/src/reader/stream_reader.rs#L332-L348
[st-numstr]: https://github.com/Marcono1234/struson/blob/e72a2a11d4cd49c441fb33a263f8d934663f67f6/src/reader/mod.rs#L1601
[st-datapos]: https://github.com/Marcono1234/struson/blob/e72a2a11d4cd49c441fb33a263f8d934663f67f6/src/reader/mod.rs#L473-L485
[st-peek]: https://github.com/Marcono1234/struson/blob/e72a2a11d4cd49c441fb33a263f8d934663f67f6/src/reader/stream_reader.rs#L910
[jy-doc]: https://github.com/timothee-haudebourg/json-syntax/blob/359134460bf12e8896ab620de51843545088bc2e/src/lib.rs#L1-L25
[jy-map]: https://github.com/timothee-haudebourg/json-syntax/blob/359134460bf12e8896ab620de51843545088bc2e/src/code_map.rs#L308-L314
[jy-key]: https://github.com/timothee-haudebourg/json-syntax/blob/359134460bf12e8896ab620de51843545088bc2e/src/code_map.rs#L219-L224
[jy-stack]: https://github.com/timothee-haudebourg/json-syntax/blob/359134460bf12e8896ab620de51843545088bc2e/src/parse/value.rs#L98
[jy-obj]: https://github.com/timothee-haudebourg/json-syntax/blob/359134460bf12e8896ab620de51843545088bc2e/src/object.rs#L83-L85
[jy-deps]: https://crates.io/api/v1/crates/json-syntax/0.13.1/dependencies
[jc-ast]: https://github.com/dprint/jsonc-parser/blob/c7d4cf51f772762e00cc22a62b1454e76dfbe5a2/src/ast.rs#L224-L227
[jc-lenient]: https://github.com/dprint/jsonc-parser/blob/c7d4cf51f772762e00cc22a62b1454e76dfbe5a2/src/parse_to_ast.rs#L51-L89
[jc-str]: https://github.com/dprint/jsonc-parser/blob/c7d4cf51f772762e00cc22a62b1454e76dfbe5a2/src/scanner.rs#L193-L222
[jc-ws]: https://github.com/dprint/jsonc-parser/blob/c7d4cf51f772762e00cc22a62b1454e76dfbe5a2/src/scanner.rs#L371-L393
[jc-depth]: https://github.com/dprint/jsonc-parser/blob/c7d4cf51f772762e00cc22a62b1454e76dfbe5a2/src/parse_to_ast.rs#L283
[ji-idx]: https://github.com/pydantic/jiter/blob/ffaf768d349a16b9323d69db038ca4df802c8285/crates/jiter/src/jiter.rs#L84
[ji-numb]: https://github.com/pydantic/jiter/blob/ffaf768d349a16b9323d69db038ca4df802c8285/crates/jiter/src/jiter.rs#L195
[ji-dup]: https://github.com/pydantic/jiter/blob/ffaf768d349a16b9323d69db038ca4df802c8285/crates/jiter/src/python.rs#L278-L285
[ji-val]: https://github.com/pydantic/jiter/blob/ffaf768d349a16b9323d69db038ca4df802c8285/crates/jiter/src/value.rs#L30-L33
[sp-val]: https://github.com/JulesGuesnon/spanned-json-parser/blob/ea959372a85eb0d54cb132a227b8220fe40df57e/src/value.rs#L30
[sp-val2]: https://github.com/JulesGuesnon/spanned-json-parser/blob/ea959372a85eb0d54cb132a227b8220fe40df57e/src/value.rs#L92-L101
[sonic]: https://github.com/cloudwego/sonic-rs/blob/cec949bef7d4d81cd785fe727f41723fdeb20e53/src/lazyvalue/value.rs#L351
[simd]: https://github.com/simd-lite/simd-json/blob/00a71f6a931f7f365c23d7fc8d1f84ad693304d3/src/value/tape.rs#L76-L100
[ac-bytes]: https://github.com/michel-kraemer/actson-rs/blob/a9c51cb25121c9b93970c2299a31ae2f28dce74c/src/parser.rs#L704
[ac-str]: https://github.com/michel-kraemer/actson-rs/blob/a9c51cb25121c9b93970c2299a31ae2f28dce74c/src/parser.rs#L684-L700
[jstrict-deps]: https://crates.io/api/v1/crates/jstrict/0.16.2/dependencies
[justjson]: https://github.com/khonsulabs/justjson/blob/9d27b776e4f8cedf7d7c6668f839c28c3410ed94/src/error.rs#L7
[miniserde]: https://github.com/dtolnay/miniserde/blob/a477a15268457b9168f3ded0f791c37630b805de/src/json/de.rs#L43
[suite]: https://github.com/nst/JSONTestSuite/tree/1ef36fa01286573e846ac449e8683f8833c5b26a
[dupkey]: https://github.com/nst/JSONTestSuite/blob/1ef36fa01286573e846ac449e8683f8833c5b26a/test_parsing/y_object_duplicated_key.json
[dupkv]: https://github.com/nst/JSONTestSuite/blob/1ef36fa01286573e846ac449e8683f8833c5b26a/test_parsing/y_object_duplicated_key_and_value.json
[rs4ts]: https://rs4ts.dev/21-performance/08-binary-size/
[leptos]: https://github.com/leptos-rs/leptos/issues/5008
