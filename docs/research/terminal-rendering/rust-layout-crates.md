# Rust crates that lay out and render terminal text, tables and widgets

A source survey for the design of a new rich-style rendering library.
Every claim below comes from the published `.crate` source at the version named.
Nothing was compiled or run.
Data collected on 2026-11-07 (UTC).
Download counts come from the crates.io API.
"Recent" means the last 90 days.
GitHub "open" counts include pull requests.

## 1. Answers first

1. **comfy-table 8 cannot keep ANSI-styled cell content without crossterm.**
   Both ways to style a cell sit behind the `tty` feature, and `tty` is the feature that enables the optional `crossterm` dependency.
   The `custom_styling` feature, which makes comfy-table read ANSI already present in cell text, lists `tty` as a requirement.
   With `default-features = false`, a cell string that holds ANSI is not stripped or parsed.
   It is measured with `unicode_width::UnicodeWidthStr::width`, which counts the ESC byte and the sequence text as visible columns.
   Details are in section 3 [ct-cargo] [ct-cell-fg] [ct-measure-plain] [uw-lookup].
2. **Four groups of crates have a measure step in the API: the Rich ports, papergrid, cli-table and textwrap.**
   papergrid has `Estimate` and `Dimension` traits.
   cli-table has `RequiredDimension` and `AvailableDimension`.
   Every Rich port has a `Measurement { minimum, maximum }`.
   textwrap has a `Fragment` trait that carries only widths.
   ratatui does not measure content at all.
3. **No non-Rich crate gives a content-based minimum width (longest word).**
   comfy-table and tabled take minimums from the user or from padding and borders.
   The Rich ports carry a `minimum` field in `Measurement`.
   I did not check how each one computes it.
4. **Width models disagree.**
   Most crates call `unicode-width` and sum or take the string width.
   gilt and rich_rust sum per code point, and gilt documents why.
   rs-rich ships Rich's own width tables and says `unicode-width` differs from them on 348 code points.
   miette, prettytable-rs and termimad still depend on `unicode-width` 0.1.
5. **ANSI in text is handled in three different ways.**
   Strip then measure (comfy-table `custom_styling`, tabled `ansi`, textwrap, cli-table, prettytable-rs).
   Decode into styled spans (the Rich ports have an `AnsiDecoder`).
   Ignore it (ratatui drops the ESC byte and prints the rest of the sequence as text).
6. **Two crates keep style outside the text, and wrapping stays correct.**
   ratatui carries a `Style` on every span and every grapheme through wrapping.
   papergrid keeps colours in a map keyed by position.
   Crates that embed ANSI in strings must close and reopen the style at every cut.

## 2. Versions read

| Crate | Version | Downloads (total / recent) | Last release | Stable releases in 12 months | Open (issues and PRs) | `src` lines |
|:--|:--|:--|:--|:--|:--|:--|
| comfy-table | 8.0.1 | 114.7M / 24.0M | 2026-09-25 | 3 | 4 | 4,154 |
| tabled | 0.22.0 | 49.5M / 14.4M | 2026-09-05 | 2 | 116 | 27,263 |
| papergrid | 0.18.0 | 49.4M / 14.3M | 2026-05-31 | 1 | (same repo as tabled) | 11,915 |
| ratatui | 0.30.2 | 68.8M / 25.8M | 2026-06-19 | 3 | 195 | 2,351 |
| ratatui-core | 0.1.2 | 31.7M / 17.2M | 2026-06-19 | 3 | (ratatui repo) | 25,809 |
| ratatui-widgets | 0.3.2 | 30.1M / 16.7M | 2026-06-19 | 3 | (ratatui repo) | 29,723 |
| kasuari | 0.4.13 | 31.7M / 17.2M | 2026-10-15 | 3 | 3 | 3,100 |
| textwrap | 0.16.4 | 468.3M / 66.3M | 2026-09-13 | 2 | 33 | 4,836 |
| unicode-width | 0.2.3 | 947.3M / 218.8M | 2026-10-28 | 1 | 9 | 23,234 |
| unicode-segmentation | 1.13.3 | 666.9M / 173.4M | 2026-06-01 | 4 | 29 | 6,182 |
| unicode-truncate | 3.0.0 | 70.7M / 29.7M | 2026-08-20 | not counted | not counted | 604 |
| miette | 7.6.0 | 90.7M / 24.7M | 2025-04-27 | 0 | 115 | 8,924 |
| cli-table | 0.5.0 | 7.8M / 1.3M | 2025-03-18 | 0 | 4 | 1,700 |
| prettytable-rs | 0.10.0 | 27.5M / 5.7M | 2022-12-27 | 0 | 49 | 3,332 |
| termimad | 0.35.5 | 9.2M / 2.7M | 2026-09-23 | 7 | 18 | 8,193 |
| rich_rust | 0.2.3 | 203.5k / 81.2k | 2026-08-19 | 6 | 0 | 48,895 |
| rs-rich | 0.0.15 | 3.9k / 3.8k | 2026-11-05 | 15 | 174 | 54,050 |
| fast-rich | 0.4.0 | 1.5k / 1.2k | 2026-11-06 | 6 | 1 | 17,863 |
| rich-rs | 1.3.0 | 1.4k / 0.6k | 2026-09-07 | 12 | 0 | 49,173 |
| richrs | 0.2.1 | 5.9k / 2.5k | 2025-12-11 | 3 | 3 | 20,766 |
| rusty-rich | 0.4.3 | 162 / 10 | 2026-06-04 | 7 | not found | 25,569 |
| gilt | 2.3.1 | 2.6k / 0.5k | 2026-06-25 | 39 | 0 | 99,710 |

`src` lines are `wc -l` over every `.rs` file under `src/`.
For unicode-width, most lines are generated tables.
Other crates include inline tests in these counts.

## 3. comfy-table 8.0.1

### 3.1 What it renders and the API

Static tables, returned as a `String` through `Display`.
Cells hold text split on `\n`.

```rust
use comfy_table::{presets::UTF8_FULL, Cell, Color, ContentArrangement, Table};

let mut table = Table::new();
table
    .load_style(UTF8_FULL)
    .set_content_arrangement(ContentArrangement::Dynamic)
    .set_width(80)
    .set_header(vec!["Name", "Size"])
    .add_row(vec![Cell::new("a").fg(Color::Green), Cell::new("1")]);
println!("{table}");
```

The snippet follows the shipped no-tty example [ct-example] and the cell API [ct-cell-fg].
Only `Cell::fg` needs `tty`.

### 3.2 Styling

- Cell style is `fg`, `bg` and `attributes` on `Cell`, all behind `#[cfg(feature = "tty")]` [ct-cell].
- `Color` and `Attribute` are mirrored enums owned by comfy-table.
  With `reexport_crossterm` they become crossterm's own types [ct-style-mod].
- Output ANSI is produced by `crossterm::style::style(..)` in `style_line` [ct-style-line].
- Whether to emit style at all is decided at render time by `should_style()`, which is `enforce_styling` or "stdout is a terminal" [ct-should-style].

### 3.3 Width model

- `unicode-width` 0.2 and `unicode-segmentation` 1.
- Measuring a line is `s.width()`, or `s.ansi_strip().width()` with `custom_styling` [ct-measure-plain] [ct-measure-ansi].
- Splitting a word that is too long walks `grapheme_indices(true)` and adds each grapheme's width.
  The source comment names zero-width joiners and variation selectors [ct-split-word].
- There is a fast path for printable ASCII.
- It calls `width()` only, never `width_cjk()`, so East Asian ambiguous characters are one column.
- Measuring uses string width but splitting uses grapheme width.
  These can disagree on a sequence where `unicode-width` applies a sequence rule across grapheme boundaries.
  I did not test this.

### 3.4 ANSI handling

- Off by default.
  Without `custom_styling`, escapes in cell text are counted as visible characters.
  The ESC character has width 1 in `str` width [uw-lookup].
  A five-byte `ESC [31m` therefore adds five columns.
- With `custom_styling`, `ansi-str` strips for measuring and `console::AnsiCodeIterator` walks escapes when splitting [ct-measure-ansi] [ct-split-ansi].
- At cell creation, `fix_style_in_split_str` closes and reopens the style around each `\n` [ct-cell-new] [ct-fix-style].
- When `max_height` truncates a cell, the last line is stripped of all ANSI before the cut.
  The source comment says the cut "might break ansi code" otherwise [ct-truncate].

### 3.5 Layout algorithm

The pipeline is measure, arrange, format.

1. `column_max_content_widths` returns the widest line per column, minimum 1, over header and rows [ct-max-widths].
2. `arrange_content` resolves constraints that need no table width.
   These are `ContentWidth`, `Absolute(Fixed)`, `Hidden`, and a `LowerBoundary` already larger than the content [ct-arrange] [ct-constraint].
3. If no table width is known, it falls back to `Disabled`.
   The width is known only from `set_width` or from the terminal [ct-tty-width].
