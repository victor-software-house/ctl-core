# Rich and Typer: a source survey for a Rust terminal rendering library

This survey is read from source code only.
It supports the design of a Rust terminal rendering library.
Nothing here is an implementation.

Every claim cites a permalink pinned to a commit.
Links are reference-style and defined at the end of the file.
The code was read, never run.
Where a statement comes from reading and was not reproduced, the text says so.

## 0. Versions read

| Project | Release tag | Commit | Tag date |
|:--|:--|:--|:--|
| [Rich][rich-repo] | `v15.0.0` | `6ac483cbea39cab124dfd3483bba70ffafb71050` | 2026-04-12 |
| [Typer][typer-repo] | `0.28.0` | `bf5cfc1b5ea5092254d1c3924eeccf13d48605eb` | 2026-11-06 |

Each tag was the highest `X.Y.Z` tag on the remote when read.
Issue and pull request states below were read from GitHub on 2026-11-07 and may change.

## 1. The renderable protocol

### 1.1 The three hooks

A renderable is any object with one of three methods.
Strings are renderable too.
The check is by attribute presence ([protocol.py:9-15][protocol-check]).

| Hook | Purpose | Called by |
|:--|:--|:--|
| `__rich__()` | Cast the object to another renderable. Recursive, with a visited-type guard. | `rich_cast` ([protocol.py:18-41][protocol-cast]) |
| `__rich_console__(console, options)` | Yield `Segment` objects or further renderables. | `Console.render` ([console.py:1294-1343][console-render]) |
| `__rich_measure__(console, options)` | Return the min and max width the object wants. Optional. | `Measurement.get` ([measure.py:78-122][measurement-get]) |

The Python protocol types are in [console.py:246-271][console-protocols].
The result of `__rich_console__` is `Iterable[Renderable | Segment]`.
`Console.render` flattens it by recursion: a non-`Segment` item is rendered again with the same options, and height is reset to `None` for children ([console.py:1331-1343][console-render-1331-1343]).

```python
from rich.console import Console, ConsoleOptions, RenderResult
from rich.measure import Measurement
from rich.segment import Segment
from rich.style import Style

class Badge:
    def __rich_measure__(self, console: Console, options: ConsoleOptions) -> Measurement:
        return Measurement(5, 5)

    def __rich_console__(self, console: Console, options: ConsoleOptions) -> RenderResult:
        yield Segment("[ok]", Style(bold=True, color="green"))
```

Design points:

1. Render is lazy and recursive. A parent can yield a child renderable and let the console expand it.
2. A cast hook (`__rich__`) is separate from the render hook. This lets domain types stay free of layout code.
3. Strings are promoted to `Text` inside `render` and `measure`, using the options' `markup` flag ([console.py:1317-1324][console-render-1317-1324], [measure.py:98-101][measurement-get-98-101]).

### 1.2 Measurement

`Measurement` is a `NamedTuple(minimum, maximum)` in cells ([measure.py:11-57][measurement]).
It has `normalize`, `with_maximum`, `with_minimum`, and `clamp`.

`Measurement.get` does four things ([measure.py:78-122][measurement-get]):

1. Return `(0, 0)` if `options.max_width < 1`.
2. Promote `str` to `Text`, then run `rich_cast`.
3. If the object has `__rich_measure__`, call it, normalize, clamp the maximum to `options.max_width`, and return `(0, 0)` if the maximum is below 1.
4. If it has no `__rich_measure__`, return `Measurement(0, options.max_width)`. Such an object is "greedy": it takes whatever it is offered ([measure.py:116-117][measure-default]).

`measure_renderables` takes the max of all minimums and the max of all maximums ([measure.py:125-151][measure-renderables]).
That is the rule for stacking children vertically.

Text measures itself as: maximum is the longest line in cells, minimum is the longest whitespace-separated word ([text.py:707-717][text-measure]).

### 1.3 ConsoleOptions

`ConsoleOptions` is a dataclass passed down the render tree ([console.py:112-140][console-options]).
It is copied and edited at each level with `update`, `update_width`, `update_height`, and `update_dimensions` ([console.py:157-243][console-options-update-157-243]).

| Field | Meaning |
|:--|:--|
| `size` | `ConsoleDimensions(width, height)` of the console |
| `legacy_windows` | Old Windows console flag, used for box substitution |
| `min_width`, `max_width` | The width window offered to this renderable |
| `is_terminal` | True if the sink understands escape sequences |
| `encoding` | Sink encoding; `ascii_only` is derived from it |
| `max_height`, `height` | Container height, and an optional exact height |
| `justify`, `overflow`, `no_wrap` | Text layout overrides |
| `highlight`, `markup` | Overrides for string promotion |

`update(width=...)` sets both `min_width` and `max_width`.
So most parents render children at an exact width.

### 1.4 Segment

`Segment` is a `NamedTuple(text, style, control)` ([segment.py:61-104][segment-class]).
It is the only output unit of the render pipeline.
A control segment carries cursor or screen codes (`ControlType`) and has cell length zero ([segment.py:32-50][segment-control-type], [control.py:28-40][control-formats]).

Utilities on `Segment`:

| Utility | What it does | Source |
|:--|:--|:--|
| `split_cells(cut)` | Split at a cell column. A cut inside a wide character replaces it with two spaces, one on each side. | [segment.py:106-179][segment-split-cells] |
| `split_and_crop_lines(segments, length, style, pad)` | Split a segment stream on `\n`, crop each line to `length` cells, optionally pad with spaces. | [segment.py:306-351][segment-split-crop] |
| `adjust_line_length(line, length, style, pad)` | Pad or crop one line of segments to an exact width. | [segment.py:353-396][segment-adjust] |
| `set_shape(lines, width, height, style)` | Make a list of lines a width-by-height rectangle. | [segment.py:425-459][segment-set-shape] |
| `divide(segments, cuts)` | Split a segment stream at cell positions into several lists. | [segment.py:629-696][segment-divide] |
| `simplify` | Merge adjacent segments with equal style. | [segment.py:550-575][segment-simplify] |
| `remove_color` | Replace each style by its colorless twin, with a cache. | [segment.py:607-627][segment-remove-color] |

Observation: no module in `rich/` calls `Segment.divide`.
It is a public utility with no internal caller at this commit (searched with `rg` over the package).

A "line" is a `list[Segment]` with no `\n`.
`Console.render_lines` renders a renderable at the given options, applies an optional style, splits and crops to `max_width`, and pads to `height` ([console.py:1345-1407][console-render-lines]).
Containers use `render_lines` to turn a child into a rectangle of cells.

`Lines` in `containers.py` is a different thing: a list of `Text` lines with `justify` for left, center, right, and full ([containers.py:66-167][containers-lines]).
`Renderables` is a plain list renderable whose measure is the max of its children ([containers.py:30-63][containers-renderables]).

### 1.5 How measure feeds render

Measure runs first, with the same `ConsoleOptions` type.
The parent decides a width from child measurements, then renders the child with `options.update_width(w)`.
Examples:

- `Padding`, when not expanding, measures the child, adds left and right padding, caps at `max_width`, then renders the child at the reduced width ([padding.py:79-135][padding-render]).
- `Panel` with `expand=False` calls `console.measure` on the padded child to get `child_width` ([panel.py:210-216][panel-render-210-216]).
- `Align` measures the child, renders it inside a `Constrain` at that width, finds the real shape with `Segment.get_shape`, then adds side padding by the leftover space ([align.py:143-233][align-render-143-233]). `Align` forwards `__rich_measure__` to its child ([align.py:235-239][align-render-235-239]).
- `Constrain` clamps the offered width, and clamps the measure the same way ([constrain.py:22-37][constrain]).
- `Table` measures every cell in a column to pick widths, then renders each cell with `render_lines` at the chosen width (section 6.1).

### 1.6 Containers at a glance

| Container | Measure | Render of children |
|:--|:--|:--|
| `Group` | `measure_renderables` if `fit`, else full width ([console.py:450-480][console-group]) | Yields its renderables in order |
| `Padding` | child measure plus side padding ([padding.py:125-135][padding-render-125-135]) | `render_lines` at reduced width, then side and top and bottom blanks |
| `Align` | child measure | See 1.5 |
| `Panel` | `(w, w)` where `w` is padded child max plus 2, or the fixed width ([panel.py:277-297][panel-measure]) | `render_lines` of the padded child, then border segments around each line ([panel.py:141-275][panel-render]) |
| `Columns` | none, so greedy ([measure.py:116-117][measure-default]) | Builds a borderless `Table.grid`, trying fewer columns until measured widths fit ([columns.py:62-171][columns-render]) |
| `Layout` | none | Splits a region by ratio, then renders each leaf at its exact region size ([layout.py:345-415][layout-regions]) |
| `Tree` | max over nodes of label measure plus 4 per depth level ([tree.py:176-201][tree-measure]) | Iterative stack walk, one `render_lines` per node, guide segments as prefix ([tree.py:86-174][tree-render]) |

`Panel` returns equal min and max.
So a `Panel` inside a flexible `Table` cell cannot shrink below its content width through measurement alone.

`Layout` uses `ratio_resolve` on edges that have `size`, `ratio`, and `minimum_size` ([_ratio.py:14-72][ratio-resolve], [layout.py:80-138][layout-splitters]).
It does not measure content.

## 2. Style model

### 2.1 Style

`Style` is an immutable-by-convention value with `__slots__` ([style.py:40-205][style-class]).

- 13 attributes (bold, dim, italic, underline, blink, blink2, reverse, conceal, strike, underline2, frame, encircle, overline) are stored in **two bitsets**: `_attributes` for the value and `_set_attributes` for "was this set at all". Each attribute is therefore tri-state: on, off, or unset ([style.py:25-36][style-bits], [style.py:160-197][style-class-160-197]).
- Fields: `color`, `bgcolor` (as `Color`), `link` (URL), `meta` (a pickled dict), and a hash.
- A style is false when nothing is set. A shared `NULL_STYLE` singleton is the null style ([style.py:207-210][style-null]).