4. `Dynamic` runs these steps [ct-dynamic-doc]:
   1. Remaining width is the table width minus borders, minus padding of undecided columns, minus columns already fixed.
   2. A `LowerBoundary` larger than the average share fixes that column to its minimum.
   3. Any column whose content or `MaxWidth` fits the average share is fixed, and the surplus goes back to the pool.
      This repeats until nothing changes [ct-dynamic-fit].
   4. For the remaining columns it simulates the word wrap at the average width.
      A column can need less than its share after wrapping, so the surplus is returned [ct-dynamic-split].
   5. The rest is divided equally, with at least 1 column each [ct-dynamic-distribute].
   6. `DynamicFullWidth` hands any left-over width back to all columns.
5. `Width::Percentage` is resolved against the table width minus borders, capped at 100.
   It does nothing when the width is unknown [ct-constraint].
6. Formatting wraps each line with `split_line` on a delimiter (a space by default).
   A word wider than the column is cut by grapheme width.
   `max_height` cuts lines and appends a configurable truncation indicator [ct-truncate].

### 3.6 Terminal detection

`tty` feature only.
`Table::width()` asks `crossterm::terminal::size()` when stdout is a terminal.
`is_tty` uses `std::io::IsTerminal` on stdout, or stderr after `use_stderr`, and is cached [ct-tty-width] [ct-is-tty].
`force_no_tty` and `enforce_styling` override it.
No `NO_COLOR` handling was found.

### 3.7 Dependencies

| Configuration | Direct dependencies |
|:--|:--|
| Default (`tty`) | `unicode-segmentation`, `unicode-width`, `crossterm` 0.29 (no default features) |
| `default-features = false` | `unicode-segmentation`, `unicode-width` |
| `custom_styling` | the default set plus `ansi-str` 0.9 and `console` 0.16 |

`crossterm` with no default features pulls `bitflags`, `parking_lot`, `document-features`, and `rustix` on Unix.
Windows adds `crossterm_winapi`.
This comes from the crates.io dependency data for crossterm 0.29.0 [crates-api].

### 3.8 Maintenance

Last release 2026-09-25, 3 stable releases in 12 months, 4 open issues and PRs, 4,154 source lines, `#![forbid(unsafe_code)]` [ct-lib].

### 3.9 Lessons

- Copy: the arrangement doc comment lists the algorithm as numbered steps and the code follows it.
- Copy: the `_debug` feature prints each step of the arrangement.
- Copy: a plain-text path with no terminal dependency, proven by an example that builds with `--no-default-features`.
- Avoid: tying style to a terminal crate.
  A table library does not need a terminal crate to hold a colour.
- Avoid: a measure function that depends on a compile-time feature.
  The same string measures differently by build.
- Avoid: stripping all styling when truncating by height.
- Measure and render: the stages exist in the code but are private.
  Callers cannot ask for a column width without rendering.

## 4. tabled 0.22.0 and papergrid 0.18.0

### 4.1 What they render and the API

tabled builds tables from `Tabled` types or from nested iterators.
papergrid is the engine.
It draws a grid from three inputs: records, a config and a dimension [pg-lib].

```rust
use tabled::{settings::{peaker::Priority, Style, Width}, Table};

let mut table = Table::new([("1", "Hello", 100), ("2", "World", 1000)]);
table.with(Style::modern());
table.with(Width::wrap(15).priority(Priority::max(false)));
println!("{table}");
```

The snippet follows the peaker doc test [tb-peaker].

### 4.2 Styling

- Cells are plain `String` records.
- Colour is separate.
  papergrid's `Colors` trait maps a `Position` to a value that implements `ANSIFmt`, which writes a prefix and a suffix (`ESC[0m` by default) [pg-colors] [pg-ansifmt].
- tabled's `Color` is a static prefix and suffix or an owned buffer [tb-color].
- Border characters, padding, spans and alignment are config values in `SpannedConfig`.

### 4.3 Width model

- `unicode-width` 0.2 only. No `unicode-segmentation`.
- `get_string_width` is `UnicodeWidthStr::width`.
  `get_char_width` is `UnicodeWidthChar::width(c).unwrap_or_default()` [pg-width].
- No ambiguous-width switch.

### 4.4 ANSI handling

- Needs the `ansi` feature, which is off by default [tb-features].
- With it, `get_line_width` uses `ansitok::parse_ansi` and sums only the text elements.
  The comment says this is done because of terminal links that `ansi_str` cannot strip [pg-width].
- Wrapping and truncating use `ansi_str::get_blocks`, so a style is closed and reopened across a cut.
  OSC 8 links are stripped and rebuilt as a prefix and suffix on every output line [tb-wrap-text] [tb-wrap-ansi].
- Without the feature, escapes are counted as characters.

### 4.5 Layout algorithm

1. papergrid separates the estimate from the draw.
   `Estimate::estimate(&mut self, records, config)` fills a `Dimension` with a width per column and a height per row [pg-dimension].
   `IterGridDimension` takes the maximum cell width per column, with spans handled separately [pg-iter-dim].
2. The grid is then built as `IterGrid::new(&records, &cfg, &dims, colors)`.
3. `Width::wrap(n)` and `Width::truncate(n)` are table options that run in the order of `.with(..)` calls.
   `n` is a `usize` or a `Percent` of the table width [tb-wrap] [tb-width-mod] [tb-percent].
4. `Wrap` estimates the dimension, sums widths and borders, and returns if the target is not smaller.
   It then calls `decrease_widths`, which removes one column at a time from the column picked by a `Peaker` until the total fits [tb-decrease] [tb-peaker].
5. `PriorityNone` cycles columns one by one.
   `PriorityMax` picks the widest. `PriorityMin`, `PriorityLeft` and `PriorityRight` also exist [tb-peaker-max].
6. The minimum width per column is the width of empty records under the config, so only padding and borders.
   Content never sets a floor, and a column can shrink to nothing.
7. After deciding widths, `get_decrease_cell_list` finds the cells that are too wide.
   It rewrites those cell strings with hard line breaks (`wrap_text`).
   The records are mutated, and the dimension is re-estimated for heights (`hint_change`) [tb-wrap].

### 4.6 Terminal detection

None.
tabled has no terminal-size dependency.
A caller reads the width and passes it to `Width::wrap`.

### 4.7 Dependencies

| Configuration | tabled direct dependencies | papergrid direct dependencies |
|:--|:--|:--|
| Default features (`derive`, `macros`, `assert`) | `papergrid` (no default features), `tabled_derive`, `testing_table` | `bytecount`, `unicode-width`, `fnv` (`std`) |
| `default-features = false` | `papergrid` (no default features) | `bytecount`, `unicode-width` |
| `ansi` | adds `ansi-str`, `ansitok` | adds `ansi-str`, `ansitok` |

`testing_table` is in the default set because of the `assert` feature, which exposes `tabled::assert` to downstream tests.
Data from the crates.io dependency API [crates-api].
tabled has 27,263 source lines and papergrid 11,915.

### 4.8 Maintenance

tabled: last release 2026-09-05, 2 stable releases in 12 months, 116 open issues and PRs.
papergrid: last release 2026-05-31, 1 stable release in 12 months.
Both are versioned below 1.0 and break the API in minor releases.

### 4.9 Lessons

- Copy: the `Estimate` and `Dimension` split.
  A grid takes a dimension as an argument, so a caller can supply, cache or override it.
- Copy: the `Peaker` trait.
  The shrink policy is a small strategy object.
- Copy: `ANSIFmt` as prefix and suffix, with colours kept outside the text.
- Avoid: rewriting cell strings in place to wrap them.
  After `Wrap`, the original text is gone.
- Avoid: a minimum width that ignores content.
- Avoid: a settings pipeline whose result depends on the order of `.with(..)` calls.

## 5. ratatui 0.30.2, ratatui-core 0.1.2, ratatui-widgets 0.3.2, kasuari 0.4.13

### 5.1 What they render and the API

A retained-buffer terminal UI.
Widgets draw into a `Buffer` of `Cell`s inside a `Rect`.
A backend crate diffs two buffers and writes the changes.
ratatui itself re-exports the pieces and has 2,351 source lines.
The backends (`ratatui-crossterm` and others) are separate crates that I did not read.

```rust
use ratatui::{layout::Constraint, style::Stylize, widgets::{Row, Table}};

let rows = [Row::new(vec!["Cell1", "Cell2", "Cell3"])];
let widths = [Constraint::Length(5), Constraint::Length(5), Constraint::Fill(1)];
let table = Table::new(rows, widths).column_spacing(1).red();
// frame.render_widget(table, area);
```

The snippet follows the `Table` doc test [rt-table-new].
The core trait is `Widget::render(self, area: Rect, buf: &mut Buffer)` [rt-widget].

### 5.2 Styling

- Its own types.
  `Style { fg, bg, underline_color, add_modifier, .. }`, with a sub-modifier set as well [rt-style].
- `Span { style, content: Cow<str> }`, `Line { style, alignment, spans }`, `Text { alignment, style, lines }` [rt-span] [rt-line] [rt-text].
- `Buffer { area, content: Vec<Cell> }`. A `Cell` holds a `CompactString` symbol, colours and modifiers [rt-buffer] [rt-cell].
- Styled text is spans with styles.
  There is an optional `anstyle` conversion [rt-anstyle].

### 5.3 Width model

- `unicode-width` 0.2 and `unicode-segmentation`. `unicode-truncate` for cutting.
- `Buffer::set_stringn` iterates grapheme clusters, drops any grapheme that contains a control character, drops zero-width ones, and stops when the remaining width is used up.
  After a wide symbol it resets the cells it covers [rt-set-stringn].
- `CellWidth for str` uses `UnicodeWidthStr::width`.
  It adds a correction for halfwidth katakana voiced sound marks (U+FF9E, U+FF9F) "for terminal compatibility" [rt-cellwidth].
- A `Cell` can carry `ForcedWidth`, and the diff skips cells covered by a wide symbol [rt-cellwidth] [rt-diff].
- `Span`, `Line` and `Text` implement `UnicodeWidthStr`, including `width_cjk`.
  The default `width()` is the non-CJK one [rt-line-width].

### 5.4 ANSI handling

None.
The ESC character is dropped by the control-character filter, and the rest of the sequence is printed as text.
The crate's own test shows `"I \x1b[0;36mwas\x1b[0m here!"` rendering as `I [0;36mwas[0m here!` [rt-ansi-test].
`set_symbol` accepts escape text on purpose for OSC 8 links and images, but the caller owns the width [rt-cellwidth].

### 5.5 Layout algorithm

1. `Table` does not measure content.
   The caller passes `Constraint`s per column. With none, widths are equal, `max_width / col_count` [rt-table-widths].
2. Column rectangles come from `Layout::horizontal(widths).flex(..).spacing(..).split(area)` [rt-table-widths].
3. `Constraint` has `Min`, `Max`, `Length`, `Percentage`, `Ratio` and `Fill`.
   The doc gives the priority order Min, Max, Length, Percentage, Ratio, Fill [rt-constraint].
4. `Layout::try_split` builds a fresh kasuari `Solver` per call.
   It creates two variables per constraint plus two, which form alternating spacers and segments.
   It adds area, flex, per-constraint and fill constraints, and a "grow" constraint between equal-fill neighbours [rt-layout-solve].
5. Constraint strengths are fixed constants.
   For example `SPACER_SIZE_EQ` is `REQUIRED / 10`, `MIN_SIZE_GE` and `MAX_SIZE_LE` are `STRONG * 100`, `LENGTH_SIZE_EQ` is `STRONG * 10`, `PERCENTAGE_SIZE_EQ` is `STRONG`, and `RATIO_SIZE_EQ` is `STRONG / 10` [rt-strengths].
6. With the `layout-cache` feature (on in ratatui by default), results are cached in an LRU keyed on the layout, because the solver is re-run every frame [rt-layout-solve].
7. kasuari is a Cassowary implementation, `no_std`, with 3,100 lines [ks-lib].
   Cassowary lets a constraint be violated by strength order, which gives graceful degradation.
8. Text wrapping is `WordWrapper` and `LineTruncator`, state machines over `StyledGrapheme`s.
   Because each grapheme carries its style, a wrap needs no escape repair [rt-reflow].

### 5.6 Terminal detection

None in `ratatui-core` or `ratatui-widgets`.
Size and capability come from the backend crate.

### 5.7 Dependencies

| Configuration | Direct dependencies |
|:--|:--|
| `ratatui` default | `ratatui-core`, `ratatui-widgets`, `instability`, `ratatui-crossterm`, `ratatui-macros` |
| `ratatui-core`, no default features | `bitflags`, `compact_str`, `hashbrown`, `itertools`, `kasuari`, `lru`, `strum`, `thiserror`, `unicode-segmentation`, `unicode-truncate`, `unicode-width` |
| `ratatui-widgets`, no default features | `bitflags`, `hashbrown`, `indoc`, `instability`, `itertools`, `line-clipping`, `ratatui-core`, `strum`, `unicode-segmentation`, `unicode-width` |
| `kasuari` | `hashbrown`, `thiserror` |

Optional in ratatui-core: `anstyle`, `critical-section`, `palette`, `serde`.
The `lru` crate is listed as a required dependency of ratatui-core in the crates.io data, while its use in the source is behind the `layout-cache` feature [rt-layout-solve].
I did not check whether the build drops it.
ratatui-core and ratatui-widgets declare `#![no_std]` [rt-nostd].
`unicode-truncate` 3.0.0 was read, but ratatui-core 0.1.2 asks for `^2`.

### 5.8 Maintenance

All three are on 2026-06-19, with 3 stable releases each in 12 months.
ratatui has 195 open issues and PRs.
kasuari was released 2026-10-15, with 3 open.

### 5.9 Lessons

- Copy: styled spans through wrapping, so style never needs repair.
- Copy: a cell grid where a wide symbol owns the cells it covers, with an explicit rule for who resets them.
- Copy: `no_std` core with the terminal outside it.
- Copy: a named constraint vocabulary (`Min`, `Max`, `Length`, `Percentage`, `Ratio`, `Fill`).
- Avoid: a solver for one-dimensional splits if content-based tables are the main use.
  ratatui needs it for 2D UI nesting.
  A column-width pass is a smaller problem.
- Avoid: rendering as the only way to learn a size.
  There is no way to ask a `Widget` how wide it wants to be.
- Measure and render: not separated. Layout is decided from constraints before any content is seen.

## 6. textwrap 0.16.4

### 6.1 What it renders and the API

Wraps a string to a width. It returns `Vec<Cow<str>>`, or a joined `String`.

```rust
use textwrap::{wrap, Options};

let lines = wrap("a long line of text", Options::new(10).break_words(false));
```

`wrap`, `fill`, `fill_inplace`, `refill`, `unfill`, `indent`, `dedent` and `wrap_columns` are the public functions.
`wrap` takes a width or an `Options` [tw-options].

### 6.2 Styling

None. It works on `&str` and keeps any ANSI in the text as it found it.

### 6.3 Width model

- `display_width` skips ANSI escapes and sums `UnicodeWidthChar::width(ch).unwrap_or(0)` per `char` [tw-width] [tw-ch-width].
- With the `unicode-width` feature off, width is a crude guess: 1 below U+1100, otherwise 2.
- The doc of `display_width` shows that a ZWJ emoji family counts 4 with the feature and 6 without, and notes terminals differ on how they draw it [tw-width].
- No grapheme segmentation. No CJK switch.

### 6.4 ANSI handling

`skip_ansi_escape_sequence` skips a CSI sequence up to a final byte in 0x40 to 0x7E.
It skips an OSC sequence up to BEL or ST [tw-skip].
It is used for measuring only.
I found no code that reopens an active style on the next wrapped line.

### 6.5 Layout algorithm

1. `WordSeparator` splits a line into `Word`s, by ASCII space or by Unicode line-break properties through `icu_segmenter` [tw-sep].
2. `WordSplitter` may split words at hyphens or with a hyphenation dictionary.
3. Break words that exceed the width when `break_words` is true.
4. `WrapAlgorithm::FirstFit` is greedy. `OptimalFit(Penalties)` looks at the whole paragraph with SMAWK [tw-algos].
5. Both algorithms take `Fragment`s, a trait with `width()`, `whitespace_width()` and `penalty_width()` returning `f64` [tw-fragment].

Truncation is not offered.

### 6.6 Terminal detection

Optional `terminal_size` feature. `termwidth()` returns the size or 80 [tw-termwidth].

### 6.7 Dependencies

Default features: `std`, `unicode-linebreak` (which pulls `icu_segmenter` with `compiled_data`), `unicode-width`, `smawk`.
With `default-features = false` there are no dependencies [crates-api].
Optional: `hyphenation`, `terminal_size`.

### 6.8 Maintenance

Last release 2026-09-13, 2 stable releases in 12 months, 33 open issues and PRs.

### 6.9 Lessons

- Copy: the `Fragment` trait.
  Wrapping needs widths and nothing else, so any renderer can use the algorithm.
- Copy: wrapping and line-breaking as swappable strategy enums.
- Avoid: a different width answer per feature flag.
- Avoid: wrapping that is not style-aware.

## 7. unicode-width 0.2.3 and unicode-segmentation 1.13.3 (as they affect width)

### 7.1 unicode-width

- `UnicodeWidthChar::width` returns `Option<usize>`, `None` for control characters.
  `UnicodeWidthStr::width` returns `usize` [uw-char].