Combining is an overlay, `base + top`.
Where `top` sets an attribute, it wins.
Colors and link take `top` if present, else `base` ([style.py:732-762][style-add]).
`Style.combine` and `Style.chain` fold a list with `+` ([style.py:600-624][style-combine]).
`_add` is wrapped in `lru_cache` ([style.py:732][style-add-732-732]).

```python
from rich.style import Style

base = Style(bold=True, color="red")
top = Style(bold=False, bgcolor="white")
assert (base + top).bold is False        # explicit "off" overrides
assert (base + top).color.name == "red"  # unset color keeps the base
Style.parse("bold red on white")         # text definition
Style.parse("not bold")                  # tri-state off
```

`Style.parse` reads a space-separated definition: attribute names and short aliases, `not <attr>`, `on <color>`, `link <url>`, else a color ([style.py:496-561][style-parse]).
`str(style)` re-generates the definition ([style.py:294-338][style-str]).

Equality compares hashes, not fields ([style.py:431-445][style-eq]).
That treats a hash collision as equal.
This is from reading; no collision was constructed.

### 2.2 ANSI emission

`Style.render(text, color_system, legacy_windows)` writes `ESC[<sgr>m text ESC[0m` for each segment.
It does not diff against the previous style.
Links are wrapped in OSC 8 as `ESC ] 8 ; id=<id> ; <url> ESC \ ... ESC ] 8 ; ; ESC \` ([style.py:694-718][style-render]).
The URL is inserted as given.

The SGR parameter string is built once and cached in `_ansi` ([style.py:344-385][style-ansi]).
Observation from reading: the cache has no key for the color system.
`render` takes `self._ansi or self._make_ansi_codes(color_system)` ([style.py:712][style-render-712-712]) and `copy` copies `_ansi` ([style.py:626-646][style-copy]).
A style shared between two consoles with different color systems would reuse the first result.
Theme styles are shared objects ([default_styles.py:5-30][default-styles]).
This was not reproduced.

### 2.3 Color

`Color` is `NamedTuple(name, type, number, triplet)` ([color.py:303-313][color-class]).
`ColorType` has default, standard (0-15), 8-bit (16-255), truecolor, and windows ([color.py:21-47][color-systems]).
`ColorSystem` is the capability of the terminal: standard, 256, truecolor, windows.

Parsing accepts a name, `default`, `#rrggbb`, `color(N)`, or `rgb(r,g,b)` ([color.py:292-299][color-regex], [color.py:433-482][color-parse]).
Names map to 0-255 through the `ANSI_COLOR_NAMES` table that starts at [color.py:49-285][color-names].

SGR codes: default is `39`/`49`; standard is `30+n` or `90+n-8`; 8-bit is `38;5;n`; truecolor is `38;2;r;g;b` ([color.py:485-510][color-ansi-codes]).

Downgrade runs at emission time, per color, and is `lru_cache`d on `(color, system)` ([color.py:513-568][color-downgrade]):

- Truecolor to 8-bit: convert to HLS. If saturation is below 0.15, use the 24-step gray ramp (`231 + round(l * 25)`, with 16 and 231 at the ends). Otherwise snap each channel to the 6-level cube with the non-uniform steps `0, 95, 135, 175, 215, 255`.
- To standard: nearest match in a fixed palette (`STANDARD_PALETTE.match`).
- To Windows: nearest in the Windows palette.

`ColorTriplet(red, green, blue)` has `hex`, `rgb`, and `normalized` ([color_triplet.py:4-38][color-triplet]).

### 2.4 Theme and style names

A `Theme` is a dict of name to `Style`, with optional inheritance from `DEFAULT_STYLES` ([theme.py:7-27][theme-class], [default_styles.py:5-30][default-styles]).
A `ThemeStack` holds one merged dict per pushed theme ([theme.py:81-106][theme-stack]).
Themes can be loaded from an INI file with a `[styles]` section ([theme.py:37-57][theme-class-37-57]).

`Console.get_style(name)` first looks the name up in the theme stack.
Then it falls back to `Style.parse(name)`.
A parse failure raises `MissingStyle` unless a `default` is given ([console.py:1470-1498][console-get-style]).
So one string namespace serves both semantic names (`"repr.number"`, `"table.header"`) and inline definitions (`"bold red"`).

Text spans store the style as a name or a `Style` and resolve it at render time with a default of the null style ([text.py:736-740][text-render-736-740]).
So an unknown span name is dropped silently.
This matters for markup (section 2.6).

### 2.5 Text and spans

`Text` is a string plus `Span(start, end, style)` records ([text.py:47-115][text-span], [text.py:144-165][text-init]).
Offsets are Python string indices, which are code points.
Spans may overlap and nest.

The constructor strips five control characters (BEL, BS, VT, FF, CR) and nothing else ([text.py:156][text-init-156-156], [control.py:9-18][control-strip], [control.py:181-192][control-strip-fn]).
ESC (0x1B) is not stripped.
Section 4.3 covers the consequence.

`Text.render` turns spans into segments with a sweep line ([text.py:719-776][text-render]):

1. Build events `(offset, leaving, span_id)` for each span start and end, plus the whole-text base style at id 0.
2. Sort by offset.
3. Walk the events with a stack of active span ids. For each gap between events, yield `Segment(text[a:b], Style.combine(styles of the stack in id order))`, with a cache keyed by the tuple of styles.

Later spans win over earlier ones because the stack is combined in span order.

```python
from rich.text import Text

t = Text("hello world")
t.stylize("bold", 0, 5)
t.stylize("red", 3, 11)   # overlaps; "lo" is bold and red
```

`stylize` ignores empty or out-of-range spans ([text.py:457-481][text-stylize]).
`Text.divide(offsets)` splits a text and its spans at offsets using a binary search over line ranges ([text.py:1106-1183][text-divide]).

### 2.6 Markup

The markup parser is a small regex plus a stack ([markup.py:12-15][markup-regex], [markup.py:73-103][markup-parse], [markup.py:106-231][markup-render]).

- Tag regex: `((\\*)\[([a-z#/@][^[]*?)])`. A tag starts with `[` and a lowercase letter, `#`, `/`, or `@`.
- `[bold red]...[/]` opens and closes. `[/bold]` closes the nearest matching open tag, even if not the top. `[/]` closes the top.
- A closing tag with no match raises `MarkupError` ([markup.py:159-176][markup-render-159-176]).
- Backslashes before a tag escape it: an odd count escapes the tag, and pairs are literal backslashes ([markup.py:83-98][markup-parse-83-98]).
- `markup.escape` adds the backslash ([markup.py:48-70][markup-escape]).
- `[link=URL]` is an ordinary parameterized tag. Its text becomes a style `link URL`. `[@name=args]` stores `meta` and parses the args with `ast.literal_eval` ([markup.py:178-215][markup-render-178-215]).
- Emoji codes `:name:` are replaced by a dictionary lookup unless markup rendering turns emoji off ([_emoji_replace.py:9-31][emoji-replace]).

```python
from rich.text import Text

Text.from_markup("[bold red]stop[/] and [link=https://example.com]go[/link]")
```