- The `str` width follows sequence rules.
  Fully-qualified emoji ZWJ sequences, emoji modifier sequences and emoji presentation sequences count 2.
  Text presentation sequences count 1 outside an East Asian context.
  Many Arabic, Buginese, Hebrew and Tifinagh joiners have special rules.
  `\r\n` counts 1 [uw-rules].
- East Asian Wide and Fullwidth count 2.
  East Asian Ambiguous counts 1, or 2 through `width_cjk`, which exists only with the default-on `cjk` feature [uw-rules].
- In `str` width, control characters count 1.
  `\n` counts 1, `\r` counts 0 only before `\n`, and every other C0 control counts 1 [uw-lookup].
- Tables are for Unicode 18.0.0 [uw-version].
- No dependencies. `no_std`.

### 7.2 unicode-segmentation

- `graphemes(true)` yields extended grapheme clusters. `GraphemeCursor` is the incremental form [us-graphemes].
- It gives boundaries only. It does not give width.
- Its tables are for Unicode 17.0.0, one version behind `unicode-width` 0.2.3 [us-version].
  A new emoji can be segmented by one rule set and measured by another.
- A cluster is not a cell count.
  A flag is two code points and two columns.
  A ZWJ family is one cluster and 2 columns in `unicode-width` 0.2, but 6 if each emoji is summed.
- No dependencies. Optional `no_std` feature.

### 7.3 unicode-truncate 3.0.0

A trait on `str` with `unicode_truncate`, `unicode_truncate_start`, `unicode_truncate_centered` and `unicode_pad`.
Each returns the slice and the width it kept [ut-trait].
Dependencies: `itertools`, `unicode-segmentation`, `unicode-width` [crates-api].

### 7.4 Lessons

- Copy: one width function behind one module, with the Unicode version exposed (`UNICODE_VERSION`).
- Copy: the "return the kept width with the slice" shape of `unicode_truncate`.
- Avoid: sum-of-chars width and grapheme width in the same library.
  Pick one and test it on a ZWJ family, a flag, a variation selector and a combining mark.

## 8. miette 7.6.0 (graphical report handler)

### 8.1 What it renders and the API

Error reports with source snippets, labelled spans, a cause chain, related errors and a footer.

```rust
use miette::GraphicalReportHandler;

let mut out = String::new();
GraphicalReportHandler::new().with_width(80).render_report(&mut out, &diagnostic)?;
```

`render_report` writes to any `fmt::Write` [mt-render].
The width setter is `with_width` [mt-with-width].

### 8.2 Styling

`GraphicalTheme { characters, styles }` where `styles` hold `owo_colors::Style` values [mt-theme].
Output is written straight to the writer with colours applied while writing.
There is no intermediate styled-text value.

### 8.3 Width model

`unicode-width` `^0.1.11` [crates-api].
Highlight columns come from per-character `c.width().unwrap_or(0)`, and indents from `str.width()` [mt-width].
The handler has a `tab_width` setting, default 4.

### 8.4 ANSI handling

None on input.
Source text is expected to be plain.
Wrapping is done through `textwrap` with the width set to `termwidth - 2`, and nested renderers reduce `termwidth` by the indent width [mt-wrap].

### 8.5 Layout algorithm

- Text blocks (message, causes, footer) go through `textwrap::Options::new(width)` with the handler's `break_words`, word separator and word splitter.
- Snippets lay out a line-number gutter sized to the widest number, then each source line, then the label lines under it.
  Single-line highlights are drawn with column offsets.
  Multi-line spans draw a gutter rule.
- Related reports render by a fresh handler with a smaller width.

### 8.6 Terminal detection

In `handler.rs`: `supports-color`, `supports-unicode`, `supports-hyperlinks` and `terminal_size`, plus `NO_COLOR` and `NO_GRAPHICS` environment variables [mt-detect].
`GraphicalReportHandler::new()` starts at width 200 [mt-handler].
`MietteHandlerOpts` uses the detected terminal width, or 80 when none is found [mt-opts].

### 8.7 Dependencies

| Configuration | Direct dependencies |
|:--|:--|
| Default (`derive`) | `cfg-if`, `unicode-width` 0.1, `miette-derive` |
| `fancy` | adds `owo-colors`, `textwrap`, `terminal_size`, `supports-color`, `supports-hyperlinks`, `supports-unicode`, `backtrace`, `backtrace-ext` |
| `syntect-highlighter` | adds `syntect` |

### 8.8 Maintenance

Last release 2025-04-27. No stable release in the last 12 months. 115 open issues and PRs.

### 8.9 Lessons

- Copy: theme object with a character set (unicode and ASCII) and a style set (colour and none) chosen once.
- Copy: nested rendering by reducing the width and calling the same code.
- Avoid: colour applied while writing, with no text-first form.
  A caller cannot measure or test a block without a writer.
- Avoid: a large dependency set for width, colour and hyperlink detection spread over four crates.

## 9. cli-table 0.5.0, prettytable-rs 0.10.0, termimad 0.35.5

### 9.1 cli-table

1. Renders a static bordered table from nested `Vec`s or a derive. API [cl-lib]:

   ```rust
   use cli_table::{format::Justify, print_stdout, Cell, Style, Table};

   let table = vec![vec!["Tom".cell(), 10.cell().justify(Justify::Right)]
       .table()
       .title(vec!["Name".cell().bold(true), "Age".cell().bold(true)]);
   print_stdout(table)?;
   ```

2. Styling: `termcolor` `ColorSpec` held on the cell, written to a `termcolor::Buffer` [cl-cell].
3. Width: `unicode-width` 0.2, per char, in a function ported from the `ansi-width` crate [cl-width].
4. ANSI: `display_width` skips CSI (to a byte in 0x40..=0x7C), OSC (to BEL or ST), and for an `ESC` followed by any other character it counts only that character [cl-width].
5. Layout: each cell computes `required_dimension()`. A table takes the maximum per column, then each cell is printed with the `available_dimension` [cl-dimension].
   There is no shrink, no wrap, and no terminal width. Wide tables overflow.
6. Terminal: `termcolor::ColorChoice`, default `Always` [cl-table].
7. Dependencies: `termcolor`, `unicode-width` 0.2. Defaults add `csv`, `cli-table-derive`. No-default is the first two.
8. Maintenance: last release 2025-03-18, 4 open.
9. Lessons: copy the `RequiredDimension` and `AvailableDimension` pair.
   Avoid a default that always colours.

### 9.2 prettytable-rs

1. Renders bordered tables with the `table!` and `row!` macros or `Table::new` [pt-lib].
2. Styling: the `term` crate's `Attr` and colours on cells, written through a `Terminal` [pt-lib].
3. Width: `unicode-width` 0.1. `display_width` is `UnicodeWidthStr::width` minus hidden escape width [pt-width].
4. ANSI: a four-state machine that matches only `ESC [ ... m`.
   It subtracts the width of the escape characters from the string width [pt-width].
5. Layout: column width is the maximum cell width, with `hspan` support.
   No shrink and no wrap [pt-lib].
6. Terminal: `print_tty(force_colorize)` uses `IsTerminal` on stdout [pt-tty].
7. Dependencies: `encode_unicode`, `is-terminal`, `lazy_static`, `term` 0.7, `unicode-width` 0.1. Defaults add `csv`.
8. Maintenance: last release 2022-12-27. 49 open.
9. Lessons: avoid a width fix that handles only SGR.

### 9.3 termimad

1. Renders Markdown to terminal text, with tables, lists, quotes and code blocks.
   The text type is `FmtText { skin, lines, width }` [tm-text].

   ```rust
   let skin = termimad::MadSkin::default();
   let text = skin.text(markdown, Some(80));   // or skin.term_text(markdown)
   println!("{text}");
   ```

   The two calls are in [tm-skin-text].
2. Styling: `MadSkin` holds a `CompoundStyle` (a crossterm `ContentStyle`) per element and a `LineStyle` for paragraphs, headers and tables [tm-skin] [tm-compound].
3. Width: `unicode-width` 0.1, `char.width().unwrap_or(0)` and `str.width()` [tm-tokens].
4. ANSI: none on input. Markdown is parsed by `minimad` first.
5. Layout (tables): a `TblFit` observes every cell width, then fits columns [tm-fit]:
   1. If the sum of maximum widths fits, use them.
   2. A column minimum is 3.
   3. Step 1 cuts columns wider than 4 toward their average width, by at most 4 each, in proportion to how far above average they are.
   4. Step 2 removes the rest in proportion to `width - 3`.
   5. A text line is hard-wrapped to width, with a minimum of 3 [tm-fit].
6. Terminal: `crossterm::terminal::size()` in `terminal_size()` [tm-term].
7. Dependencies, all required: `coolor` (crossterm feature), `crokey`, `crossbeam`, `lazy-regex`, `minimad`, `serde`, `thiserror`, `unicode-width` 0.1. The only feature, `special-renders`, has no dependencies.
8. Maintenance: last release 2026-09-23, 7 stable releases in 12 months, 18 open.
9. Lessons: copy the observe-then-fit shape of `TblFit` (observe cells, then compute).
   The two-step shrink is simple.
   Avoid a hard minimum column of 3 with no way to change it.

## 10. Rich ports on crates.io

I searched the crates.io API for `rich`, `rich-rs`, `rich terminal` and `python rich port`.
Seven crates follow Rich's structure and were read.
Six describe themselves as ports of Python's Rich.
gilt follows the same structure (Console, Segment, Measurement) but its crate description does not call it a port.
The `rich` crate at 0.1.1 is unrelated (a metadata framework).
I did not read `rich-click-rs`, `rich-typography-rs`, `textual`, `standout-render`, `fastapi-output`, `fastmcp-console`, `hud` and the other `rs-rich-*` companions.

| Crate | Created | Last release | Releases | Downloads (total / recent) | Claim |
|:--|:--|:--|:--|:--|:--|
| rich_rust 0.2.3 | 2026-01-19 | 2026-08-19 | 6 | 203.5k / 81.2k | Parity with Rich 13.9.4 [rr-ansi] |
| rs-rich 0.0.15 | 2026-08-08 | 2026-11-05 | 15 | 3.9k / 3.8k | "Faithful" port of Rich 15.0.0, module for module [rsr-lib] |
| rich-rs 1.3.0 | 2026-02-04 | 2026-09-07 | 12 | 1.4k / 0.6k | Port of Rich [rrs-lib] |
| fast-rich 0.4.0 | 2025-12-31 | 2026-11-06 | 7 (6 stable) | 1.5k / 1.2k | Port, with its own types [fr-renderable] |
| richrs 0.2.1 | 2025-12-10 | 2025-12-11 | 3 | 5.9k / 2.5k | Port; no release since 2025-12-11 |
| rusty-rich 0.4.3 | 2026-06-01 | 2026-06-04 | 7 in 4 days | 162 / 10 | Port; the GitHub API returned no data for its repository |
| gilt 2.3.1 | 2026-02-08 | 2026-06-25 | 39 | 2.6k / 0.5k | Rich-style toolkit with extra widgets (charts, forms, HTTP) |

Maturity reading: rich_rust is the only one with real adoption.
rs-rich is the most recent, the most literal, and in daily release (15 releases in about three months, 174 open issues and PRs, one star).
Everything else has under 6,000 downloads in total.
These are all `0.x` or fast-moving; none has a track record yet.

### 10.1 Shared design (what they copy from Python Rich)

- A `Segment { text, style, control }` is the atom of output.
  rich_rust, rs-rich, rich-rs, richrs, rusty-rich and gilt use it [rr-segment] [rsr-segment] [rrs-segment] [rcs-segment] [ru-segment] [gl-segment].
  fast-rich differs: its `Segment` is a list of styled `Span`s plus a newline flag [fr-renderable].
- A `Renderable` produces segments from a console and options.
  A `Measurement { minimum, maximum }` reports the width range.
- `Style` is a struct of `color`, `bgcolor`, attributes, a link and metadata (rich_rust) [rr-style], or a bool per attribute (fast-rich) [fr-style].
- A `Console` owns options, terminal facts and the output stream.
- Tables use Rich's width algorithm: measure columns, collapse the widest wrappable columns, then expand by ratio.

### 10.2 Per crate

**rich_rust 0.2.3**

1. API: `Renderable::render(&self, &Console, &ConsoleOptions) -> Vec<Segment>` [rr-renderable].
   Measuring is a separate trait, `RichMeasure::rich_measure` [rr-measure].
   A `RichCast` trait stands in for Python's `__rich__` [rr-protocol].
2. Width: `cell_len` sums `char.width().unwrap_or(0)`, with an LRU cache (1,024 entries) for strings of 8 or more [rr-cells].
   Per code point, so a ZWJ family counts 2+0+2+0+2.
3. ANSI: `AnsiDecoder` turns SGR and OSC 8 into a styled `Text`. Lenient on bad input [rr-ansi].
4. Table: natural width per column from header, footer and cells, clamped by column min, max or fixed.
   If the total exceeds the space, `collapse_widths`.
   Optionally expand by ratio or by weights [rr-table].
5. Terminal: `crossterm::terminal::size()`, `IsTerminal`, `NO_COLOR`, `COLORTERM`, `TERM` [rr-terminal].
6. Dependencies, required: `bitflags`, `crossterm`, `fancy-regex`, `log`, `lru`, `num-rational`, `once_cell`, `os_pipe`, `regex`, `smallvec`, `stdio-override`, `time`, `unicode-width`.
   Optional: `backtrace`, `pulldown-cmark`, `serde`, `serde_json`, `syntect`, `tracing`, `tracing-subscriber`.
   Default features are empty.
7. Lessons: the measure trait is separate and optional.
   Its table does not use it for cell content.
   The dependency list is long for a core.

#### rs-rich 0.0.15

1. API: one `Renderable` trait with `rich_render` and a `measure` that defaults to `(0, max_width)` [rsr-protocol].
   `Segment { text: CompactString, style, control: bool }` [rsr-segment].
2. Width: its own tables, vendored from Rich's Unicode data for 21 Unicode versions, picked at run time by `UNICODE_VERSION`.
   The source says `unicode-width` was used first and differed on 348 of 21,500 code points checked, so it was dropped [rsr-widths] [rsr-cells].
   `split_graphemes` treats a ZWJ as swallowing the next character [rsr-cells].
3. ANSI: `AnsiDecoder`, a port of `rich.ansi`, with SGR and OSC 8 [rsr-ansi].
4. Table: direct ports of `measure_column`, `collapse_widths` and `ratio_reduce` [rsr-table].
5. Terminal: `terminal_size` and `anstyle-query`.
6. Dependencies, required: `anstyle-query`, `compact_str`, `fancy-regex`, `serde_json`, `terminal_size`.
   Optional: `pulldown-cmark`, `syntect`. Default features: `syntax`, `markdown`.
7. Lessons: the only crate to question `unicode-width` against Rich's tables.
   It keeps extensions out of the port crate by design.
   The plugin and companion crates it needs are many (15 versions in 3 months).

#### rich-rs 1.3.0

1. API: `Renderable::render(&self, &Console, &ConsoleOptions) -> Segments`, plus `measure` with a default that renders and measures the result [rrs-lib].
2. Width: `cell_len` is `UnicodeWidthStr::width` (cluster-aware) and `char_width` is per char [rrs-cells].
3. Table: `measure_column`, `calculate_column_widths`, `ratio_reduce`, `ratio_distribute` and `collapse_widths` as in Rich [rrs-table].
4. Terminal: `atty` and `crossterm`, plus `NO_COLOR`.
5. Dependencies, all required: `atty`, `crossterm`, `once_cell`, `phf`, `pulldown-cmark`, `regex`, `smallvec`, `syntect` (with `regex-onig`), `thiserror`, `unicode-width`. No feature flags.
6. Lessons: a default `measure` that renders is correct but costs a full render per measure.

#### fast-rich 0.4.0

1. API: `Renderable::render(&self, &RenderContext) -> Vec<Segment>`, with `min_width()` and `max_width()` as separate methods that take no context [fr-renderable].
   `Measurement { minimum, maximum, lines }` adds a line count and is computed by rendering [fr-measure].
2. Width: `unicode-width` 0.2 and `unicode-segmentation`; grapheme loops appear in `rule.rs`.
3. Dependencies: `crossterm` 0.28 required; `unicode-segmentation`, `unicode-width`.
   Optional: `log`, `pulldown-cmark`, `regex`, `serde`, `serde_json`, `syntect`. No default features.
4. Lessons: width methods that do not take the console cannot depend on the terminal.
   The `lines` field in `Measurement` is useful for a height pass.

#### richrs 0.2.1

1. API: `Renderable::render(&self, &RenderOptions) -> Result<Segments>` and `measure(&self, &MeasureOptions) -> Result<Measurement>`, with a default that renders [rcs-protocol].
   Errors are in the return type.
2. Width: `unicode-width` 0.2 and `unicode-segmentation` (grapheme counts in `text.rs`) [rcs-text].
3. Dependencies, required: `crossterm`, `once_cell`, `regex`, `serde`, `serde_json`, `thiserror`, `unicode-segmentation`, `unicode-width`.
4. Lessons: a fallible render is an honest signature but hard to compose.
   No release since 2025-12-11.

#### rusty-rich 0.4.3

1. API: `Renderable::render(&self, &ConsoleOptions) -> RenderResult`, and an optional `measure(&self, &ConsoleOptions) -> Option<Measurement>` [ru-renderable].
   A render result may hold nested renderables that the console flattens.