From reading ([markup.py:12-15][markup-regex], [text.py:736-740][text-render-736-740]):
text such as `list[int]` is read as a tag named `int`.
The bracketed text is removed from the output, and the unknown style is dropped at render time without an error.
Issue [#3203][issue-3203] (open) reports that `[link=PATH]` fails when the path contains brackets.

### 2.7 Highlighters

A `Highlighter` takes a `str` or `Text` and returns a `Text` with extra spans ([highlighter.py:17-48][highlighter-regex-17-48]).
`RegexHighlighter` runs a list of regexes through `Text.highlight_regex` ([highlighter.py:61-77][highlighter-regex], [text.py:593-631][text-highlight-regex]).
Each named regex group becomes a span with the style name `base_style + group_name`, such as `repr.number`.
The theme gives those names their look.

`ReprHighlighter` is the default highlighter of a `Console` ([console.py:642-646][console-init-highlight]).
Automatic highlighting is on by default (`highlight=True`).
Its patterns cover tags, `name=value`, braces, IPv4, IPv6, EUI-48, EUI-64, UUID, calls, `True`/`False`/`None`, ellipsis, complex numbers, numbers, paths, strings, and URLs ([highlighter.py:80-103][highlighter-repr]).

`JSONHighlighter` uses its own regex plus a second pass to mark keys: a string followed by `:` ([highlighter.py:106-130][highlighter-json]).

Known false positives (section 9.2) come from this pattern list.

### 2.8 Emoji

Emoji are a `:code:` dictionary of 3,610 lines in `_emoji_codes.py` and a `Emoji` renderable that yields one segment ([_emoji_replace.py:9-31][emoji-replace]).
A variant suffix `-emoji` or `-text` appends U+FE0F or U+FE0E.
The replacement runs by default on printed strings (`emoji=True`, [console.py:640-641][console-init-emoji]).

## 3. Width model

### 3.1 cell_len and the width table

`cell_len(text)` returns terminal cells ([cells.py:98-158][cells-cell-len]):

1. Strings under 512 chars go through an `lru_cache` ([cells.py:108-110][cells-cell-len-108-110]).
2. A fast path checks whether every char is in a fixed set of single-cell ranges (Latin, Greek and Cyrillic, box drawing, braille) ([cells.py:13-35][cells-single]).
3. Without U+200D or U+FE0F in the text, sum `get_character_cell_size` per code point.
4. With them, walk the text. A ZWJ (U+200D) skips the next code point. A VS16 (U+FE0F) adds one cell if the previous measured char is in the `narrow_to_wide` set.

`get_character_cell_size` returns 0 for code points under 32 and for 0x7F to 0x9F, then does a binary search in a table of `(start, end, width)` triples; anything past the last entry is width 1 ([cells.py:46-78][cells-char-size]).

In v15 the table is no longer one file.
There is one generated module per Unicode version (21 modules, 4.1.0 to 17.0.0), plus a loader and a version list. The 23 files total 11,928 lines.
The data comes from the `wcwidth` project ([unicode17-0-0.py:1-11][unicode-table-head]).
`load("auto")` reads the `UNICODE_VERSION` environment variable and defaults to the latest ([_unicode_data/__init__.py:58-93][unicode-load]).
Zero-width entries sit in the same table as `(start, end, 0)`, for example `(768, 879, 0)` for combining marks.

### 3.2 Graphemes

Pull request [#3930][pr-3930] ("Handle graphemes", closed) says Rich "has never handled multi-codepoint emoji" and moves to the wcwidth data.
`split_graphemes` returns `(start, end, width)` spans that cover the whole string ([cells.py:161-232][cells-graphemes]):

- A zero-width code point attaches to the previous span.
- A ZWJ extends the previous span over the next code point.
- A VS16 widens the previous span if it is narrow-to-wide.

It does not use Unicode grapheme break properties.
Only U+200D, U+FE0F, and zero width are special cases.
The code has no table for regional indicators, Hangul jamo, or Indic conjuncts beyond what the width table says.
This is from reading; behavior on those inputs was not run.

Wide characters:

- `_split_text` finds a cell column inside the span list. If the cut falls inside a 2-cell span, it returns `left + " "` and `" " + right` ([cells.py:235-276][cells-split-text]).
- `set_cell_size` pads with spaces or crops using that split ([cells.py:299-323][cells-set-size]).
- `chop_cells` folds a string into rows no wider than `width`, walking grapheme spans ([cells.py:326-352][cells-chop]).
- `Segment.split_cells` has its own older approximation loop for segments ([segment.py:106-153][segment-split-cells-106-153]). Several open or closed pull requests rework it: [#3974][issue-3974], [#4146][issue-4146], [#4155][issue-4155].

Other bugs in this area: [#3897][issue-3897] (VS16 width, closed), [#3947][issue-3947] (ZWJ error, closed).

### 3.3 Wrapping

`Text.wrap(console, width, justify, overflow, tab_size, no_wrap)` ([text.py:1201-1250][text-wrap]):

1. Split the text on `\n` into lines.
2. Expand tabs.
3. If `no_wrap`, keep each line whole. Else call `divide_line(line, width, fold)` to get break offsets, then `Text.divide(offsets)`, then trim trailing whitespace that overhangs the width.
4. Apply justify to the new lines.
5. Truncate each line to `width` by the overflow mode.

`divide_line` works on words matched by `\s*\S+\s*` ([_wrap.py:9-23][wrap-words], [_wrap.py:26-76][wrap-divide-line]):

- Measure each word without trailing space.
- If it fits the rest of the line, advance the cell offset.
- If it is wider than a whole line and `fold` is true, chop it into width-sized pieces with `chop_cells`. If `fold` is false, break before it and leave it to be cropped.
- Otherwise start a new line before it.

The returned break positions are string indices.
The docstring calls them cell offsets, but the code appends `start` from the regex match, which is an index into the string.

Overflow modes ([text.py:859-884][text-truncate]):

| Mode | Effect |
|:--|:--|
| `fold` (default) | Wrap long words onto the next row |
| `crop` | Cut at the width |
| `ellipsis` | Cut at width minus one and add `…` |
| `ignore` | No wrapping or cropping at all |

`Column` in `Table` defaults to `ellipsis` ([table.py:90][table-column-90-90]).
Justify values: left, center, right, full ([containers.py:111-167][containers-lines-111-167]).
Full justify distributes spaces from the last gap backwards and skips the last line.

`Console.print` has `soft_wrap`, which sets `no_wrap=True` and `overflow="ignore"` and turns off cropping ([console.py:1695-1702][console-soft-wrap]).

## 4. ANSI handling

### 4.1 Where ANSI is generated

Only two places write escape sequences:

1. `Style.render`, one pair of `ESC[..m` and `ESC[0m` per segment, plus OSC 8 for links ([style.py:694-718][style-render]).
2. `Control` segments, mapped by `CONTROL_CODES_FORMAT` to cursor, clear, alt-screen, title, and similar codes ([control.py:28-40][control-formats]).

The console writes in `_render_buffer`.
It loops over buffered segments and calls `style.render(text, color_system, legacy_windows)`.
Control segments are dropped when the sink is not a terminal ([console.py:2132-2154][console-render-buffer]).
When `no_color` is set and a color system exists, it first replaces every style with its colorless twin ([console.py:2139-2140][console-render-buffer-2139-2140], [style.py:478-494][style-without-color]).
So `NO_COLOR` removes color and keeps bold, underline, and so on.

### 4.2 Where foreign ANSI is parsed

`AnsiDecoder` tokenizes with a regex for CSI, OSC, and a few two-byte escapes ([ansi.py:10-56][ansi-tokenize]).
It keeps a running `Style`.
It maps SGR codes through a table ([ansi.py:59-117][ansi-sgr-map]) and handles `38;5;n`, `38;2;r;g;b`, and the same for background.
It handles OSC 8 links and ignores unknown codes ([ansi.py:120-211][ansi-decoder]).
`Text.from_ansi` joins decoded lines ([text.py:293-329][text-from-ansi]).

Callers of the decoder in the package:

| Caller | Input |
|:--|:--|
| `Pretty.__rich_console__` | The string from `pretty_repr`, i.e. user `__repr__` output ([pretty.py:304-337][pretty-render]) |
| `FileProxy.write` | Text written to a redirected `stdout` or `stderr` while a `Live` is active ([file_proxy.py:11-48][file-proxy]) |

Typer calls `Text.from_ansi` when help text contains `ESC[` ([rich_utils.py:166-184][typer-make-text]).

### 4.3 Does Rich re-parse its own output?

No.
The render pipeline goes from renderable to `Segment` to string and stops.
`Console.capture()` returns the final string, with ANSI if the console has a color system ([console.py:866-879][console-end-capture]).
Recording stores segments, not strings: `_write_buffer` copies the segment buffer into `_record_buffer` when `record=True` ([console.py:2059-2130][console-write-buffer]).
So `export_text`, `export_html`, and `export_svg` read segments (section 5.3).

Untrusted text is a hazard.
The `Text` constructor does not strip ESC ([control.py:9-18][control-strip]).
An ESC in a printed string reaches the sink.
`cell_len` counts ESC as zero width but counts the following `[31m` as four cells ([cells.py:58-59][cells-char-size-58-59]).
Layout then drifts.
This is from reading; no output was captured.

## 5. Terminal detection, recording, capture

### 5.1 Console construction

`Console.__init__` takes many keyword-only arguments ([console.py:619-651][console-init-619-651]).
It reads these at construction:

| Source | Used for |
|:--|:--|
| `JUPYTER_COLUMNS`, `JUPYTER_LINES` | Jupyter size ([console.py:657-670][console-init-657-670]) |
| `COLUMNS`, `LINES` | Width and height if not given. `COLUMNS` has `legacy_windows` subtracted ([console.py:684-691][console-init-width-env]) |
| `NO_COLOR` (non-empty) | `no_color` ([console.py:725-729][console-no-color]) |
| `TTY_INTERACTIVE` (`0` or `1`) | `is_interactive` ([console.py:730-742][console-tty-interactive]) |
| `COLORTERM`, `TERM` | Color system, at construction ([console.py:789-811][console-detect-color]) |

### 5.2 is_terminal, color system, size

`is_terminal` is a property, evaluated on each call ([console.py:930-976][console-is-terminal]).
The order is:

1. The `force_terminal` argument, if not `None`.
2. IDLE: false.
3. Jupyter: false.
4. `TTY_COMPATIBLE`: `"0"` false, `"1"` true.
5. `FORCE_COLOR` ([force-color.org][force-color]): if set, true when non-empty.
6. `file.isatty()`, false on error or if absent.

`TTY_COMPATIBLE` and `TTY_INTERACTIVE` are known here from the code only. No external specification for them was checked.

`is_dumb_terminal` is `TERM` equal to `dumb` or `unknown`, and only when `is_terminal` ([console.py:978-988][console-dumb]).

Color system detection runs once, in the constructor ([console.py:789-811][console-detect-color]):

1. Jupyter: truecolor.
2. Not a terminal, or dumb: none.
3. Windows: legacy gives the Windows system. Otherwise truecolor if the console features say so, else 8-bit.
4. Otherwise, `COLORTERM` of `truecolor` or `24bit` gives truecolor.
5. Otherwise, the suffix after the last `-` in `TERM`: `256color` and `kitty` give 8-bit, `16color` gives standard, anything else gives standard ([console.py:96-100][console-term-colors]).

`NO_COLOR` ([no-color.org][no-color]) is not part of detection.
It is applied at write time and strips color only (section 4.1).

Size ([console.py:1004-1043][console-size]):

1. If both width and height were given, use them (minus 1 on legacy Windows).
2. If dumb terminal: 80 by 25.
3. Try `os.get_terminal_size(fd)` for fds 0, 1, 2 in that order. On Windows only 1 and 2. The first success wins.
4. `COLUMNS` and `LINES` override the result when they are digits.
5. A zero result becomes 80 by 25.

The fd order starts at stdin.
So if output goes to a pipe but stdin is a tty, the width comes from the tty.

The output `file` defaults to `sys.stdout`, or `sys.stderr` if `stderr=True` ([console.py:756-763][console-init-756-763]).

### 5.3 Buffer, write, record, capture, export

- Output is buffered per thread in `ConsoleThreadLocals` ([console.py:535-541][console-thread-locals]). A `with console:` block raises a buffer depth, and the buffer is written when depth returns to zero ([console.py:813-820][console-enter-buffer]).
- `print` collects renderables, applies render hooks, builds options, renders each, optionally crops to the console width, and extends the buffer ([console.py:1652-1756][console-print]).
- `_collect_renderables` turns strings into `Text`, joins adjacent text with `sep`, wraps non-renderable objects in `Pretty`, and applies the console style ([console.py:1500-1587][console-collect]).
- `_check_buffer` drops the buffer when `quiet`, and handles `BrokenPipeError` by pointing `stdout` at `/dev/null` and exiting ([console.py:2030-2057][console-broken-pipe]).
- On Windows, writes are split into chunks of at most 8,192 characters ([console.py:2074-2120][console-write-buffer-2074-2120]).
- `capture()` pushes a buffer level and returns the rendered string on exit ([console.py:310-340][console-capture-class], [console.py:866-879][console-end-capture]).

```python
from rich.console import Console

console = Console(record=True, width=60)
console.print("[bold]hello[/bold]")
text = console.export_text()             # plain text from segments
html = console.export_html(inline_styles=True)
svg  = console.export_svg(title="demo")
```

Exports:

| Export | How | Source |
|:--|:--|:--|
| `export_text` | Join segment text, skipping control segments. With `styles=True`, call `style.render(text)` with the default truecolor system. | [console.py:2192-2222][console-export-text] |
| `export_html` | Simplify segments, escape text, map each style to CSS using a `TerminalTheme`, either inline or as numbered classes. | [console.py:2244-2319][console-export-html] |
| `export_svg` | Draw backgrounds as rectangles and text as `<text>` elements on a cell grid. | [console.py:2352-2360][console-export-svg] |

The HTML export puts `style.link` into `href` without escaping ([console.py:2284-2289][console-export-html-link]).
The SVG export advances `x` by `cell_len` but sets `textLength` from `len(text)` ([console.py:2530-2536][console-svg-text-length]).
Issue [#2742][issue-2742] (open) reports narrow SVG text for Asian languages.

## 6. Widgets

### 6.1 Table

Model:

- `Column` is a dataclass with `header`, `footer`, styles, `justify`, `vertical`, `overflow` (default `ellipsis`), `width`, `min_width`, `max_width`, `ratio`, `no_wrap`, and `highlight` ([table.py:38-129][table-column]).
- `Table` adds `box`, `padding` `(top, right, bottom, left)`, `collapse_padding`, `pad_edge`, `expand`, `show_header`, `show_footer`, `show_edge`, `show_lines`, `leading`, `row_styles` (cycled), `title`, `caption`.
- `Table.grid(...)` builds a borderless, headerless table ([table.py:253-284][table-grid]).
- `add_row` accepts renderables and strings. `end_section=True` forces a rule after the row.

```python
from rich.table import Table

t = Table(title="Releases", show_lines=False, row_styles=["", "dim"])
t.add_column("Name", no_wrap=True)
t.add_column("Notes", ratio=1, overflow="fold")
t.add_row("rich", "terminal rendering")
```

Width algorithm.
`_calculate_column_widths(console, options)` takes `options.max_width` equal to the table width minus borders (`_extra_width` is 2 for the outer edge plus one per inner divider, [table.py:296-303][table-extra-width]) ([table.py:523-586][table-calc]).
Steps:

1. **Measure each column** with `_measure_column` ([table.py:716-753][table-measure-column]).
   - A fixed `width` column returns `(width + padding, width + padding)` capped by `max_width`.
   - Else measure every cell, each wrapped in `Padding` ([table.py:627-698][table-get-cells]). The column minimum is the max of cell minimums, the maximum is the max of cell maximums, capped to `max_width` and clamped by `min_width`/`max_width` plus padding.
   - Padding width is `pad_left + pad_right`, with edge columns zeroed when `pad_edge` is false ([table.py:700-714][table-padding-width]).
2. **Start from maxima.** `widths = [max or 1]`.
3. **Ratio columns** (only if `expand` and some `ratio`): flexible columns get zero fixed width. Space left is `max_width - sum(fixed)`. It is split by `ratio_distribute`, with each flexible column's minimum being its `width or 1` plus padding. Result: `fixed + share` ([table.py:536-553][table-calc-536-553], [_ratio.py:107-140][ratio-distribute]).
4. **If the total exceeds `max_width`, collapse** with `_collapse_widths(widths, wrapable, max_width)` ([table.py:588-625][table-collapse]). Only columns with no fixed `width` and `no_wrap` false may shrink. The loop:
   a. Compute excess = total - max.
   b. Find the widest wrappable width and the second widest distinct width among wrappable columns.
   c. Reduce all columns tied at the widest by up to `min(excess, widest - second)`, split by `ratio_reduce` ([_ratio.py:75-104][ratio-reduce]).
   d. Repeat until the excess is gone, or no wrappable column differs from the next.
5. **Last resort.** If still too wide, reduce every column evenly with `ratio_reduce(excess, [1]*n, widths, widths)`. This touches fixed and no-wrap columns too ([table.py:563-567][table-calc-563-567]).
6. **Re-measure at the final widths.** Each column is measured again with `update_width(width)`, and the new maximum replaces the width ([table.py:569-573][table-calc-569-573]). This lets a column drop below its allotment if the content wrapped narrower.
7. **Grow.** If the total is below `max_width` and `expand`, or below `min_width - extra`, distribute the leftover in proportion to the current widths ([table.py:575-584][table-calc-575-584]).

Notes from reading:

- Cell minimums (the longest word) are never consulted by `_collapse_widths`. Minimums only feed `Table.__rich_measure__` ([table.py:320-351][table-measure]). A long word can therefore be folded even when other columns could have given way.
- With `collapse_padding`, `_get_padding_width` sets `pad_left = 0` and then computes `abs(pad_left - pad_right)` ([table.py:704-706][table-padding-width-704-706]). The result is `pad_right`, which differs from the padding applied in `_get_cells` ([table.py:648-652][table-get-cells-648-652]). The effect on layout was not run.

Rendering ([table.py:755-935][table-render]):

1. Build cells per column, zip into rows.
2. Pick the box, substituting for ASCII or legacy Windows ([box.py:67-83][box-class-67-83]).
3. For each row, `render_lines` each cell at `options.update(width=w, justify=..., no_wrap=..., overflow=..., height=None, highlight=...)` ([table.py:829-844][table-render-cells-829-844]).
4. Take the tallest cell as the row height. Align each cell vertically (header bottom, footer top) and `set_shape` to width and height ([table.py:846-877][table-render-cells-846-877]).
5. Emit the left edge, each cell line joined by the divider, and the right edge.
6. Emit the header rule after the first row, a row rule if `show_lines`, `leading`, or `end_section`, and the bottom edge.

Boxes.
`Box` is built from an 8-line string: top, head, head row, mid, row, foot row, foot, bottom, each 4 characters ([box.py:10-183][box-class]).
There are 19 built-in boxes (ASCII, ASCII2, SQUARE, MINIMAL, SIMPLE, HORIZONTALS, ROUNDED, HEAVY, DOUBLE, MARKDOWN, and variants) plus two substitution maps ([box.py:185-474][box-defs]).
`get_top`, `get_row(level)`, and `get_bottom` build a rule string from column widths.

```python
from rich import box
from rich.table import Table

Table(box=box.MARKDOWN)   # GitHub-style table
Table(box=None)           # no borders, as Typer uses for its help tables
```

### 6.2 Panel

`Panel(renderable, box=ROUNDED, title, subtitle, expand=True, padding=(0, 1), ...)` ([panel.py:141-275][panel-render]).
Steps:

1. Wrap the child in `Padding` if padding is non-zero.
2. Pick the width: full if `expand`, else the child's measured maximum plus 2.
3. Grow the width to fit the title plus 2 cells ([panel.py:220-223][panel-render-220-223]).
4. `render_lines` the child at `width - 2`.
5. Draw the top border with the title inserted via `align_text`, which fills with the border character on the free sides ([panel.py:159-204][panel-render-159-204]).
6. Draw each line between `mid_left` and `mid_right`, then the bottom border with the subtitle.

A string title is parsed as markup ([panel.py:109-123][panel-title]).
`Panel.fit` is the shortcut for `expand=False` ([panel.py:73-107][panel-fit]).

### 6.3 Tree

Iterative traversal with a stack of iterators.
No recursion, so depth is not limited by the call stack ([tree.py:86-174][tree-render]).
Guide characters come from `TREE_GUIDES`: light, heavy (when the guide style is bold), and double (when `underline2`), or ASCII guides ([tree.py:30-35][tree-render-30-35], [tree.py:101-108][tree-render-101-108]).
Each node's label is rendered with `render_lines` at the width left after the prefix.

### 6.4 Columns

`Columns` lays out items in a grid ([columns.py:62-171][columns-render]).
It measures each item's maximum, then tries decreasing column counts until the widest-per-column sum fits.
It builds a `Table.grid(collapse_padding=True, pad_edge=False)` and yields it.
`equal`, `column_first`, `right_to_left`, and `align` options reorder or constrain cells.

### 6.5 Rule

A line of a repeating character, with an optional title centered, left, or right ([rule.py:49-103][rule-render]).
It truncates the title with an ellipsis and falls back to `-` on ASCII-only sinks.
Fill characters are measured with `cell_len` and the final string is cut with `set_cell_size`.

### 6.6 Padding and Align

See 1.5 and 1.6.
`Padding.unpack` accepts 1, 2, or 4 integers in CSS order ([padding.py:60-74][padding-render-60-74]).

### 6.7 Layout

A tree of `Layout` nodes.
Each splits its region into rows or columns ([layout.py:80-138][layout-splitters]).
`_make_region_map` walks the tree with a stack and asks each splitter to divide a `Region` by `ratio_resolve` ([layout.py:345-364][layout-regions-345-364]).
`render` renders every leaf with `render_lines` at its region size, then assembles lines by writing each leaf's lines into row buffers by `y` ([layout.py:366-415][layout-regions-366-415]).
A leaf wider or taller than its region is cropped by `render_lines`.

### 6.8 Live

`Live` is a renderable that is redrawn in place.
It registers itself as a console render hook ([live.py:111-143][live-start-stop-111-143]).
When something prints, the hook inserts a cursor-reset control before the output and appends the live renderable after it ([live.py:278-297][live-refresh-278-297]).
`LiveRender` renders the child with `render_lines`, records its shape, and builds the control codes for the next redraw: carriage return, then `cursor up` and `erase line` for each earlier row ([live_render.py:51-116][live-render]).
Vertical overflow is `crop`, `ellipsis`, or `visible`.

Non-interactive sinks get no redraws.
`refresh` only writes when `is_terminal` and not dumb ([live.py:244-276][live-refresh-244-276]).
A final frame is written once at stop when the sink is not interactive and the live is not transient ([live.py:272-276][live-refresh-272-276], [live.py:292-295][live-refresh-292-295]).

`auto_refresh` runs a daemon thread that calls `refresh` every `1 / refresh_per_second` seconds ([live.py:22-38][live-thread]).
`Live` can redirect `sys.stdout` and `sys.stderr` through `FileProxy` ([live.py:129][live-start-stop-129-129]).
Issue [#4158][issue-4158] (closed) reports a deadlock between `Live._lock` and `Console._lock`.

### 6.9 Status and spinners

`Status` is a `Spinner` inside a transient `Live` ([status.py:23-46][status-init]).
A spinner is a list of frames and an interval in milliseconds.
The frame is a pure function of the clock: `frame = ((now - start) * speed) / interval + offset` ([spinner.py:61-93][spinner-render]).
Nothing ticks the spinner. The refresh thread just re-renders it.

### 6.10 Progress

`Progress` composes a `Live` with a `Table.grid` ([progress.py:1077-1111][progress-init], [progress.py:1562-1593][progress-table]).

- Each `ProgressColumn` is a callable from `Task` to a renderable, with a per-task cache and an optional `max_refresh` ([progress.py:507-546][progress-column]).
- A column gives a `Column` template for width and `no_wrap`.
- On every refresh, the table is rebuilt: one row per visible task, one cell per column.
- `BarColumn` returns a `ProgressBar` renderable that animates a pulse when the task has not started ([progress.py:646-685][progress-bar-column]).
- `Task` stores `completed`, `total`, timestamps, and a deque of `ProgressSample` ([progress.py:935-1052][progress-task]).
- Speed is the sum of samples after the first over the time window. Samples older than `speed_estimate_period` (default 30 s) are dropped on update ([progress.py:1023-1038][progress-task-1023-1038], [progress.py:1460-1467][progress-samples]).

```python
from rich.progress import Progress

with Progress() as p:
    t = p.add_task("download", total=100)
    p.update(t, advance=10)
```

### 6.11 Syntax

`Syntax` uses Pygments to lex, then maps each token type to a `Style` via a `SyntaxTheme` ([syntax.py:141-180][syntax-theme], [syntax.py:470-553][syntax-highlight]).
The output is one `Text` with a span per token.
Line numbers, indent guides, highlight ranges, and word wrap are applied on top ([syntax.py:625-650][syntax-measure]).
Themes are Pygments styles or an ANSI theme that uses the terminal's 16 colors.
Pygments is a hard dependency ([pyproject.toml:29-33][pyproject-deps]).

### 6.12 Markdown

`Markdown` parses with `markdown-it-py` (`MarkdownIt().enable("strikethrough").enable("table")`) ([markdown.py:559][markdown-parser]).
It flattens inline tokens and walks them, pushing block elements (`Paragraph`, `Heading`, `CodeBlock`, `ListElement`, `TableElement`, ...) on a stack and rendering each ([markdown.py:569-577][markdown-render-569-577], [markdown.py:579-670][markdown-render]).
Links become styles with `link=href` when hyperlinks are on, else `text (url)` ([markdown.py:611-644][markdown-link-611-644]).
Code blocks use `Syntax`.
Markdown is a hard dependency too.

### 6.13 Pretty, JSON, Traceback, Prompt

- **Pretty.** `pretty_repr` builds a tree of `Node` objects for containers, dataclasses, attrs, and namedtuples, and renders to a width by expanding only the nodes that do not fit ([pretty.py:878-915][pretty-repr]). `Pretty` then parses the string back through `Text.from_ansi` and highlights it ([pretty.py:304-337][pretty-render]).
- **JSON.** `JSON` runs `json.dumps` and applies `JSONHighlighter`, then sets `no_wrap=True` and `overflow=None` on the text ([json.py:25-51][json-class], [json.py:84-99][json-from-data]). `Console.print_json` prints it with `soft_wrap=True` ([console.py:1801-1817][console-print-json]). A plain `console.print(JSON(...))` has `no_wrap` but not `soft_wrap`, so long lines are cropped to the console width (from reading; not run).
- **Traceback.** Extracts stacks and frames, shows source with `Syntax`, optional locals via `Pretty`, `suppress` paths, and `max_frames` ([traceback.py:262-350][traceback-class]). Default width is 100.
- **Prompt.** A loop: print the prompt, read with `input()` or `getpass`, validate, print an error panel on failure ([prompt.py:280-301][prompt-call], [prompt.py:258-265][prompt-validate], [console.py:2156-2190][console-input]). Prompt text is parsed as markup.

## 7. Typer's use of Rich

Typer `0.28.0` ships its own copy of Click in `typer/_click/` (5,564 lines).
Rich is used in one module, `typer/rich_utils.py` (776 lines).

### 7.1 Switching Rich on and off

- `HAS_RICH = parse_boolean_env_var(os.getenv("TYPER_USE_RICH"), default=True)` ([core.py:24-32][typer-has-rich], [utils.py:189-197][typer-parse-bool]).
- If false, the default markup mode is `None`. Help and errors use the Click formatter ([core.py:975-988][typer-format-help-cmd], [core.py:1208-1217][typer-format-help-group], [core.py:203-242][typer-error-handling]).
- Rich is a hard dependency: `rich >=13.8.0` ([pyproject.toml:35-40][typer-deps]). So "degrading" is a runtime switch, not an `ImportError` fallback.
- The Click fallback formats at `min(terminal width, 80) - 2`, at least 50 ([_click/formatting.py:104-114][typer-click-width]).

Markup modes ([main.py:412-420][typer-markup-mode-doc]):

| Mode | Help text is read as |
|:--|:--|
| `"rich"` (default when Rich is on) | Rich markup, or ANSI if the text contains `ESC[` |
| `"markdown"` | Markdown, with emoji codes replaced first |
| `None` | Plain Click formatting |

See `_make_rich_text` ([rich_utils.py:166-184][typer-make-text]).

### 7.2 The console

A new `Console` is created for each help or error print.
There is no global ([rich_utils.py:145-163][typer-console]):

```python
Console(
    theme=Theme({...}),          # option, switch, negative_*, types, usage
    highlighter=highlighter,     # Typer's own RegexHighlighter
    color_system=COLOR_SYSTEM,   # "auto"
    force_terminal=FORCE_TERMINAL,
    width=MAX_WIDTH,
    stderr=stderr,
)
```

Environment variables, read once at import ([rich_utils.py:72-84][typer-env]):

| Variable | Effect |
|:--|:--|
| `TERMINAL_WIDTH` | Fixed console width |
| `FORCE_COLOR`, `PY_COLORS`, `GITHUB_ACTIONS` | `force_terminal=True` when set to any non-empty value, so `FORCE_COLOR=0` also forces |
| `_TYPER_FORCE_DISABLE_TERMINAL` | `force_terminal=False`, wins over the others |
| `TYPER_USE_RICH` | Rich on or off ([core.py:27][typer-has-rich-27-27]) |

`NO_COLOR`, `COLORTERM`, `TERM`, and `COLUMNS` are left to Rich.

### 7.3 Help panels

`rich_format_help` ([rich_utils.py:555-694][typer-format-help]):

1. Print usage, highlighted, padded.
2. Print the command help text in a padded `Align`.
3. Group parameters by `rich_help_panel`. The default names are "Arguments" and "Options". Custom panels follow the defaults ([rich_utils.py:591-643][typer-group-panels-591-643]).
4. For a group, group subcommands the same way. Compute the longest command name across all panels.
5. Print the epilogue.

`rich_help_panel` is a string on `Option`, `Argument`, commands, and `Typer` ([params.py:908-925][typer-option-panel-param], [main.py:430-445][typer-panel-attr]).

Each parameter panel is a `Panel(Table(...))` ([rich_utils.py:355-476][typer-options-panel]):

- `Table(highlight=True, show_header=False, expand=True, box=None, pad_edge=False, padding=(0, 1))`.
- Columns per row: required star (only if any), long option, short option, negative long, negative short, type, help.
- The help cell is a `Columns` of items: help text, `[env var: ...]`, `[default: ...]`, `[required]`. `Columns` is used because it can place `Text` and `Markdown` side by side ([rich_utils.py:239-325][typer-param-help]).

Each commands panel is a table with two columns ([rich_utils.py:479-552][typer-commands-panel]):

- A fixed-width, no-wrap name column, with `width` set to the longest name across all panels, so the panels line up.
- A description column with `ratio=10`. The comment says a large ratio makes it take the free space, so the name column cannot grow and misalign.

Because the table has `expand=True`, step 3 of the width algorithm in 6.1 is the path in use.

### 7.4 Errors, aborts, tracebacks, docs

- `rich_format_error` writes to a **stderr** console: highlighted usage, a hint `Try 'cmd --help' for help.` with the command path escaped, then a `Panel` titled "Error" with a red border ([rich_utils.py:697-728][typer-error]).
- `rich_abort_error` prints "Aborted." in red ([rich_utils.py:731-734][typer-abort]).
- Pretty exceptions use Rich `Traceback.from_exception` with Typer and Click directories in `suppress`, `width=MAX_WIDTH`, `code_width=None`, `word_wrap=True` ([rich_utils.py:761-776][typer-traceback], [main.py:69-95][typer-excepthook]).
- `rich_to_html` uses `Console(record=True, file=StringIO())` and `export_html(inline_styles=True, code_format="{code}")` for generated docs ([rich_utils.py:737-752][typer-html]).

## 8. Size and dependencies

### 8.1 Dependencies

Rich runtime dependencies from `pyproject.toml` ([pyproject.toml:29-33][pyproject-deps]):

| Package | Constraint | Used by |
|:--|:--|:--|
| Python | `>=3.9.0` | all |
| `pygments` | `^2.13.0` | `syntax.py`, `traceback.py`, `markdown.py` |
| `markdown-it-py` | `>=2.2.0` | `markdown.py` |
| `ipywidgets` | optional (`jupyter` extra) | `live.py` in notebooks |

Typer runtime dependencies ([pyproject.toml:35-40][typer-deps]): `shellingham >=1.3.0`, `rich >=13.8.0`, `annotated-doc >=0.0.2`, and `colorama` on Windows.
Transitive dependencies of `markdown-it-py` were not checked.

### 8.2 Line counts (`wc -l`)

Rich top-level modules: 26,587 lines in total (77 modules).
Subpackage `_unicode_data/`: 11,928 lines in 23 generated files.

| Group | Modules | Lines |
|:--|:--|--:|
| Core | `console.py` 2,698; `text.py` 1,363; `style.py` 796; `segment.py` 780; `measure.py` 151 | 5,788 |
| Core support | `cells.py` 352; `color.py` 621; `box.py` 474; `markup.py` 251; `ansi.py` 241; `highlighter.py` 232; `control.py` 219; `default_styles.py` 196; `_palettes.py` 309; `containers.py` 167; `_ratio.py` 153; `theme.py` 116; `align.py` 320; `padding.py` 141; `_wrap.py` 93; `protocol.py` 41; and small files | 4,842 |
| Widgets and tools | `progress.py` 1,716; `pretty.py` 1,016; `table.py` 1,015; `syntax.py` 988; `traceback.py` 924; `markdown.py` 802; `layout.py` 442; `live.py` 404; `prompt.py` 400; `_spinners.py` 482 (data); `logging.py` 305; `tree.py` 257; `panel.py` 317; `columns.py` 187; `_inspect.py` 272; and more | 10,884 |
| Data | `_emoji_codes.py` | 3,610 |
| Windows console | `_win32_console.py` 661; `_windows.py` 71; `_windows_renderer.py` 56 | 788 |

The groups are from the `wc -l` run on the listed files.
They do not sum to 26,587 because small files (`__init__`, `__main__`, `diagnose`, `_null_file`, and others) are in no group.

Typer: 8,347 lines in `typer/*.py` (`main.py` 2,015; `params.py` 1,833; `core.py` 1,221; `rich_utils.py` 776; `models.py` 743), plus the vendored `_click/` at 5,564 lines.

### 8.3 What is core

Core is five files: `console`, `segment`, `style`, `text`, `measure`.
Add `cells`, `color`, `markup`, `control`, `theme`, and `box` and the library can render `Text`, `Panel`, `Table`, `Rule`, and `Padding`.
`syntax`, `markdown`, `pretty`, `traceback`, `progress`, `live`, `layout`, `prompt`, `logging`, and `json` are leaves that depend on core and on each other only through `Text`, `Table`, `Panel`, and `Live`.

## 9. Design lessons for a Rust library

### 9.1 What to copy

1. **One output unit, deferred ANSI.** Render to `Segment { text, style, control }` and turn styles into escape codes only in the final writer. This is what makes `NO_COLOR`, color downgrade, recording, and HTML or SVG export cheap ([console.py:2132-2154][console-render-buffer]).
2. **A line is a rectangle of cells.** `render_lines` plus `set_shape` and `adjust_line_length` turn any child into a fixed-size grid. Borders, padding, layout, and side-by-side cells then become line concatenation ([console.py:1345-1407][console-render-lines], [segment.py:353-396][segment-adjust]).
3. **Two passes, one options value.** Measure returns `(min, max)`. Render receives an options value that parents copy and narrow. A type without a measure is greedy ([measure.py:116-117][measure-default]). In Rust this is a trait with a default `measure` method and a `render` method that returns lines.
4. **Tri-state style attributes in two bitsets, overlay combine, and a null style.** This gives `bold` on, off, and inherit with a small copyable struct ([style.py:25-36][style-bits], [style.py:732-762][style-add]).
5. **Spans over a string, sweep-line to segments.** Keep overlapping spans cheap and resolve them once at render ([text.py:719-776][text-render]).
6. **Cut a wide character into spaces.** When a cut falls inside a 2-cell glyph, replace it with spaces on both sides to keep the width exact ([cells.py:235-276][cells-split-text]).
7. **Detection as an ordered list of sources.** Explicit argument, then `TTY_COMPATIBLE`, then `FORCE_COLOR`, then `isatty` ([console.py:930-976][console-is-terminal]). Keep "is a terminal" apart from "color system" and from "no color".
8. **Width tables per Unicode version, generated from a reference.** Rich now generates them from `wcwidth` data and lets `UNICODE_VERSION` choose ([_unicode_data/__init__.py:58-93][unicode-load]). In Rust, use a maintained width and segmentation crate and expose the version as data.
9. **Progress is a table that is rebuilt on each refresh.** Columns are small functions from task state to renderable. Spinners are a pure function of time ([progress.py:1562-1593][progress-table], [spinner.py:61-93][spinner-render]).
10. **Live as a render hook.** A print while a live region exists erases the region, prints, and redraws it. Non-interactive sinks skip redraws and get one final frame ([live.py:278-297][live-refresh-278-297]).
11. **Record segments, not strings.** Exports then need no parser ([console.py:2059-2130][console-write-buffer]).
12. **Typer's table pattern for aligned help.** Fixed-width name column across panels and a high-ratio description column ([rich_utils.py:507-516][typer-commands-cols]).
13. **Iterative traversal for deep trees.** `Tree` uses a stack ([tree.py:86-174][tree-render]).

### 9.2 Known flaws, and the fix for each

| # | Flaw | Evidence | Fix in Rust |
|:--|:--|:--|:--|
| 1 | Automatic highlighting is on by default, and the patterns tint substrings of identifiers and numbers. The number rule allows a digit run after a hyphen, so `A-2` colors the `2` ([highlighter.py:97-98][highlighter-repr-number]). `0x` with zero digits matches inside `1920x1080` ([#3854][issue-3854]). The IPv6 rule `([A-Fa-f0-9]{1,4}::?){1,7}[A-Fa-f0-9]{1,4}` matches short hex groups with colons, such as clock times ([highlighter.py:90][highlighter-repr-ipv6], [#3246][issue-3246], open). The hyphen case is [#3306][issue-3306] (closed). | Regex read at v15.0.0; not run | Make highlighting opt-in per call. Match whole tokens split on a delimiter set, not substrings. Never highlight inside an identifier or a version string. |
| 2 | Markup is parsed in many entry points by default: `print`, `Panel` titles, `Spinner` text ([spinner.py:38-40][spinner-init]), prompts ([prompt.py:66-70][prompt-init]). Bracketed lowercase words are consumed as tags, and unknown names disappear without error. | [markup.py:12-15][markup-regex], [text.py:736-740][text-render-736-740], [panel.py:109-123][panel-title], [#3203][issue-3203] | Plain text is the default. Markup is a separate constructor that returns a `Result` and rejects unknown tags. |
| 3 | Untrusted text is not sanitized. Only 5 control characters are stripped. ESC passes to the sink, and `cell_len` miscounts the rest of the sequence. | [control.py:9-18][control-strip], [cells.py:58-59][cells-char-size-58-59] | Strip or escape C0, C1, and ESC at `Text` construction. Validate link URLs for OSC 8. Parse ANSI only through an explicit API. |
| 4 | The ANSI string cache on `Style` is not keyed by color system. | [style.py:344-385][style-ansi], [style.py:712][style-render-712-712] | Key by `ColorSystem`, or do not cache the string on the style. |
| 5 | Style equality by hash. | [style.py:431-445][style-eq] | Derive `Eq` on the fields. |
| 6 | The JSON widget sets `no_wrap` with no soft wrap, so `console.print(JSON(...))` crops long lines. Only `print_json` passes `soft_wrap=True`. | [json.py:84-99][json-from-data], [console.py:1817][console-print-json-1817-1817] | A JSON view should never crop. Give it a "no wrap, no crop" mode that is part of its type. |
| 7 | Global console and global state. `get_console()` caches a module-level `Console`; `reconfigure` replaces its `__dict__`. Locks nest across `Live` and `Console`, with a reported deadlock. | [__init__.py:13-50][global-console], [#4158][issue-4158] | Pass a `Console` or context value. One lock order, or lock-free per-thread buffers. |
| 8 | Grapheme handling is a special case on U+200D and U+FE0F, not UAX #29. Segment splitting had a separate approximation with several rework PRs. | [cells.py:161-232][cells-graphemes], [segment.py:106-153][segment-split-cells-106-153], [#3974][issue-3974], [#4155][issue-4155] | Segment into graphemes once with a crate. Store cell widths per grapheme. Cut only on boundaries. |
| 9 | Offsets mix code points (spans) and cells (layout). `divide_line` documents cell offsets but returns string indices. | [_wrap.py:26-76][wrap-divide-line], [text.py:47-115][text-span] | Use byte offsets on `str` for storage. Convert to cells only through a width cache. Name each unit in the type. |
| 10 | Table collapse ignores minimum widths and can shrink fixed and no-wrap columns in the last-resort step. | [table.py:588-625][table-collapse], [table.py:563-567][table-calc-563-567] | Make the shrink order explicit: wrappable columns down to their minimum first, then the rest. |
| 11 | `Panel` measures as `(w, w)`, so it cannot be shrunk by a parent. | [panel.py:277-297][panel-measure] | Return a real minimum (border plus padding plus child minimum). |
| 12 | `FORCE_COLOR` handling: any non-empty value forces color, including `0`. Typer repeats this with `PY_COLORS` and `GITHUB_ACTIONS`. | [console.py:963-966][console-is-terminal-963-966], [rich_utils.py:78-82][typer-env-78-82] | Follow [force-color.org][force-color] and [no-color.org][no-color] exactly, and document precedence. Allow an explicit `ColorChoice`. |
| 13 | Terminal width is read from stdin first, even when output goes to a pipe. | [console.py:1021-1028][console-size-1021-1028] | Query the output stream first. Fall back to others only if asked. |
| 14 | HTML export writes link text into `href` without escaping. SVG export sets `textLength` from `len`, not cell width. | [console.py:2284-2289][console-export-html-link], [console.py:2530-2536][console-svg-text-length], [#2742][issue-2742] | Escape attribute values. Use cell width for every x and length. |
| 15 | Typer reads its environment once at import (`TERMINAL_WIDTH`, `FORCE_COLOR`), so later changes are ignored. | [rich_utils.py:72-84][typer-env] | Read the environment into a value at construction and pass it. |
| 16 | `Segment.divide` has no internal caller. Dead code in a core module. | `rg` over the package | Do not port it. |

### 9.3 Suggested shape of the Rust API

This is a sketch, not an implementation.

```text
trait Renderable {
    fn measure(&self, opts: &Options) -> Measurement { Measurement::greedy(opts.max_width) }
    fn render(&self, opts: &Options) -> Vec<Line>;   // Line = Vec<Segment>
}
struct Segment { text: String, style: Style, control: Option<Control> }
struct Style { fg: Option<Color>, bg: Option<Color>, set: u16, value: u16, link: Option<Url> }
struct Options { max_width, min_width, height, justify, overflow, no_wrap, caps: Caps }
struct Caps { is_terminal, color: Option<ColorSystem>, no_color, unicode_version, ascii_only }
fn detect(env: &Env, out: &dyn Tty) -> Caps    // pure function of a snapshot
```

Choices that follow from the sections above:

- `Caps` is a value computed by a pure function from an environment snapshot, so tests need no process state (section 5.2).
- `Text` is plain by default. A separate parser builds spans (section 2.6).
- The writer is the only code that emits escape bytes (section 4.1).
- Layout containers work on `Vec<Line>` rectangles (sections 1.4 and 1.5).
- Widgets that need a heavy parser (Syntax, Markdown) sit behind features, as Rich's two hard dependencies show (section 8.1).

## 10. Not verified

- No code was executed. Every behavior statement comes from reading source. The flaws marked "from reading" are not reproduced.
- Issue and pull request states come from GitHub's API on 2026-11-07. Whether [#3306][issue-3306] was fixed in code could not be determined from the state alone. The v15.0.0 regex still reads as in 9.2 row 1.
- Typer packaging (for example a `typer-slim` variant) was not checked in this clone. The pyproject here lists Rich as a plain dependency.
- Transitive dependencies of `markdown-it-py` and `pygments` were not read.
- The terminal behavior of Windows-specific paths (`_win32_console.py`, `_windows_renderer.py`) was only noted, not read in detail.
- `rich/logging.py`, `rich/_inspect.py`, `rich/pager.py`, and `rich/jupyter.py` were not read beyond their size.
- Which external projects document `TTY_COMPATIBLE` and `TTY_INTERACTIVE` was not looked up.

<!-- Link definitions -->

[align-render-143-233]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/align.py#L143-L233
[align-render-235-239]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/align.py#L235-L239
[ansi-decoder]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/ansi.py#L120-L211
[ansi-sgr-map]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/ansi.py#L59-L117
[ansi-tokenize]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/ansi.py#L10-L56
[box-class]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/box.py#L10-L183
[box-class-67-83]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/box.py#L67-L83
[box-defs]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/box.py#L185-L474
[cells-cell-len]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/cells.py#L98-L158
[cells-cell-len-108-110]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/cells.py#L108-L110
[cells-char-size]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/cells.py#L46-L78
[cells-char-size-58-59]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/cells.py#L58-L59
[cells-chop]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/cells.py#L326-L352
[cells-graphemes]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/cells.py#L161-L232
[cells-set-size]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/cells.py#L299-L323
[cells-single]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/cells.py#L13-L35
[cells-split-text]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/cells.py#L235-L276
[color-ansi-codes]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/color.py#L485-L510
[color-class]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/color.py#L303-L313
[color-downgrade]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/color.py#L513-L568
[color-names]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/color.py#L49-L285
[color-parse]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/color.py#L433-L482
[color-regex]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/color.py#L292-L299
[color-systems]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/color.py#L21-L47
[color-triplet]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/color_triplet.py#L4-L38
[columns-render]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/columns.py#L62-L171
[console-broken-pipe]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/console.py#L2030-L2057
[console-capture-class]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/console.py#L310-L340
[console-collect]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/console.py#L1500-L1587
[console-detect-color]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/console.py#L789-L811
[console-dumb]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/console.py#L978-L988
[console-end-capture]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/console.py#L866-L879
[console-enter-buffer]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/console.py#L813-L820
[console-export-html]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/console.py#L2244-L2319
[console-export-html-link]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/console.py#L2284-L2289
[console-export-svg]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/console.py#L2352-L2360
[console-export-text]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/console.py#L2192-L2222
[console-get-style]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/console.py#L1470-L1498
[console-group]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/console.py#L450-L480
[console-init-619-651]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/console.py#L619-L651
[console-init-657-670]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/console.py#L657-L670
[console-init-756-763]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/console.py#L756-L763
[console-init-emoji]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/console.py#L640-L641
[console-init-highlight]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/console.py#L642-L646
[console-init-width-env]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/console.py#L684-L691
[console-input]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/console.py#L2156-L2190
[console-is-terminal]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/console.py#L930-L976
[console-is-terminal-963-966]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/console.py#L963-L966
[console-no-color]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/console.py#L725-L729
[console-options]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/console.py#L112-L140
[console-options-update-157-243]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/console.py#L157-L243
[console-print]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/console.py#L1652-L1756
[console-print-json]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/console.py#L1801-L1817
[console-print-json-1817-1817]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/console.py#L1817
[console-protocols]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/console.py#L246-L271
[console-render]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/console.py#L1294-L1343
[console-render-1317-1324]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/console.py#L1317-L1324
[console-render-1331-1343]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/console.py#L1331-L1343
[console-render-buffer]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/console.py#L2132-L2154
[console-render-buffer-2139-2140]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/console.py#L2139-L2140
[console-render-lines]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/console.py#L1345-L1407
[console-size]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/console.py#L1004-L1043
[console-size-1021-1028]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/console.py#L1021-L1028
[console-soft-wrap]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/console.py#L1695-L1702
[console-svg-text-length]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/console.py#L2530-L2536
[console-term-colors]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/console.py#L96-L100
[console-thread-locals]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/console.py#L535-L541
[console-tty-interactive]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/console.py#L730-L742
[console-write-buffer]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/console.py#L2059-L2130
[console-write-buffer-2074-2120]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/console.py#L2074-L2120
[constrain]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/constrain.py#L22-L37
[containers-lines]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/containers.py#L66-L167
[containers-lines-111-167]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/containers.py#L111-L167
[containers-renderables]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/containers.py#L30-L63
[control-formats]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/control.py#L28-L40
[control-strip]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/control.py#L9-L18
[control-strip-fn]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/control.py#L181-L192
[default-styles]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/default_styles.py#L5-L30
[emoji-replace]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/_emoji_replace.py#L9-L31
[file-proxy]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/file_proxy.py#L11-L48
[force-color]: https://force-color.org/
[global-console]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/__init__.py#L13-L50
[highlighter-json]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/highlighter.py#L106-L130
[highlighter-regex]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/highlighter.py#L61-L77
[highlighter-regex-17-48]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/highlighter.py#L17-L48
[highlighter-repr]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/highlighter.py#L80-L103
[highlighter-repr-ipv6]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/highlighter.py#L90
[highlighter-repr-number]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/highlighter.py#L97-L98
[issue-2742]: https://github.com/Textualize/rich/issues/2742
[issue-3203]: https://github.com/Textualize/rich/issues/3203
[issue-3246]: https://github.com/Textualize/rich/issues/3246
[issue-3306]: https://github.com/Textualize/rich/issues/3306
[issue-3854]: https://github.com/Textualize/rich/issues/3854
[issue-3897]: https://github.com/Textualize/rich/issues/3897
[issue-3947]: https://github.com/Textualize/rich/issues/3947
[issue-3974]: https://github.com/Textualize/rich/pull/3974
[issue-4146]: https://github.com/Textualize/rich/pull/4146
[issue-4155]: https://github.com/Textualize/rich/pull/4155
[issue-4158]: https://github.com/Textualize/rich/issues/4158
[json-class]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/json.py#L25-L51
[json-from-data]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/json.py#L84-L99
[layout-regions]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/layout.py#L345-L415
[layout-regions-345-364]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/layout.py#L345-L364
[layout-regions-366-415]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/layout.py#L366-L415
[layout-splitters]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/layout.py#L80-L138
[live-refresh-244-276]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/live.py#L244-L276
[live-refresh-272-276]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/live.py#L272-L276
[live-refresh-278-297]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/live.py#L278-L297
[live-refresh-292-295]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/live.py#L292-L295
[live-render]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/live_render.py#L51-L116
[live-start-stop-111-143]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/live.py#L111-L143
[live-start-stop-129-129]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/live.py#L129
[live-thread]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/live.py#L22-L38
[markdown-link-611-644]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/markdown.py#L611-L644
[markdown-parser]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/markdown.py#L559
[markdown-render]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/markdown.py#L579-L670
[markdown-render-569-577]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/markdown.py#L569-L577
[markup-escape]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/markup.py#L48-L70
[markup-parse]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/markup.py#L73-L103
[markup-parse-83-98]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/markup.py#L83-L98
[markup-regex]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/markup.py#L12-L15
[markup-render]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/markup.py#L106-L231
[markup-render-159-176]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/markup.py#L159-L176
[markup-render-178-215]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/markup.py#L178-L215
[measure-default]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/measure.py#L116-L117
[measure-renderables]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/measure.py#L125-L151
[measurement]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/measure.py#L11-L57
[measurement-get]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/measure.py#L78-L122
[measurement-get-98-101]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/measure.py#L98-L101
[no-color]: https://no-color.org/
[padding-render]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/padding.py#L79-L135
[padding-render-125-135]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/padding.py#L125-L135
[padding-render-60-74]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/padding.py#L60-L74
[panel-fit]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/panel.py#L73-L107
[panel-measure]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/panel.py#L277-L297
[panel-render]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/panel.py#L141-L275
[panel-render-159-204]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/panel.py#L159-L204
[panel-render-210-216]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/panel.py#L210-L216
[panel-render-220-223]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/panel.py#L220-L223
[panel-title]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/panel.py#L109-L123
[pr-3930]: https://github.com/Textualize/rich/pull/3930
[pretty-render]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/pretty.py#L304-L337
[pretty-repr]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/pretty.py#L878-L915
[progress-bar-column]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/progress.py#L646-L685
[progress-column]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/progress.py#L507-L546
[progress-init]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/progress.py#L1077-L1111
[progress-samples]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/progress.py#L1460-L1467
[progress-table]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/progress.py#L1562-L1593
[progress-task]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/progress.py#L935-L1052
[progress-task-1023-1038]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/progress.py#L1023-L1038
[prompt-call]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/prompt.py#L280-L301
[prompt-init]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/prompt.py#L66-L70
[prompt-validate]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/prompt.py#L258-L265
[protocol-cast]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/protocol.py#L18-L41
[protocol-check]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/protocol.py#L9-L15
[pyproject-deps]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/pyproject.toml#L29-L33
[ratio-distribute]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/_ratio.py#L107-L140
[ratio-reduce]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/_ratio.py#L75-L104
[ratio-resolve]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/_ratio.py#L14-L72
[rich-repo]: https://github.com/Textualize/rich/tree/6ac483cbea39cab124dfd3483bba70ffafb71050
[rule-render]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/rule.py#L49-L103
[segment-adjust]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/segment.py#L353-L396
[segment-class]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/segment.py#L61-L104
[segment-control-type]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/segment.py#L32-L50
[segment-divide]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/segment.py#L629-L696
[segment-remove-color]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/segment.py#L607-L627
[segment-set-shape]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/segment.py#L425-L459
[segment-simplify]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/segment.py#L550-L575
[segment-split-cells]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/segment.py#L106-L179
[segment-split-cells-106-153]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/segment.py#L106-L153
[segment-split-crop]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/segment.py#L306-L351
[spinner-init]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/spinner.py#L38-L40
[spinner-render]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/spinner.py#L61-L93
[status-init]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/status.py#L23-L46
[style-add]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/style.py#L732-L762
[style-add-732-732]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/style.py#L732
[style-ansi]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/style.py#L344-L385
[style-bits]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/style.py#L25-L36
[style-class]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/style.py#L40-L205
[style-class-160-197]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/style.py#L160-L197
[style-combine]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/style.py#L600-L624
[style-copy]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/style.py#L626-L646
[style-eq]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/style.py#L431-L445
[style-null]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/style.py#L207-L210
[style-parse]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/style.py#L496-L561
[style-render]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/style.py#L694-L718
[style-render-712-712]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/style.py#L712
[style-str]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/style.py#L294-L338
[style-without-color]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/style.py#L478-L494
[syntax-highlight]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/syntax.py#L470-L553
[syntax-measure]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/syntax.py#L625-L650
[syntax-theme]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/syntax.py#L141-L180
[table-calc]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/table.py#L523-L586
[table-calc-536-553]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/table.py#L536-L553
[table-calc-563-567]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/table.py#L563-L567
[table-calc-569-573]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/table.py#L569-L573
[table-calc-575-584]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/table.py#L575-L584
[table-collapse]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/table.py#L588-L625
[table-column]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/table.py#L38-L129
[table-column-90-90]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/table.py#L90
[table-extra-width]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/table.py#L296-L303
[table-get-cells]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/table.py#L627-L698
[table-get-cells-648-652]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/table.py#L648-L652
[table-grid]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/table.py#L253-L284
[table-measure]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/table.py#L320-L351
[table-measure-column]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/table.py#L716-L753
[table-padding-width]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/table.py#L700-L714
[table-padding-width-704-706]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/table.py#L704-L706
[table-render]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/table.py#L755-L935
[table-render-cells-829-844]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/table.py#L829-L844
[table-render-cells-846-877]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/table.py#L846-L877
[text-divide]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/text.py#L1106-L1183
[text-from-ansi]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/text.py#L293-L329
[text-highlight-regex]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/text.py#L593-L631
[text-init]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/text.py#L144-L165
[text-init-156-156]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/text.py#L156
[text-measure]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/text.py#L707-L717
[text-render]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/text.py#L719-L776
[text-render-736-740]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/text.py#L736-L740
[text-span]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/text.py#L47-L115
[text-stylize]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/text.py#L457-L481
[text-truncate]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/text.py#L859-L884
[text-wrap]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/text.py#L1201-L1250
[theme-class]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/theme.py#L7-L27
[theme-class-37-57]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/theme.py#L37-L57
[theme-stack]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/theme.py#L81-L106
[traceback-class]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/traceback.py#L262-L350
[tree-measure]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/tree.py#L176-L201
[tree-render]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/tree.py#L86-L174
[tree-render-101-108]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/tree.py#L101-L108
[tree-render-30-35]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/tree.py#L30-L35
[typer-abort]: https://github.com/fastapi/typer/blob/bf5cfc1b5ea5092254d1c3924eeccf13d48605eb/typer/rich_utils.py#L731-L734
[typer-click-width]: https://github.com/fastapi/typer/blob/bf5cfc1b5ea5092254d1c3924eeccf13d48605eb/typer/_click/formatting.py#L104-L114
[typer-commands-cols]: https://github.com/fastapi/typer/blob/bf5cfc1b5ea5092254d1c3924eeccf13d48605eb/typer/rich_utils.py#L507-L516
[typer-commands-panel]: https://github.com/fastapi/typer/blob/bf5cfc1b5ea5092254d1c3924eeccf13d48605eb/typer/rich_utils.py#L479-L552
[typer-console]: https://github.com/fastapi/typer/blob/bf5cfc1b5ea5092254d1c3924eeccf13d48605eb/typer/rich_utils.py#L145-L163
[typer-deps]: https://github.com/fastapi/typer/blob/bf5cfc1b5ea5092254d1c3924eeccf13d48605eb/pyproject.toml#L35-L40
[typer-env]: https://github.com/fastapi/typer/blob/bf5cfc1b5ea5092254d1c3924eeccf13d48605eb/typer/rich_utils.py#L72-L84
[typer-env-78-82]: https://github.com/fastapi/typer/blob/bf5cfc1b5ea5092254d1c3924eeccf13d48605eb/typer/rich_utils.py#L78-L82
[typer-error]: https://github.com/fastapi/typer/blob/bf5cfc1b5ea5092254d1c3924eeccf13d48605eb/typer/rich_utils.py#L697-L728
[typer-error-handling]: https://github.com/fastapi/typer/blob/bf5cfc1b5ea5092254d1c3924eeccf13d48605eb/typer/core.py#L203-L242
[typer-excepthook]: https://github.com/fastapi/typer/blob/bf5cfc1b5ea5092254d1c3924eeccf13d48605eb/typer/main.py#L69-L95
[typer-format-help]: https://github.com/fastapi/typer/blob/bf5cfc1b5ea5092254d1c3924eeccf13d48605eb/typer/rich_utils.py#L555-L694
[typer-format-help-cmd]: https://github.com/fastapi/typer/blob/bf5cfc1b5ea5092254d1c3924eeccf13d48605eb/typer/core.py#L975-L988
[typer-format-help-group]: https://github.com/fastapi/typer/blob/bf5cfc1b5ea5092254d1c3924eeccf13d48605eb/typer/core.py#L1208-L1217
[typer-group-panels-591-643]: https://github.com/fastapi/typer/blob/bf5cfc1b5ea5092254d1c3924eeccf13d48605eb/typer/rich_utils.py#L591-L643
[typer-has-rich]: https://github.com/fastapi/typer/blob/bf5cfc1b5ea5092254d1c3924eeccf13d48605eb/typer/core.py#L24-L32
[typer-has-rich-27-27]: https://github.com/fastapi/typer/blob/bf5cfc1b5ea5092254d1c3924eeccf13d48605eb/typer/core.py#L27
[typer-html]: https://github.com/fastapi/typer/blob/bf5cfc1b5ea5092254d1c3924eeccf13d48605eb/typer/rich_utils.py#L737-L752
[typer-make-text]: https://github.com/fastapi/typer/blob/bf5cfc1b5ea5092254d1c3924eeccf13d48605eb/typer/rich_utils.py#L166-L184
[typer-markup-mode-doc]: https://github.com/fastapi/typer/blob/bf5cfc1b5ea5092254d1c3924eeccf13d48605eb/typer/main.py#L412-L420
[typer-option-panel-param]: https://github.com/fastapi/typer/blob/bf5cfc1b5ea5092254d1c3924eeccf13d48605eb/typer/params.py#L908-L925
[typer-options-panel]: https://github.com/fastapi/typer/blob/bf5cfc1b5ea5092254d1c3924eeccf13d48605eb/typer/rich_utils.py#L355-L476
[typer-panel-attr]: https://github.com/fastapi/typer/blob/bf5cfc1b5ea5092254d1c3924eeccf13d48605eb/typer/main.py#L430-L445
[typer-param-help]: https://github.com/fastapi/typer/blob/bf5cfc1b5ea5092254d1c3924eeccf13d48605eb/typer/rich_utils.py#L239-L325
[typer-parse-bool]: https://github.com/fastapi/typer/blob/bf5cfc1b5ea5092254d1c3924eeccf13d48605eb/typer/utils.py#L189-L197
[typer-repo]: https://github.com/fastapi/typer/tree/bf5cfc1b5ea5092254d1c3924eeccf13d48605eb
[typer-traceback]: https://github.com/fastapi/typer/blob/bf5cfc1b5ea5092254d1c3924eeccf13d48605eb/typer/rich_utils.py#L761-L776
[unicode-load]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/_unicode_data/__init__.py#L58-L93
[unicode-table-head]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/_unicode_data/unicode17-0-0.py#L1-L11
[wrap-divide-line]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/_wrap.py#L26-L76
[wrap-words]: https://github.com/Textualize/rich/blob/6ac483cbea39cab124dfd3483bba70ffafb71050/rich/_wrap.py#L9-L23