2. Width: `unicode-width` `width()` per string and per char [ru-cells].
3. Dependencies, required: `chrono`, `crossterm`, `log`, `palette`, `regex`, `serde_json`, `terminal_size`, `unicode-width`. Defaults add `pulldown-cmark` and `syntect`.
4. Seven releases in four days suggests a bulk publish, not a cadence.

#### gilt 2.3.1

1. API: `Renderable::gilt_console(&self, &Console, &ConsoleOptions) -> Vec<Segment>`, with a default `measure` that renders.
   The doc fixes the empty-output case to `(0, max_width)` to match Rich [gl-renderable].
2. Width: per-code-point sum on purpose.
   The doc explains that cluster-aware width underflows on terminals that do not join ZWJ emoji, and that over-reserving space is safer than under-reserving [gl-width].
3. Style is a crate-private field behind accessors, with an interner planned [gl-segment].
4. Dependencies, required: `arc-swap`, `compact_str`, `lru`, `regex`, `thiserror`, `unicode-segmentation`, `unicode-width`.
   About 20 optional ones: `crossterm`, `syntect`, `tokio`, `reqwest`, `image`, `rustyline` and others.
   Default features: `json`, `markdown`, `syntax`, `interactive`, `logging`, `terminal-size`.
5. Lessons: the required core is small and everything else is a feature.
   At 99,710 lines it has grown into an application toolkit.

## 11. Does any crate separate measure from render the way Rich does?

Rich's split has three parts.
A renderable reports a `Measurement(min, max)` for a given set of options.
A container (table, columns, panel) asks children for measurements, picks widths, and calls render with those widths.
Render returns segments, and the console writes them.

| Crate | Measure API | Separate from render? | Used by the layout? |
|:--|:--|:--|:--|
| papergrid | `Estimate` and `Dimension` [pg-dimension] | Yes, as a pass over records | Yes, the grid takes a `Dimension` |
| cli-table | `RequiredDimension`, `AvailableDimension` [cl-dimension] | Yes | Yes, but the available size is only the column maximum |
| comfy-table | `column_max_content_widths`, `arrange_content` [ct-max-widths] [ct-arrange] | Stages are separate, but private | Yes |
| textwrap | `Fragment` widths [tw-fragment] | Yes, for wrapping | Yes |
| rich_rust | `RichMeasure` [rr-measure] | Yes, a separate trait | Used by `Constrain`; the table measures its own cells |
| rs-rich, rich-rs, richrs, gilt | `measure` method on `Renderable` | In the same trait, default renders | Yes (ported from Rich) |
| fast-rich | `min_width`, `max_width` | Same trait | Partly |
| rusty-rich | optional `measure` returning `Option` | Same trait | Partly |
| tabled | settings call `estimate` | Settings mutate records between passes | Yes, with rewriting |
| ratatui | None. `Line::width` only [rt-line-width] | No | Constraints only |
| termimad | `TblFit::see_cell` then `fit` [tm-fit] | Yes, for tables only | Yes |
| miette | None | No | No |

The closest to Rich in the Rust ecosystem is the pair `Estimate` plus `Dimension` in papergrid, for tables.
For arbitrary nested content, only the Rich ports have it.

## 12. What a new library should copy and avoid

### 12.1 Copy

1. A measurement type with a minimum and a maximum, and a content-based minimum (longest unbreakable word).
   No Rust table crate has this, and it is what makes shrink decisions good.
2. A measure that takes the same options as render, and can be called without rendering.
   Make the default implementation render, as rich-rs and gilt do, and document that it is slow.
3. A `Segment` list (text, style, control flag) as the output of every renderable.
   Style stays out of the text, so wrapping, cutting and padding never repair escapes.
4. A grid-and-dimension split like papergrid, so a caller can supply or cache widths.
5. A shrink policy as a strategy (tabled's `Peaker`), with Rich's "reduce the widest wrappable column first" as the default.
6. A width module with one function and a stated Unicode version.
   Test it against a ZWJ family, a flag, a variation selector, a halfwidth katakana mark and a tab.
7. ANSI decoding into styled text at the edge (the Rich ports' `AnsiDecoder`), and width-stripping for strings that cannot be decoded (textwrap, cli-table).
8. A `no_std` core with terminal detection outside it (ratatui-core).
9. Terminal detection as data on a console value (width, colour depth, is-tty, hyperlinks), set by the caller, with environment variables read in one place (rich_rust `terminal.rs`).
10. A debug feature that prints each layout step (comfy-table `_debug`).

### 12.2 Avoid

1. Colour or style types that need a terminal crate (comfy-table `tty`, termimad, rich_rust).
2. A width answer that changes with a feature flag (comfy-table `custom_styling`, textwrap `unicode-width`, tabled `ansi`).
3. Rewriting source cells to wrap them (tabled `Wrap`).
4. Writing styled output while laying out, with no value to measure first (miette).
5. A fixed minimum column width or a shrink that ignores content (tabled, termimad).
6. Mixed `unicode-width` major versions across the dependency tree.
   miette, prettytable-rs and termimad pin 0.1 while the others use 0.2.
   I did not read 0.1 source, so the exact difference is unverified.
7. Mixed Unicode versions between segmentation (17.0.0) and width (18.0.0) in one pipeline.
8. Always-on colour as a default (cli-table).
9. Treating every control character the same. ratatui drops it, comfy-table counts it, textwrap and cli-table parse it.
10. A solver for one-dimensional content-driven columns (ratatui's Cassowary is for UI nesting).

## 13. Not verified

- No code was compiled or run, so every behaviour above is from reading.
  In particular the claim that comfy-table without `custom_styling` overcounts ANSI is derived from `UnicodeWidthStr::width` and the `unicode-width` source, not from a run.
- I did not read the `unicode-width` 0.1 source.
- I did not read the ratatui backend crates (`ratatui-crossterm` and others), `ratatui-macros`, `console`, `ansi-str` or `ansitok`.
- For fast-rich, richrs, rusty-rich and gilt, I did not read terminal detection or ANSI input handling.
  Files named `ansi.rs` exist in rich-rs, rusty-rich and gilt and were not opened.
- Open-issue counts come from the GitHub repository API, include pull requests, and are a single snapshot.
  The GitHub API returned no data for the rusty-rich repository.
- Permalinks use docs.rs source at the exact crate version.
  I did not resolve upstream GitHub commits, so none is pinned to a commit.
- Line ranges for functions are approximate to the nearest block where a function ends.
- Dependency lists are from the crates.io dependency API for each version, not from a resolver run.
  The `console` dependencies came from version 0.16.1, which I assumed is what `^0.16` resolves to.

## 14. Comparison table

| Crate | Renders | Style representation | Width model | ANSI in text | Layout | Measure split | Terminal detection | Required deps (minimal) | Last release |
|:--|:--|:--|:--|:--|:--|:--|:--|:--|:--|
| comfy-table 8.0.1 | Table | Own `Color`, `Attribute` on cell, behind `tty` | `unicode-width` 0.2 str width, grapheme cut | Only with `custom_styling` (needs crossterm) | Constraint steps, dynamic fit | Private stages | `tty`: crossterm and `IsTerminal` | 2 | 2026-09-25 |
| tabled 0.22.0 | Table | `String` cells, `Color` prefix and suffix by position | `unicode-width` 0.2 | Only with `ansi` | Peaker-driven shrink, rewrites cells | Settings call `estimate` | None | 1 (papergrid), which needs 2 | 2026-09-05 |
| papergrid 0.18.0 | Grid | `Colors` map to `ANSIFmt` | `unicode-width` 0.2 | Only with `ansi` | Max per column | `Estimate` and `Dimension` | None | 2 | 2026-05-31 |
| ratatui 0.30.2 | Cell buffer, widgets | Own `Style`, spans, lines | `unicode-width` 0.2 plus graphemes, kana fix | None, ESC dropped | Cassowary on constraints, user widths | None | Backend crate | 11 (core) | 2026-06-19 |
| textwrap 0.16.4 | Wrapped text | None | Per char `unicode-width`, ANSI skipped | Skipped when measuring | First fit or optimal fit | `Fragment` | Optional | 0 | 2026-09-13 |
| miette 7.6.0 | Error reports | `owo-colors` styles in a theme | `unicode-width` 0.1 | None | Gutter plus `textwrap` | None | `supports-*`, `terminal_size` | 2 | 2025-04-27 |
| cli-table 0.5.0 | Table | `termcolor` `ColorSpec` | `unicode-width` 0.2 per char | Skipped when measuring | Max per column, no shrink | `RequiredDimension` | `ColorChoice` (default Always) | 2 | 2025-03-18 |
| prettytable-rs 0.10.0 | Table | `term` `Attr` | `unicode-width` 0.1 | SGR subtracted | Max per column, no shrink | None | `IsTerminal` | 5 | 2022-12-27 |
| termimad 0.35.5 | Markdown to text | crossterm `ContentStyle` in a skin | `unicode-width` 0.1 | None | `TblFit` two-step shrink | `see_cell`, then `fit` | crossterm size | 8 | 2026-09-23 |
| rich_rust 0.2.3 | Rich objects | Own `Style` struct, segments | Per-char sum of `unicode-width`, LRU | `AnsiDecoder` to `Text` | Rich table algorithm | `RichMeasure` trait | crossterm, env vars | 13 | 2026-08-19 |
| rs-rich 0.0.15 | Rich objects | Own `Style`, segments | Rich's tables, 21 Unicode versions | `AnsiDecoder` | Direct Rich port | `measure` on trait | `terminal_size`, `anstyle-query` | 5 | 2026-11-05 |
| rich-rs 1.3.0 | Rich objects | Own `Style`, segments | `UnicodeWidthStr::width` | Not read | Rich port | `measure`, default renders | `atty`, crossterm | 10 | 2026-09-07 |
| fast-rich 0.4.0 | Rich objects | Own `Style`, spans in a segment | `unicode-width` 0.2 plus graphemes | Not read | Not read | `min_width`, `max_width` | crossterm | 3 | 2026-11-06 |
| richrs 0.2.1 | Rich objects | Own `Style`, segments | `unicode-width` 0.2 | Not read | Not read | `measure`, default renders | Not read | 8 | 2025-12-11 |
| rusty-rich 0.4.3 | Rich objects | Own `Style`, segments | `unicode-width` | Not read | Not read | optional `measure` | Not read | 8 | 2026-06-04 |
| gilt 2.3.1 | Rich objects and widgets | Own `Style`, segments | Per-code-point sum, deliberate | Not read | Not read | `measure`, default renders | Not read | 7 | 2026-06-25 |

"Required deps" counts direct non-optional dependencies with default features off, from the crates.io dependency data.
For tabled it counts papergrid only.

<!-- Reference-style links -->

[crates-api]: https://crates.io/api/v1/crates/comfy-table/8.0.1/dependencies

[ct-cargo]: https://docs.rs/crate/comfy-table/8.0.1/source/Cargo.toml#L39-49
[ct-cell]: https://docs.rs/crate/comfy-table/8.0.1/source/src/cell.rs#L5-22
[ct-cell-fg]: https://docs.rs/crate/comfy-table/8.0.1/source/src/cell.rs#L92-94
[ct-cell-new]: https://docs.rs/crate/comfy-table/8.0.1/source/src/cell.rs#L28-50
[ct-style-mod]: https://docs.rs/crate/comfy-table/8.0.1/source/src/style/mod.rs#L1-35
[ct-style-line]: https://docs.rs/crate/comfy-table/8.0.1/source/src/utils/formatting/content_format.rs#L330-340
[ct-should-style]: https://docs.rs/crate/comfy-table/8.0.1/source/src/table.rs#L389-396
[ct-tty-width]: https://docs.rs/crate/comfy-table/8.0.1/source/src/table.rs#L255-282
[ct-is-tty]: https://docs.rs/crate/comfy-table/8.0.1/source/src/table.rs#L349-371
[ct-measure-plain]: https://docs.rs/crate/comfy-table/8.0.1/source/src/utils/formatting/content_split/normal.rs#L6-10
[ct-measure-ansi]: https://docs.rs/crate/comfy-table/8.0.1/source/src/utils/formatting/content_split/custom_styling.rs#L9-13
[ct-split-word]: https://docs.rs/crate/comfy-table/8.0.1/source/src/utils/formatting/content_split/normal.rs#L17-52
[ct-split-ansi]: https://docs.rs/crate/comfy-table/8.0.1/source/src/utils/formatting/content_split/custom_styling.rs#L15-70
[ct-fix-style]: https://docs.rs/crate/comfy-table/8.0.1/source/src/utils/formatting/content_split/custom_styling.rs#L129-166
[ct-truncate]: https://docs.rs/crate/comfy-table/8.0.1/source/src/utils/formatting/content_format.rs#L91-121
[ct-max-widths]: https://docs.rs/crate/comfy-table/8.0.1/source/src/table.rs#L631-660
[ct-arrange]: https://docs.rs/crate/comfy-table/8.0.1/source/src/utils/arrangement/mod.rs#L13-55
[ct-constraint]: https://docs.rs/crate/comfy-table/8.0.1/source/src/utils/arrangement/constraint.rs#L8-132
[ct-dynamic-doc]: https://docs.rs/crate/comfy-table/8.0.1/source/src/utils/arrangement/dynamic.rs#L108-127
[ct-dynamic-fit]: https://docs.rs/crate/comfy-table/8.0.1/source/src/utils/arrangement/dynamic.rs#L220-314
[ct-dynamic-split]: https://docs.rs/crate/comfy-table/8.0.1/source/src/utils/arrangement/dynamic.rs#L373-391
[ct-dynamic-distribute]: https://docs.rs/crate/comfy-table/8.0.1/source/src/utils/arrangement/dynamic.rs#L536-548
[ct-lib]: https://docs.rs/crate/comfy-table/8.0.1/source/src/lib.rs#L1-3
[ct-example]: https://docs.rs/crate/comfy-table/8.0.1/source/examples/readme_table_no_tty.rs#L1-21

[pg-lib]: https://docs.rs/crate/papergrid/0.18.0/source/src/lib.rs#L1-60
[pg-dimension]: https://docs.rs/crate/papergrid/0.18.0/source/src/dimension/mod.rs#L9-45
[pg-iter-dim]: https://docs.rs/crate/papergrid/0.18.0/source/src/dimension/iterable.rs#L15-100
[pg-width]: https://docs.rs/crate/papergrid/0.18.0/source/src/util/string.rs#L16-50
[pg-colors]: https://docs.rs/crate/papergrid/0.18.0/source/src/colors/mod.rs#L7-17
[pg-ansifmt]: https://docs.rs/crate/papergrid/0.18.0/source/src/ansi/mod.rs#L15-27
[tb-features]: https://docs.rs/crate/tabled/0.22.0/source/src/lib.rs#L236-246
[tb-peaker]: https://docs.rs/crate/tabled/0.22.0/source/src/settings/peaker/mod.rs#L19-60
[tb-peaker-max]: https://docs.rs/crate/tabled/0.22.0/source/src/settings/peaker/max.rs#L1-50
[tb-wrap]: https://docs.rs/crate/tabled/0.22.0/source/src/settings/width/wrap.rs#L105-136
[tb-decrease]: https://docs.rs/crate/tabled/0.22.0/source/src/settings/width/wrap.rs#L778-816
[tb-wrap-text]: https://docs.rs/crate/tabled/0.22.0/source/src/settings/width/wrap.rs#L211-238
[tb-wrap-ansi]: https://docs.rs/crate/tabled/0.22.0/source/src/settings/width/wrap.rs#L284-300
[tb-width-mod]: https://docs.rs/crate/tabled/0.22.0/source/src/settings/width/mod.rs#L106-120
[tb-percent]: https://docs.rs/crate/tabled/0.22.0/source/src/settings/measurement/mod.rs#L90-100
[tb-color]: https://docs.rs/crate/tabled/0.22.0/source/src/settings/color/mod.rs#L38-47

[rt-buffer]: https://docs.rs/crate/ratatui-core/0.1.2/source/src/buffer/buffer.rs#L67-73
[rt-cell]: https://docs.rs/crate/ratatui-core/0.1.2/source/src/buffer/cell.rs#L37-60
[rt-set-stringn]: https://docs.rs/crate/ratatui-core/0.1.2/source/src/buffer/buffer.rs#L336-370
[rt-cellwidth]: https://docs.rs/crate/ratatui-core/0.1.2/source/src/buffer/cell_width.rs#L1-46
[rt-diff]: https://docs.rs/crate/ratatui-core/0.1.2/source/src/buffer/buffer.rs#L471-507
[rt-ansi-test]: https://docs.rs/crate/ratatui-core/0.1.2/source/src/buffer/buffer.rs#L1399-1413
[rt-span]: https://docs.rs/crate/ratatui-core/0.1.2/source/src/text/span.rs#L99-104
[rt-line]: https://docs.rs/crate/ratatui-core/0.1.2/source/src/text/line.rs#L184-193
[rt-line-width]: https://docs.rs/crate/ratatui-core/0.1.2/source/src/text/line.rs#L441
[rt-text]: https://docs.rs/crate/ratatui-core/0.1.2/source/src/text/text.rs#L197-204
[rt-style]: https://docs.rs/crate/ratatui-core/0.1.2/source/src/style.rs#L239-260
[rt-anstyle]: https://docs.rs/crate/ratatui-core/0.1.2/source/src/style/anstyle.rs#L1-10
[rt-widget]: https://docs.rs/crate/ratatui-core/0.1.2/source/src/widgets/widget.rs#L70-76
[rt-nostd]: https://docs.rs/crate/ratatui-core/0.1.2/source/src/lib.rs#L1
[rt-constraint]: https://docs.rs/crate/ratatui-core/0.1.2/source/src/layout/constraint.rs#L8-30
[rt-layout-solve]: https://docs.rs/crate/ratatui-core/0.1.2/source/src/layout/layout.rs#L793-900
[rt-strengths]: https://docs.rs/crate/ratatui-core/0.1.2/source/src/layout/layout.rs#L1286-1345
[rt-table-new]: https://docs.rs/crate/ratatui-widgets/0.3.2/source/src/table.rs#L80-100
[rt-table-widths]: https://docs.rs/crate/ratatui-widgets/0.3.2/source/src/table.rs#L1036-1062
[rt-reflow]: https://docs.rs/crate/ratatui-widgets/0.3.2/source/src/reflow.rs#L15-35
[ks-lib]: https://docs.rs/crate/kasuari/0.4.13/source/src/lib.rs#L1-12

[tw-options]: https://docs.rs/crate/textwrap/0.16.4/source/src/options.rs#L8-35
[tw-skip]: https://docs.rs/crate/textwrap/0.16.4/source/src/core.rs#L39-85
[tw-width]: https://docs.rs/crate/textwrap/0.16.4/source/src/core.rs#L104-207
[tw-ch-width]: https://docs.rs/crate/textwrap/0.16.4/source/src/core.rs#L89-102
[tw-fragment]: https://docs.rs/crate/textwrap/0.16.4/source/src/core.rs#L209-230
[tw-algos]: https://docs.rs/crate/textwrap/0.16.4/source/src/wrap_algorithms.rs#L37-60
[tw-sep]: https://docs.rs/crate/textwrap/0.16.4/source/src/word_separators.rs#L64-70
[tw-termwidth]: https://docs.rs/crate/textwrap/0.16.4/source/src/termwidth.rs#L24-29

[uw-rules]: https://docs.rs/crate/unicode-width/0.2.3/source/src/lib.rs#L51-98
[uw-char]: https://docs.rs/crate/unicode-width/0.2.3/source/src/lib.rs#L209-235
[uw-lookup]: https://docs.rs/crate/unicode-width/0.2.3/source/src/lookup.rs#L109-114
[uw-version]: https://docs.rs/crate/unicode-width/0.2.3/source/src/gen/tables.rs#L15
[us-graphemes]: https://docs.rs/crate/unicode-segmentation/1.13.3/source/src/grapheme.rs#L80-90
[us-version]: https://docs.rs/crate/unicode-segmentation/1.13.3/source/src/tables.rs#L17
[ut-trait]: https://docs.rs/crate/unicode-truncate/3.0.0/source/src/lib.rs#L59-100

[mt-handler]: https://docs.rs/crate/miette/7.6.0/source/src/handlers/graphical.rs#L26-62
[mt-render]: https://docs.rs/crate/miette/7.6.0/source/src/handlers/graphical.rs#L242-249
[mt-wrap]: https://docs.rs/crate/miette/7.6.0/source/src/handlers/graphical.rs#L262-270
[mt-width]: https://docs.rs/crate/miette/7.6.0/source/src/handlers/graphical.rs#L1060-1070
[mt-theme]: https://docs.rs/crate/miette/7.6.0/source/src/handlers/theme.rs#L14-30
[mt-detect]: https://docs.rs/crate/miette/7.6.0/source/src/handler.rs#L275-300

[cl-lib]: https://docs.rs/crate/cli-table/0.5.0/source/src/lib.rs#L1-50
[cl-cell]: https://docs.rs/crate/cli-table/0.5.0/source/src/cell.rs#L45-56
[cl-width]: https://docs.rs/crate/cli-table/0.5.0/source/src/utils.rs#L10-61
[cl-dimension]: https://docs.rs/crate/cli-table/0.5.0/source/src/dimension.rs#L55-74
[cl-table]: https://docs.rs/crate/cli-table/0.5.0/source/src/table.rs#L20-50
[pt-lib]: https://docs.rs/crate/prettytable-rs/0.10.0/source/src/lib.rs#L100-130
[pt-width]: https://docs.rs/crate/prettytable-rs/0.10.0/source/src/utils.rs#L84-125
[pt-tty]: https://docs.rs/crate/prettytable-rs/0.10.0/source/src/lib.rs#L186-202
[tm-text]: https://docs.rs/crate/termimad/0.35.5/source/src/text.rs#L30-34
[tm-skin]: https://docs.rs/crate/termimad/0.35.5/source/src/skin.rs#L39-56
[tm-skin-text]: https://docs.rs/crate/termimad/0.35.5/source/src/skin.rs#L366-380
[tm-compound]: https://docs.rs/crate/termimad/0.35.5/source/src/compound_style.rs#L43-45
[tm-tokens]: https://docs.rs/crate/termimad/0.35.5/source/src/tokens.rs#L30-40
[tm-fit]: https://docs.rs/crate/termimad/0.35.5/source/src/fit/tbl_fit.rs#L6-176
[tm-term]: https://docs.rs/crate/termimad/0.35.5/source/src/area.rs#L159

[rr-protocol]: https://docs.rs/crate/rich_rust/0.2.3/source/src/protocol.rs#L1-30
[rr-renderable]: https://docs.rs/crate/rich_rust/0.2.3/source/src/renderables/mod.rs#L95-99
[rr-measure]: https://docs.rs/crate/rich_rust/0.2.3/source/src/measure.rs#L185-190
[rr-segment]: https://docs.rs/crate/rich_rust/0.2.3/source/src/segment.rs#L110-122
[rr-style]: https://docs.rs/crate/rich_rust/0.2.3/source/src/style.rs#L150-169
[rr-cells]: https://docs.rs/crate/rich_rust/0.2.3/source/src/cells.rs#L11-50
[rr-ansi]: https://docs.rs/crate/rich_rust/0.2.3/source/src/ansi.rs#L1-12
[rr-table]: https://docs.rs/crate/rich_rust/0.2.3/source/src/renderables/table.rs#L741-826
[rr-terminal]: https://docs.rs/crate/rich_rust/0.2.3/source/src/terminal.rs#L16-60
[rsr-lib]: https://docs.rs/crate/rs-rich/0.0.15/source/src/lib.rs#L1-10
[rsr-protocol]: https://docs.rs/crate/rs-rich/0.0.15/source/src/protocol.rs#L15-40
[rsr-segment]: https://docs.rs/crate/rs-rich/0.0.15/source/src/segment.rs#L15-22
[rsr-cells]: https://docs.rs/crate/rs-rich/0.0.15/source/src/cells.rs#L1-12
[rsr-widths]: https://docs.rs/crate/rs-rich/0.0.15/source/src/cell_widths.rs#L1-14
[rsr-ansi]: https://docs.rs/crate/rs-rich/0.0.15/source/src/ansi.rs#L1-12
[rsr-table]: https://docs.rs/crate/rs-rich/0.0.15/source/src/table.rs#L1856-1888
[rrs-lib]: https://docs.rs/crate/rich-rs/1.3.0/source/src/lib.rs#L178-191
[rrs-segment]: https://docs.rs/crate/rich-rs/1.3.0/source/src/segment.rs#L58-70
[rrs-cells]: https://docs.rs/crate/rich-rs/1.3.0/source/src/cells.rs#L18-26
[rrs-table]: https://docs.rs/crate/rich-rs/1.3.0/source/src/table.rs#L1652-1660
[fr-renderable]: https://docs.rs/crate/fast-rich/0.4.0/source/src/renderable.rs#L5-80
[fr-measure]: https://docs.rs/crate/fast-rich/0.4.0/source/src/measure.rs#L8-17
[fr-style]: https://docs.rs/crate/fast-rich/0.4.0/source/src/style.rs#L504-516
[rcs-protocol]: https://docs.rs/crate/richrs/0.2.1/source/src/protocol.rs#L48-58
[rcs-segment]: https://docs.rs/crate/richrs/0.2.1/source/src/segment.rs#L128-135
[rcs-text]: https://docs.rs/crate/richrs/0.2.1/source/src/text.rs#L147
[ru-renderable]: https://docs.rs/crate/rusty-rich/0.4.3/source/src/console.rs#L310-322
[ru-segment]: https://docs.rs/crate/rusty-rich/0.4.3/source/src/segment.rs#L141-145
[ru-cells]: https://docs.rs/crate/rusty-rich/0.4.3/source/src/cells.rs#L11-13
[gl-renderable]: https://docs.rs/crate/gilt/2.3.1/source/src/console.rs#L314-330
[gl-segment]: https://docs.rs/crate/gilt/2.3.1/source/src/segment.rs#L221-230
[gl-width]: https://docs.rs/crate/gilt/2.3.1/source/src/utils/cells.rs#L30-83

[mt-opts]: https://docs.rs/crate/miette/7.6.0/source/src/handler.rs#L362-370
[mt-with-width]: https://docs.rs/crate/miette/7.6.0/source/src/handlers/graphical.rs#L158-162
