# Charm terminal rendering stack: a code survey for a Rust rendering library

This survey covers lipgloss, glamour, fang, colorprofile, and the `ansi` and `cellbuf` packages of `charmbracelet/x`.
Every claim links to the source at a pinned commit.
No code was built or run.
Where a statement comes from reading and not from execution, it says so.

## 0. Scope and versions

Each repository was shallow-cloned at its latest release tag.
The links in this file use the commit SHA, not the tag.

| Repository | Tag | Commit | Note |
|:--|:--|:--|:--|
| lipgloss (v2) | `v2.0.6` | `733ce53541bb688fd735524b665e0d96e35433bb` | Module `charm.land/lipgloss/v2` |
| lipgloss (v1) | `v1.1.0` | `f0e45475a64ee60d712b81145172d3739db36a93` | Latest v1 tag. Read only to note differences |
| glamour | `v2.0.1` | `95c93db04489cebf252ec856584445c7e85ee276` | Module `charm.land/glamour/v2` |
| colorprofile | `v0.4.3` | `d085584efb48f2ad470e96cd0f3dcb8cc68a034b` | |
| fang | `v2.0.1` | `d89b30af5d6d3a6e3871631d0b62afe2e8c97739` | Module `charm.land/fang/v2` |
| x/ansi | `ansi/v0.11.9` | `faa4adf95555331dfb439b716cc83767ecdadeb4` | Sparse checkout of `ansi/` |
| x/cellbuf | `cellbuf/v0.0.15` | `a1c614051099a0a065aaf1c8fa98f6110e492cbe` | Sparse checkout of `cellbuf/` |

The consumers pin older dependency versions than the tags above.
lipgloss v2.0.6 pins `x/ansi v0.11.8` and `colorprofile v0.4.3` ([go.mod][lg-gomod]).
glamour v2.0.1 pins `lipgloss v2.0.4` and `x/ansi v0.11.7` ([go.mod][gl-gomod]).
fang v2.0.1 pins `lipgloss v2.0.1`, `colorprofile v0.4.2`, and `x/ansi v0.11.6` ([go.mod][fg-gomod]).
A `git diff` between `ansi/v0.11.8` and `ansi/v0.11.9` touches only `programstatus.go` and `go.mod`.
So the `x/ansi` lines cited here are the same as in the version lipgloss pins.
For the other pins I did not diff.

## 1. Summary

1. lipgloss is a string-in, string-out layout library.
   Every operation (`Render`, `JoinHorizontal`, `Place`, table, tree) takes strings that may contain ANSI and returns a string.
   Each operation re-scans its input with the `x/ansi` state machine to measure or cut it.
2. `Style` is a plain value type with a bitmask that records which properties were set.
   Inheritance fills only unset properties.
   v2 removed the renderer and the colour detection from `Style`.
   `Render` always emits full-fidelity SGR and a writer downsamples later.
3. Colour capability comes from `colorprofile.Detect(writer, env)`.
   It combines TTY state, `NO_COLOR`, `CLICOLOR`, `CLICOLOR_FORCE`, `TERM`, `COLORTERM`, terminfo, and a `tmux info` subprocess.
   Background detection is a separate, explicit OSC 11 query.
4. Several costs are visible in the code.
   Wrapping needs two passes to keep styles across line breaks.
   Underline and strikethrough wrap every rune in its own escape pair.
   glamour scans each paragraph at least five times.
5. fang picks its colour scheme with `isDark := false` unless stdout is a TTY, so it chooses the light palette whenever stdout is not a terminal.
   It also checks stdout for the error path, which writes to stderr.

## 2. lipgloss

### 2.1 The Style type

`Style` is a struct that holds values inline and a bitmask of which properties are set.
There are 50 property keys, declared as `1 << iota` constants, in a 64-bit integer ([style.go:17-109][lg-style-props]).
Boolean values live in a second integer, `attrs`.
Colours are `color.Color` interface values.
The struct also holds a slice (border blend colours) and a function (`transform`) ([style.go:141-195][lg-style-struct]).

Builders take the struct by value and return the modified copy.
This makes a `Style` effectively immutable at the call site:

```go
func (s Style) Bold(v bool) Style {
	s.set(boldKey, v)
	return s
}
```

([set.go:194-198][lg-bold], [set.go:9-108][lg-set]).
`NewStyle()` returns the zero struct ([style.go:137-139][lg-newstyle]).
The copy is shallow.
`set` stores the blend-colour slice without copying it ([set.go:63-64][lg-set]).

The mask matters for three reasons.

1. A getter returns a default when the bit is clear.
   `getAsBool(key, default)` returns the default for an unset property ([get.go:504-509][lg-getasbool]).
2. `Inherit` copies a property from the parent only when the child has not set it.
   It skips margins and padding, and copies a background colour into the margin background if the margin has none ([style.go:233-265][lg-inherit]).
3. `Unset*` clears only the bit.
   `UnsetUnderline` calls `Underline(false)`, which sets the underline property to "none" and leaves the bit set ([unset.go:3-6][lg-unset], [unset.go:20-23][lg-unset-underline]).
   Reading `Inherit`, a style that called `UnsetUnderline` therefore still blocks inheritance of the parent's underline.
   I did not run this.

In v1 every `Style` carried a `*Renderer` that held the output, the colour profile, and the background answer ([style.go:116-121][lg1-style-struct], [renderer.go:12-29][lg1-renderer]).
v2 removed it ([UPGRADE_GUIDE_V2.md:216-243][lg-upgrade-renderer]).

### 2.2 What Render does

`Style.Render(strs...)` is one long function ([style.go:267-526][lg-render]).
The order of steps is the design.

1. Join the arguments with one space, then run the optional `transform` function.
2. Return early if no property is set.
3. Build three `ansi.Style` values: text, spaces, and whitespace (padding).
4. Replace tabs with spaces (default width 4), turn `\r\n` into `\n`, and drop newlines in inline mode ([style.go:397-405][lg-render-prep]).
5. Subtract the border size from the width, then word-wrap the raw text at `width - paddingLeft - paddingRight` ([style.go:407-415][lg-render-wrap]).
6. Style each line separately with `te.Styled(line)`, which produces SGR, then the text, then a full reset.
   Add the hyperlink escape pair around the block ([style.go:417-447][lg-render-core]).
7. Apply padding by rewriting every line.
   Top and bottom padding are bare newlines ([style.go:449-478][lg-render-padding]).
8. Apply the height, then horizontal alignment.
   Alignment also pads ragged lines to equal width ([style.go:480-498][lg-render-align], [align.go:12-59][lg-align]).
9. Apply the border, then the margins ([style.go:500-503][lg-render-border], [style.go:543-580][lg-margins]).
10. Truncate each line to `MaxWidth` with `ansi.Truncate`, and cut lines to `MaxHeight` ([style.go:505-523][lg-render-max]).

A short example of the API:

```go
box := lipgloss.NewStyle().
	Width(40).
	Padding(1, 2).
	Border(lipgloss.RoundedBorder()).
	Align(lipgloss.Center).
	Render("hello")
```

The box model changed between v1 and v2.
In v2 the width includes the border (`width -= horizontalBorderSize`, [style.go:407-409][lg-render-wrap]).
In v1 `Render` computes the wrap width as `width - leftPadding - rightPadding` and never subtracts the border ([style.go:365-369][lg1-render-wrap]).
That is a read-only comparison of the two functions.
An open issue titled "`Style.Width()` and `Style.GetHorizontalFrameSize()` both include padding" exists ([#298][issue-lg-298], title only).

### 2.3 Measuring width

`lipgloss.Width` is the maximum over lines of `ansi.StringWidth`.
`Height` counts newlines plus one ([size.go:9-40][lg-size]).
The doc comment tells callers to use it "instead of `len(string)`".
The library cannot stop a caller from using `len`, because the interchange type is a plain string.

`ansi.StringWidth` runs the parser table byte by byte, skips escape sequences, and for each printable start takes the first grapheme cluster and adds its width ([width.go:61-119][ansi-width]).

- Segmentation uses `clipperhouse/uax29` graphemes.
  Width uses `clipperhouse/displaywidth` ([parser_decode.go:433-451][ansi-cluster]).
- A second mode, `WcWidth`, sums per-rune wcwidth values.
  It uses a bitset for zero-width marks and format characters in the BMP ([wcwidth.go:9-71][ansi-wcwidth]).
  The comment explains why it exists.
  Terminals without Unicode core mode (DEC mode 2027) advance the cursor once per code point, so a ZWJ emoji or an Indic conjunct can be wider on screen than its cluster width ([wcwidth.go:73-99][ansi-wcwidth-why]).
- The mode is a value (`type Method`), and each function has a `Wc` twin ([method.go:27-35][ansi-method]).
  The East Asian ambiguous setting is read from `RUNEWIDTH_EASTASIAN` once at package init ([method.go:11-25][ansi-method]).

The stack uses more than one width library.
lipgloss borders use `displaywidth` for edge widths and `uniseg` to walk graphemes in gradient borders ([borders.go:547-579][lg-border-blend]).
cellbuf builds cells with `go-runewidth` and `uniseg` ([buffer.go:12-76][cb-newcell]).
`ansi.ByteToGraphemeRange` uses `displaywidth` directly ([truncate.go:267-290][ansi-bytegraph]).
Issue [#562][issue-lg-562] reports ZWJ emoji misalignment on lipgloss v1.1.0.
The report's root-cause claim is the reporter's, and I did not reproduce it.

### 2.4 The x/ansi parser

The parser is a table-driven DEC ANSI state machine (vt100.net's design) with one added state, `Utf8State` ([parser/const.go:37-60][ansi-states], [transition_table.go:75-80][ansi-table-doc]).
The table is a flat byte slice of 4096 entries.
The index is `state << 8 | byte`, and the value is `action << 4 | nextState` ([transition_table.go:1-64][ansi-table]).
OSC strings accept bytes up to `0xFF` so UTF-8 passes through ([transition_table.go:259-270][ansi-osc]).

There are two ways to use it.

1. **Stateless scans.**
   `Strip`, `StringWidth`, `Hardwrap`, `Wordwrap`, `Wrap`, `Truncate`, and `TruncateLeft` loop over the string and call `parser.Table.Transition(state, byte)` directly.
   They build no `Parser` object and take no callbacks ([width.go:11-59][ansi-width], [truncate.go:66-166][ansi-truncate]).
2. **The full `Parser`.**
   `Parser.Advance(byte)` collects parameters (packed into ints with a "has more" flag for sub-parameters), data strings, and the command, then calls a `Handler` struct of optional callbacks ([parser.go:19-64][ansi-parser], [parser.go:158-240][ansi-advance], [parser_handler.go:34-60][ansi-handler]).
   Parsers are pooled, with a 4 KB data buffer per pooled parser and 64 KB for `NewParser` ([parser_sync.go:9-29][ansi-pool]).
   `DecodeSequence` is the chunk-friendly form: it returns the sequence, its width, the bytes read, and the next state ([method.go:102-148][ansi-decode]).

`Advance` contains `panic("invalid rune")` on a path the comment calls unreachable ([parser.go:181-193][ansi-advance-utf8]).

### 2.5 Wrapping, truncating, and foreign ANSI

`ansi.Wrap` is a single pass.
Escape sequences are appended to the current `word` buffer and add no width.
Plain text goes through space, hyphen, breakpoint, and hard-wrap rules ([wrap.go:293-463][ansi-wrap]).
It does not track the SGR state across the newline it inserts.
The cellbuf source says so:

> The problem is that `ansi.Wrap` doesn't keep track of style and link state, so combining both breaks styled space cells.
> ([cellbuf/wrap.go:24-32][cb-wrap-todo])

lipgloss v2 fixes this with a second pass.
`lipgloss.Wrap` calls `ansi.Wrap`, then streams the result through a `WrapWriter`.
The writer feeds every byte to an `ansi.Parser`, reads SGR and OSC 8 with `uv.ReadStyle` and `uv.ReadLink`, and on each `\n` writes a reset, the newline, then the saved style and link ([wrap.go:11-19][lg-wrap], [wrap.go:21-113][lg-wrapwriter]).

```go
func Wrap(s string, width int, breakpoints string) string {
	var buf bytes.Buffer
	s = ansi.Wrap(s, width, breakpoints)
	w := NewWrapWriter(&buf)
	defer w.Close()
	_, _ = io.WriteString(w, s)
	return buf.String()
}
```

So a wrap costs two scans of the text.
`WrapWriter.Write` also forwards one byte per call to the underlying writer ([wrap.go:72-93][lg-wrapwriter-write]).
v1 called `cellbuf.Wrap` instead ([style.go:365-369][lg1-render-wrap]).
That function carries its own pen tracking, with a TODO to switch to `PenWriter` ([cellbuf/wrap.go:14-32][cb-wrap-todo], [pen.go:9-37][cb-pen]).

`ansi.Truncate` first calls `StringWidth` to return early.
After it reaches the limit it stops copying printable text but keeps copying escape sequences to the end, so open styles and links are closed ([truncate.go:66-166][ansi-truncate]).
`TruncateLeft` does the same from the other side ([truncate.go:186-265][ansi-truncate-left]).

Two things in `truncate.go` look wrong on reading.
I did not run them.

- `cut` under `WcWidth` assigns `truncateLeft = TruncateWc`, which is the right-hand truncate, though `TruncateLeftWc` exists ([truncate.go:30-46][ansi-cut], [truncate.go:177-184][ansi-truncate-left]).
- `truncate` measures with the grapheme `StringWidth` even when the method is `WcWidth` ([truncate.go:66-69][ansi-truncate]).

Foreign ANSI in the input is handled because every scan understands escape sequences.
Nothing in `Render` re-applies the outer style after an inner reset.
`Styled` writes the opening SGR, the text, and a full reset `ESC[m` ([ansi/style.go:51-58][ansi-style-styled]).
So an inner `Styled` span ends the outer style until the next line is styled.
This follows from the code.
I did not test it.

### 2.6 Join and Place

`JoinHorizontal(pos, blocks...)` splits each block into lines.
`getLines` replaces tabs with four spaces and `\r\n` with `\n`, then measures each line ([get.go:630-643][lg-getlines]).
It pads the shorter blocks with empty lines according to a 0 to 1 position.
Then it concatenates line by line and pads each block cell with spaces up to the block's widest line ([join.go:28-96][lg-join-h]).
`JoinVertical` pads every line to the widest ([join.go:116-175][lg-join-v]).
`Position` is a `float64` clamped to 0..1, with constants for `Top`, `Center`, `Left`, and so on ([position.go:19-32][lg-position]).

`Place`, `PlaceHorizontal`, and `PlaceVertical` fill a box with a whitespace renderer.
The renderer cycles through a character string and optionally applies a `Style` ([position.go:36-134][lg-place], [whitespace.go:24-59][lg-whitespace]).
Join pads with plain spaces and takes no style option.

Every one of these operations splits on `\n` and calls `StringWidth` on each line.

### 2.7 Borders

`Border` is a struct of 13 strings: four edges, four corners, and the middle pieces used by tables ([borders.go:14-30][lg-border-struct]).
`applyBorder` measures the block, fills empty corners with a space, and limits corners to one rune.
It builds the top and bottom edges by repeating the middle string until the width is reached, and cycles multi-rune side strings by line ([borders.go:327-424][lg-apply-border], [borders.go:431-524][lg-apply-border-render]).
With a gradient (`BorderForegroundBlend`), each grapheme of an edge gets its own SGR, then one reset at the end ([borders.go:541-562][lg-border-blend]).

### 2.8 lipgloss/table

**Model.**
`Data` is an interface with `At(row, col) string`, `Rows()`, and `Columns()`.
Cells are strings ([table/rows.go:3-13][lg-table-data]).
`StyleFunc func(row, col int) lipgloss.Style` styles each cell, with `HeaderRow = -1` for the header ([table.go:11-37][lg-table-stylefunc]).
The table calls the function for every cell at least twice per render: once in the resizer and once when it builds the row ([table/resizing.go:84-97][lg-resize-style], [table.go:115-121][lg-table-style], [table.go:508-518][lg-table-row-cell]).
The result is merged with the base style by `Inherit`.

**Column widths.**
`resize` runs these steps ([resizing.go:11-108][lg-resize]).

1. Measure every cell with `lipgloss.Width`.
   Record the min, max, and median width per column.
   The first row seen for each column is not added to the list used for the median ([resizing.go:145-187][lg-resize-new]).
2. Read each cell's `StyleFunc` result for the column's horizontal frame size (padding plus margins) and any fixed `Width`.
3. If no table width is set, use the sum of the maximum cell widths plus padding plus borders ([resizing.go:197-200][lg-resize-detect]).
4. If everything fits, expand.
   Start from max-plus-padding per column.
   While the total is below the target, add one cell to the narrowest column that is not fixed ([resizing.go:203-231][lg-resize-expand]).
   The doc comment says columns expand "evenly" ([resizing.go:17-21][lg-resize]).
   The code equalises the narrowest first.
5. If it does not fit, shrink in up to three passes, each removing one cell at a time in a loop.
   First only columns at least half the table width, then the column that exceeds its median the most, then the widest column.
   Each pass has a floor of padding plus the smallest cell width in the column.
   If the table is still too wide, the same passes repeat without floors ([resizing.go:241-331][lg-resize-shrink]).
6. Compute row heights by running `ansi.Wrap` on every cell at its final width and counting newlines.
   Header rows are forced to height 1 ([resizing.go:334-352][lg-resize-heights], [resizing.go:439-448][lg-resize-content-height]).

**Height and scroll.**
An optional fixed height with a row offset picks the visible rows and renders a "…" overflow row ([resizing.go:457-512][lg-resize-visible], [table.go:356-360][lg-table-string]).

**Rendering.**
Each cell is a `Style` with `Height`, `Width`, `MaxHeight`, `MaxWidth` applied, then `Render`.
That wraps the text a second time.
The left and right borders are the border character repeated once per row line, joined with the cells by `JoinHorizontal` ([table.go:484-553][lg-table-row]).
The finished string goes through one more `Style` with `MaxWidth` and `MaxHeight` ([table.go:364-367][lg-table-string]).
With `Wrap(false)`, cells are cut with `ansi.Truncate(cell, length, "…")` ([table.go:555-568][lg-table-truncate]).

### 2.9 lipgloss/list and lipgloss/tree

`List` is a wrapper around `tree.Tree` ([list/list.go:30-55][lg-list]).
The tree node interface has `Value() string`, `Children()`, `Hidden()`, and setters ([tree/tree.go:35-43][lg-tree-node]).
`SetValue` turns any `fmt.Stringer` into a string, so a nested table or tree is flattened to its rendered text ([tree.go:69-79][lg-tree-setvalue]).

The renderer is recursive ([tree/renderer.go:47-165][lg-tree-render]).

1. For each child, ask the enumerator function for a prefix such as `├──`, render the prefix with its style, and find the widest.
2. Pad shorter prefixes on the left, render the item with its item style, and make multi-line prefixes the same height with `JoinVertical`.
3. `JoinHorizontal(indentPrefix, enumeratorPrefix, item)` builds the line.
4. Recurse into children with `prefix + indent`.

Enumerators and indenters are plain functions of `(children, index)`, so a user supplies a closure ([tree/enumerator.go:15-74][lg-tree-enum]).
List enumerators (`Arabic`, `Alphabet`, `Roman`, `Bullet`, `Dash`, `Asterisk`) are in [list/enumerator.go:44-128][lg-list-enum].

### 2.10 Layers and canvas (v2)

v2 adds `Layer`, `Compositor`, and `Canvas`.
A `Layer` is content (a string), an x, y, z position, an id, and child layers ([layer.go:11-28][lg-layer]).
`NewCompositor` flattens the tree to absolute positions, sorts by z, and `Draw` paints each overlapping layer in order ([layer.go:224-284][lg-compositor]).
`Hit(x, y)` returns the topmost layer with an id at a point ([layer.go:286-303][lg-hit]).
`Layer.Draw` calls `uv.NewStyledString(content).Draw(...)`, so the string is parsed into cells on every draw ([layer.go:152-156][lg-layer-draw]).
`Canvas` wraps `uv.ScreenBuffer`, sets the grapheme width method, and `Render` trims trailing spaces ([canvas.go:17-29][lg-canvas], [canvas.go:85-89][lg-canvas-render]).
These types come from `charmbracelet/ultraviolet`, which is outside the packages surveyed and which I did not read.

Two details in `layer.go`.
The sort uses each layer's own `z` field, while the doc says z is "relative to its parent" ([layer.go:68-72][lg-layer-z], [layer.go:230-233][lg-compositor]).
`AddLayers` panics on a nil layer ([layer.go:89-101][lg-layer-add]).

### 2.11 cellbuf

`cellbuf` is the cell-grid alternative to strings.
A `Cell` holds a `Style` (three colours, an 8-bit attribute mask, an underline style), a `Link`, a main rune, combining runes, and a width ([cell.go:16-35][cb-cell], [cell.go:138-182][cb-style]).
A `Line` is `[]*Cell`, a slice of pointers ([buffer.go:78-84][cb-line]).
`Screen` keeps a current and a new buffer, a per-line hash table, and cursor and pen state, and writes the difference ([screen.go:363-386][cb-screen]).
Before output it downsamples cell styles with `ConvertStyle(style, profile)` ([style.go:7-31][cb-style-convert]) and writes style changes as a diff against the pen ([screen.go:713-726][cb-update-pen]).
I read the type definitions and the pen update.
I did not read the scroll optimiser or the hash code.

## 3. Colour and terminal detection

### 3.1 colorprofile: how Detect decides

`Profile` is an ordered byte: `Unknown < NoTTY < ASCII < ANSI < ANSI256 < TrueColor` ([profile.go:10-26][cp-profile]).
The ordering lets `Detect` combine sources with `max`.

`Detect(writer, env)` works as follows ([env.go:18-54][cp-detect]).

1. `isatty` is true if `TTY_FORCE` is truthy or the writer is a terminal file.
2. `colorProfile(isatty, env)` ([env.go:74-111][cp-colorprofile]):
   - `TERM` unset (outside Windows) or `dumb`, or not a TTY: the base is `NoTTY`.
   - `NO_COLOR` truthy and a TTY: clamp to `ASCII` (bold and other attributes survive) and return.
   - `CLICOLOR_FORCE` truthy: at least `ANSI`, or the environment-derived profile if higher.
   - `CLICOLOR` truthy on a non-dumb TTY: at least `ANSI`.
3. The environment profile comes from `TERM`, `COLORTERM`, and a few other variables ([env.go:141-202][cp-envprofile]).
   - A `TERM` containing `alacritty`, `contour`, `foot`, `ghostty`, `kitty`, `rio`, `st`, or `wezterm` gives TrueColor.
     These are substring checks, so `st` matches any `TERM` that contains those two letters ([env.go:157-166][cp-term-substr]).
   - `tmux*` and `screen*` prefixes give at least ANSI256.
   - `xterm*` gives at least ANSI.
   - `WT_SESSION` or `GOOGLE_CLOUD_SHELL` gives TrueColor.
   - `COLORTERM` of `truecolor`, `24bit`, `yes`, or `true` gives TrueColor, except under `screen` or `tmux`.
   - A `256color` suffix gives ANSI256 and a `direct` suffix gives TrueColor.
4. If the result is not TrueColor and `NO_COLOR` is not set, and the writer is a TTY with a non-dumb `TERM`, the answer is `max(env, terminfo, tmux)` ([env.go:45-51][cp-detect]).
   - `Terminfo(TERM)` loads the entry with `xo/terminfo` and returns TrueColor if it has the `Tc` or `RGB` extended boolean, else ANSI ([env.go:204-229][cp-terminfo]).
   - `tmux` returns NoTTY when `$TMUX` is unset.
     Otherwise it runs the `tmux info` subprocess and looks for a `Tc` or `RGB` line containing "true", defaulting to ANSI256 ([env.go:231-262][cp-tmux]).

The doc comment says "`NO_COLOR` takes precedence over `CLICOLOR`/`CLICOLOR_FORCE`" ([env.go:29-30][cp-detect]).
The code applies `NO_COLOR` only when `isatty` is true ([env.go:86-91][cp-colorprofile]).
By my reading, with a pipe, `NO_COLOR=1`, and `CLICOLOR_FORCE=1`, the result is `ANSI`.
I did not run it.

### 3.2 Downsampling TrueColor to ANSI256 or ANSI

lipgloss v2 `Render` emits whatever colour was set.
A `color.Color` becomes `38;2;r;g;b` unless it is a basic or indexed colour type ([ansi/style.go:505-559][ansi-fgcolor]).
Conversion happens when bytes are written.

`colorprofile.Writer` is an `io.Writer` with a `Profile` ([writer.go:13-51][cp-writer]).

| Profile | What `Write` does |
|:--|:--|
| TrueColor | Passes bytes through |
| NoTTY or lower | Calls `ansi.Strip` and writes plain text |
| ASCII, ANSI, ANSI256 | Decodes each sequence and rewrites SGR |

The rewrite loop decodes sequences with `DecodeSequence`, rewrites only `CSI ... m`, and forwards everything else untouched.
In SGR, colour parameters (30-37, 38, 39, 40-47, 48, 49, 58, 59, 90-97, 100-107) are re-encoded through `Profile.Convert`.
Other parameters are copied.
For ASCII the colour parameters are dropped and attributes such as bold remain ([writer.go:54-80][cp-downsample], [writer.go:87-171][cp-handlesgr]).
The decoder state is a local variable per `Write` call, so a sequence split across two writes is not joined ([writer.go:54-62][cp-downsample]).

`Profile.Convert` keeps basic colours, maps indexed colours to the 16-colour set for ANSI, and sends other colours through `ansi.Convert256` or `ansi.Convert16`.
Results are cached in a package-level map behind a mutex ([profile.go:49-108][cp-convert]).

- `Convert256` maps RGB to the xterm 6x6x6 cube and to the 24-step grey ramp.
  It returns the one closer in HSLuv distance.
  The comment says it is ported from tmux's `colour.c` with that change ([color.go:174-247][ansi-convert256]).
- `Convert16` runs `Convert256` and looks up a fixed table ([color.go:249-265][ansi-convert16]).

### 3.3 Adaptive colours and background detection

**v1.**
`AdaptiveColor{Light, Dark string}` is resolved at render time by the style's renderer ([color.go:94-104][lg1-adaptive]).
The renderer queries the background lazily through `sync.Once` the first time something needs it, using termenv ([renderer.go:138-151][lg1-hasdark]).
The query is hidden inside the first `Render` that uses an adaptive colour.
Issue [#86][issue-lg-86] (2022, an early v0.x release) reports calls to `HasDarkBackground` taking seconds.

**v2.**
There is no renderer, and `Style` stores plain colours.
Detection is an explicit call, `HasDarkBackground(in, out)`.

- It writes `RequestBackgroundColor` (OSC 11) followed by `RequestPrimaryDeviceAttributes` (DA1) ([query.go:25-47][lg-query-bg]).
  A terminal that ignores OSC 11 still answers DA1, so DA1 ends the read.
  There is also a two-second timeout ([query.go:49][lg-query-timeout]).
- The reply is read with a cancelable reader and the response is decoded with `ansi.DecodeSequence` ([query.go:63-124][lg-query-terminal]).
- It puts the terminal in raw mode.
  On Unix both input and output must be terminals, and it tries each in turn ([terminal.go:12-26][lg-terminal-raw], [terminal.go:56-66][lg-terminal-unix]).
- On any error it returns `true` (dark) ([terminal.go:69-92][lg-terminal-hasdark]).
- "Dark" means HSL lightness below 0.5 ([color.go:225-233][lg-isdark]).

The caller then picks colours:

```go
hasDark := lipgloss.HasDarkBackground(os.Stdin, os.Stdout)
lightDark := lipgloss.LightDark(hasDark)
fg := lightDark(lipgloss.Color("#333333"), lipgloss.Color("#f1f1f1"))
```

([color.go:163-212][lg-lightdark]).
`Complete(profile)` picks among three explicit colours (ANSI, ANSI256, TrueColor) and returns "no colour" for other profiles ([color.go:235-277][lg-complete]).

**The compat package.**
`compat.AdaptiveColor` and `compat.CompleteColor` keep the v1 feel.
They read package-level variables that are initialised at import.
That initialiser performs the terminal query and the profile detection ([compat/color.go:11-17][lg-compat]).

### 3.4 Where v2 moved detection

Downsampling moved to the output layer.
The upgrade guide says: "In v1, color downsampling happened inside `Style.Render()` via the renderer. In v2, `Render()` always emits full-fidelity ANSI. Downsampling happens when you print." ([UPGRADE_GUIDE_V2.md:245-249][lg-upgrade-print]).

- `lipgloss.Writer` is a package variable: `colorprofile.NewWriter(os.Stdout, os.Environ())`, evaluated at package init ([writer.go:12-14][lg-writer]).
- `Fprint`, `Fprintln`, and `Fprintf` build a new `colorprofile.Writer` for the destination on every call ([writer.go:67-97][lg-writer-fprint]).
  `NewWriter` calls `Detect`, so each call can load terminfo and may start `tmux info` ([writer.go:23-28][cp-newwriter]).
- `Sprint*` reuse `Writer.Profile`, the profile for stdout ([writer.go:110-118][lg-writer-sprint]).
- Bubble Tea v2 is documented as doing this itself ([UPGRADE_GUIDE_V2.md:274-276][lg-upgrade-tea]).

### 3.5 The fang defect: a light palette without a TTY

fang chooses its colour scheme in `mustColorscheme`:

```go
func mustColorscheme(cs func(lipgloss.LightDarkFunc) ColorScheme) ColorScheme {
	var isDark bool
	if term.IsTerminal(os.Stdout.Fd()) {
		isDark = lipgloss.HasDarkBackground(os.Stdin, os.Stdout)
	}
	return cs(lipgloss.LightDark(isDark))
}
```

([theme.go:116-122][fg-mustcolorscheme]).
`isDark` starts as `false`.
When stdout is not a terminal, the query is skipped and the scheme is built with `LightDark(false)`, so every `c(light, dark)` slot takes its light value ([theme.go:44-63][fg-defaultscheme]).
lipgloss itself returns "dark" when it cannot detect ([terminal.go:85-92][lg-terminal-hasdark]).
So fang's no-TTY default is the opposite of the library's.

Three consequences, from reading the code:

1. The help writer is a `colorprofile.Writer`, which strips all ANSI for NoTTY output ([fang.go:130-133][fg-help-writer], [writer.go:42-44][cp-writer]).
   So the wrong palette is visible only when colour is forced (`CLICOLOR_FORCE`, `TTY_FORCE`) while stdout is not a terminal.
2. The error handler writes to stderr ([fang.go:173-177][fg-error-writer]) but the palette check looks at stdout.
   With stdout piped and stderr on a dark terminal, the styled error text takes the light palette.
3. When stdout is a TTY but stdin is not, `HasDarkBackground` fails and returns dark.
   That is the one case where the guess is dark.

A related open issue, [fang #104][issue-fang-104], says the OSC 11 plus DA1 probe runs on every help and error render when stdout is a TTY, even when the app passes `WithTheme`.
The reporter measured about 4 s per render in a pty that never answers.
I did not reproduce that.

## 4. glamour

**Pipeline.**
`NewTermRenderer` builds a goldmark parser with the GFM and definition-list extensions and auto heading ids ([glamour.go:67-97][gl-new]).
It registers one node renderer, `ANSIRenderer`, for every node kind at priority 1000 ([ansi/renderer.go:39-89][gl-register]).
The output is a string.
There is no detection of the terminal, the colour profile, or the width.
The default word wrap is 80 ([glamour.go:24-26][gl-new], [glamour.go:79-81][gl-new]).
The default style comes from `GLAMOUR_STYLE`, else `dark` ([glamour.go:279-286][gl-env]).
I searched the module for an auto-style option or any background query and found none.

**Block stack.**
Each block element has its own `bytes.Buffer`.
Children render into it.
When the block ends, `Finish` wraps the text with `lipgloss.Wrap` at the stack's width and writes it through a writer chain into the parent buffer ([renderer.go:91-137][gl-rendernode], [blockelement.go:31-66][gl-blockfinish], [paragraph.go:36-63][gl-paragraph-finish]).
The available width is `WordWrap - indent - 2*margin` ([blockstack.go:60-66][gl-blockstack-width]).

**Writer chain.**
`MarginWriter` wraps an `IndentWriter` over a `PaddingWriter`.
Each of them holds a `lipgloss.WrapWriter` that parses every byte.
The padding writer calls `ansi.StringWidth` on each line to compute padding ([margin.go:15-51][gl-margin], [margin.go:93-128][gl-padding], [margin.go:159-216][gl-indent]).
For one paragraph the text is scanned by `ansi.Wrap`, the `WrapWriter` inside `lipgloss.Wrap`, the indent writer's `WrapWriter`, the padding writer's `StringWidth`, and the padding writer's own `WrapWriter`.
That is five scans.
The count is from reading the call chain.
I did not measure it.

**Emitting ANSI.**
`renderText` builds an `ansi.Style` from the element's rules and writes `style.Styled(s)`, which is SGR, text, reset.
The code comment explains why it avoids lipgloss here:

> We're using `ansi.Style` instead of `lipgloss.Style` because Lip Gloss has a weird bug where it adds spaces when rendering joined strings. Needs further investigation.
> ([baseelement.go:39-88][gl-rendertext])

Padding and indent are emitted per cell.
`PaddingWriter` calls its `PadFunc` once per missing cell ([margin.go:101-105][gl-padding]).
The margin writer's `PadFunc` calls `renderText(w, rules, " ")` ([margin.go:35-37][gl-margin]).
When the block has any colour or attribute, each padded cell is its own escape pair.

`StylePrimitive` has `Faint` and `Conceal` fields that are parsed and cascaded ([ansi/style.go:38-58][gl-styleprimitive], [ansi/style.go:156-218][gl-cascade]).
`renderText` never reads them ([baseelement.go:39-88][gl-rendertext]).

**Style JSON themes.**
`StyleConfig` is a struct tree with a JSON tag on each field.
Optional values are pointers (`*string`, `*bool`, `*uint`) so "unset" can be told apart from zero ([ansi/style.go:96-138][gl-styleconfig]).
Colours are strings: an ANSI index such as `"252"` or a hex value.
A sample theme: [dark.json:1-12][gl-dark-json].
Built-in themes are Go values keyed by name in `DefaultStyles`, plus generated JSON files ([styles.go:28-37][gl-styles-names], [styles.go:676-687][gl-styles-map]).
`WithStylePath` tries a built-in name first and then a file path, while its doc comment says the reverse ([glamour.go:126-143][gl-stylepath]).
`cascadeStyle` lets a child override a parent when the child's pointer is non-nil.
Prefix and suffix fields cascade only for block-level merges ([ansi/style.go:140-252][gl-cascade]).

`format` strings use Go `text/template`.
The template is parsed on every call to `formatToken` ([baseelement.go:24-37][gl-format]).

**Code blocks.**
They go through chroma's `quick.Highlight` with the `terminal256` formatter.
A style named `charm` is registered from the JSON `chroma` block, guarded by a global mutex ([codeblock.go:14-23][gl-chroma-consts], [codeblock.go:84-126][gl-chroma-register], [codeblock.go:136-141][gl-chroma-run]).
`terminal256` output is 256-colour escapes, regardless of the profile.

**Tables.**
Each cell is rendered to an ANSI string, then given to `lipgloss/table` with `Width(blockWidth)`, `Wrap(...)`, and a `StyleFunc` that sets margin and alignment ([ansi/table.go:37-73][gl-table-render], [table.go:75-124][gl-table-style], [table.go:175-206][gl-table-cell]).
lipgloss measures and wraps those cell strings again.

## 5. fang

fang restyles cobra's help and errors.

**Setup.**
`Execute` sets silence flags, a version string, a custom help function, an optional hidden `man` command (via `mango-cobra` and `roff`), and optional signal handling.
It runs the root command and, on error, calls an `ErrorHandler` with a writer for stderr ([fang.go:109-179][fg-execute]).

**Help layout.**
`helpFn` ([help.go:42-108][fg-helpfn]):

1. Prints the long or short text with `Style.Width(width()).PaddingLeft(2)` ([help.go:152-158][fg-longshort]).
2. Prints "usage" and "examples" in a code-block style with a background colour.
   The block width is the widest example plus padding, limited to the terminal width.
   Examples wider than the block are cut with `ansi.Truncate(..., "…")`.
   The vertical padding is removed if the colour profile is ASCII or lower, or the block has no background.
3. Prints command groups and flags as two columns.
   The key column is the widest key plus two, with a minimum of 10, padded with spaces; keys are measured with `lipgloss.Width` ([help.go:447-466][fg-rendergroup]).

The terminal width is read once from stdout and capped at 120.
If stdout is not a terminal the width is 120 ([help.go:30-40][fg-width]).
A search for `Width(` in `help.go` finds it applied only to the long description and the code block, so flag and command descriptions are not wrapped to the width.
That is by reading.
I did not run it.

**Usage highlighting.**
`styleUsage` and the example styler split a command line on whitespace and classify tokens by simple rules: the program name, subcommands (resolved through cobra's `Traverse`), flags with or without `=value`, quoted strings, pipes, and redirects ([help.go:162-364][fg-usage]).
Each token is rendered with its own `Style` and the pieces are joined with `JoinHorizontal`.

**Command and flag tables.**
`evalFlags` and `evalCmds` build `map[string]string` keyed by the styled key, with a separate slice for order ([help.go:366-433][fg-eval]).

**Errors.**
`DefaultErrorHandler` prints the raw message if stderr is not a terminal.
Otherwise it prints a styled `ERROR` header, the message with its first word capitalised and wrapped to the terminal width, and, for a usage error, a hint to use `--help`.
Usage errors are found by string prefix on cobra's messages.
The code calls it a hack and links cobra PR 2266 ([help.go:110-150][fg-error]).

**Styles and schemes.**
`ColorScheme` has 16 `color.Color` slots, each filled from a `LightDarkFunc` ([theme.go:17-34][fg-scheme]).
`Styles` maps the scheme onto lipgloss styles.
The title style is `Bold(true).Foreground(...).Transform(strings.ToUpper).Padding(1, 0).Margin(0, 2)` ([theme.go:124-200][fg-styles]).
`DefaultColorScheme` uses the charmtone palette.
`AnsiColorScheme` uses the 16 ANSI colours only ([theme.go:44-83][fg-defaultscheme]).

## 6. Dependencies and size

Counts are the `require` entries in each `go.mod`, as written.
I did not resolve the transitive graph, because that needs the Go toolchain and the network.

| Module | Direct | Indirect | Total | Files |
|:--|--:|--:|--:|:--|
| lipgloss v2.0.6 | 10 | 8 | 18 | [go.mod][lg-gomod] |
| lipgloss v1.1.0 | 6 | 8 | 14 | [go.mod][lg1-gomod] |
| glamour v2.0.1 | 9 | 19 | 28 | [go.mod][gl-gomod] |
| colorprofile v0.4.3 | 5 | 4 | 9 | [go.mod][cp-gomod] |
| fang v2.0.1 | 13 | 18 | 31 | [go.mod][fg-gomod] |
| x/ansi v0.11.9 | 5 | 0 | 5 | [go.mod][ansi-gomod] |
| x/cellbuf v0.0.15 | 5 | 7 | 12 | [go.mod][cb-gomod] |

Notable edges:

- lipgloss v2 requires `ultraviolet`, `colorprofile`, `x/ansi`, `x/term`, `displaywidth`, `go-colorful`, `uniseg`, and `go-udiff`.
  `go-runewidth` is indirect ([go.mod][lg-gomod]).
- lipgloss v1 required `termenv` and `x/cellbuf` ([go.mod][lg1-gomod]).
- `x/ansi` requires `uax29`, `displaywidth`, `go-runewidth`, `go-colorful`, and `bitset` ([go.mod][ansi-gomod]).
- glamour adds `goldmark`, `goldmark-emoji`, `chroma`, `bluemonday`, and `x/text` ([go.mod][gl-gomod]).
- fang adds `cobra`, `pflag`, `mango-cobra`, `roff`, and `charmtone` ([go.mod][fg-gomod]).

Non-test line counts, top-level files of each package (from `wc -l`):

| Package | Lines |
|:--|--:|
| lipgloss root (v2) | 5,271 |
| lipgloss root (v1) | 3,773 |
| lipgloss/table | 1,254 |
| lipgloss/tree | 746 |
| lipgloss/list | 419 |
| lipgloss/compat | 98 |
| glamour root | 294 |
| glamour/ansi | 2,596 |
| glamour/styles | 1,116 |
| colorprofile | 625 |
| fang | 952 |
| x/ansi (top level) | 8,395 |
| x/ansi/parser | 488 |
| x/cellbuf | 4,036 |

The largest lipgloss files are `set.go` (932 lines), `get.go` (657), `style.go` (637), and `borders.go` (587).
Most of `x/ansi` is escape-sequence builders and mode constants.
The parts that matter for layout are `width.go`, `wcwidth.go`, `wrap.go`, `truncate.go`, `parser.go`, and `parser_decode.go`, about 1,900 lines together.

## 7. Design lessons for a Rust library

### 7.1 Strengths worth keeping

1. **A value-type style with an explicit "set" record.**
   Copying is cheap.
   Inheritance has a clear rule: fill only what the child left unset.
   In Rust, `Option<T>` fields give the same distinction without a bitmask.
   Keep the inherit rule, and decide up front which properties never inherit (lipgloss skips margins and padding).
2. **Styles as a function of position.**
   `StyleFunc(row, col)` for tables, and `(children, index)` for tree enumerators, keep data free of styling.
   A closure of the same shape works in Rust.
3. **String-in, string-out is easy to learn.**
   glamour hands rendered cells to a table.
   fang composes help from styled fragments.
   Any piece composes with any other.
   The cost is in 7.2.
4. **Full-fidelity output, downsample at the writer.**
   `Render` does not consult the environment, so tests are deterministic.
   The writer owns the policy.
5. **Detection as a function of explicit inputs.**
   `Detect(writer, env)` takes the environment as a parameter.
   `Profile` is an ordered type so signals combine with `max`.
6. **Table sizing that preserves short columns.**
   The shrink passes protect columns near their median, and a floor keeps one visible character.
   The tree and list share one recursive renderer.
7. **Stateless scans over a flat transition table.**
   Measuring and cutting need no parser object.
   Only SGR tracking needs the full parser.

### 7.2 Weaknesses visible in the code

1. **Repeated ANSI scans.**
   Width is recomputed in alignment, join, borders, the table resizer, tree padding, and fang's column widths.
   Wrap is two scans.
   glamour paragraphs take five.
   The table wraps each cell twice, once for height and once in `Render`.
   `applyMargins` measures the whole block even when both margins are zero ([style.go:566-570][lg-margins]).
   Issue [#721][issue-lg-721] reports exactly that and calls grapheme segmentation "the most expensive thing in the render path".
   That cost claim is the reporter's.
2. **Styles are lost at wrap points and need a repair pass.**
   `ansi.Wrap` does not carry the pen across the newline ([cellbuf/wrap.go:24-32][cb-wrap-todo]).
   lipgloss adds `WrapWriter` ([wrap.go:21-113][lg-wrapwriter]).
   A span-based representation (text plus style per run) wraps without re-parsing.
3. **Per-rune escapes for underline and strikethrough.**
   `underlineSpaces` is true whenever underline is set, whether or not `UnderlineSpaces` was called.
   So `useSpaceStyler` is true for any underlined or struck text ([style.go:310-318][lg-underline-flags]).
   The loop then writes `te.Styled(string(r))` for every rune ([style.go:428-436][lg-per-rune]).
   Each rune carries an SGR set and a full reset, at least 7 bytes of overhead per rune for a plain underline.
   That figure is arithmetic from the code.
   It also splits grapheme clusters across escape pairs, which I did not test on a terminal.
   v1 has the same logic ([style.go:276-284][lg1-underline-flags], [style.go:377-388][lg1-per-rune]).
   The doc on `Underline` says spaces are not underlined by default ([set.go:207-209][lg-underline-doc]).
   The doc on `UnderlineSpaces` says the default is true ([set.go:786-791][lg-underline-spaces-doc]).
   The code follows the second.
   An old issue, [#113][issue-lg-113], reported that the space options had no visible effect.
4. **Escape pairs per cell or per grapheme elsewhere.**
   glamour pads one cell per escape pair ([margin.go:101-105][gl-padding]).
   Gradient borders emit an SGR per grapheme ([borders.go:546-560][lg-border-blend]).
   Every `Styled` call writes a full reset, so there is no style diffing between neighbours.
   cellbuf's `Screen` does diff styles against the pen ([screen.go:713-726][cb-update-pen]).
5. **Several width libraries in one stack, plus a global setting.**
   `x/ansi` uses `uax29` and `displaywidth`, with a `go-runewidth` fallback for `WcWidth`.
   lipgloss borders use `uniseg` and `displaywidth`.
   cellbuf uses `go-runewidth` and `uniseg`.
   The East Asian setting is an environment variable read at init ([method.go:11-25][ansi-method]).
6. **Ambient and global state in detection.**
   The v1 renderer is a singleton with `sync.Once` ([renderer.go:12-29][lg1-renderer]).
   v2 still has `lipgloss.Writer` at package init ([writer.go:12-14][lg-writer]), the `compat` import-time query ([compat/color.go:11-17][lg-compat]), and `Fprint*` re-detecting on each call ([writer.go:67-97][lg-writer-fprint]).
   fang re-probes on each render ([#104][issue-fang-104]) and has the light default in 3.5.
7. **Detection heuristics.**
   The `st` substring ([env.go:157-166][cp-term-substr]).
   A subprocess (`tmux info`).
   The `NO_COLOR` and `CLICOLOR_FORCE` order differs from the doc comment (3.1).
   The writer's decoder state does not cross `Write` calls (3.2).
8. **A string is the only interchange type.**
   Tree values are strings, so a child table is flattened by `String()` ([tree.go:69-79][lg-tree-setvalue]).
   Table cells, fang's map keys, and glamour's cell text are all rendered strings.
   Widths and heights are bare `int`, with no unit type for cells versus bytes versus runes.
   The v1 and v2 `Width` semantics differ by the border (2.2).
9. **Panics and unbounded loops on odd input.**
   `AddLayers` panics on nil ([layer.go:91-94][lg-layer-add]).
   The parser has an "unreachable" panic ([parser.go:181-193][ansi-advance-utf8]).
   Open issue [#729][issue-lg-729] is titled "`Style.Width()/Height()` panic ... on extreme values" (title only).
   Table shrinking decrements by one cell per loop turn ([resizing.go:245-280][lg-resize-shrink]).
10. **Table details.**
    The median ignores each column's first row ([resizing.go:145-187][lg-resize-new]).
    The `StyleFunc` is called repeatedly per cell.
    Issues [#574][issue-lg-574] and [#764][issue-lg-764] concern border widths and cut-off lines (titles only).

### 7.3 What this suggests for a Rust design

These are design inferences, not findings.

1. Parse foreign ANSI once at the boundary into styled runs with cached cell widths.
   Wrap, truncate, pad, and join operate on runs.
   Serialise to escapes once, at the end.
2. Make the width method a value passed in (grapheme or wcwidth), not a global.
   Use one segmentation source.
   Use a newtype for cell counts.
3. Track the pen when serialising and emit only the SGR difference, as cellbuf's `Screen` does.
4. Keep the capability decision a pure function of `(is_tty flags, env map)` that returns an ordered enum.
   Keep terminfo lookup, subprocesses, and OSC 11 queries behind separate, explicit calls.
   Never guess "light" when no terminal answered.
5. Downsample in a writer that keeps decoder state across writes.
6. Keep `StyleFunc`-style closures for tables, lists, and trees.
   Keep tree and list values as typed nodes, not strings.
7. Cache column measurements once per table build and wrap each cell once.

## 8. What I could not verify

- Nothing was built or run.
  Every behaviour claim comes from reading source.
  No performance number here was measured.
  Scan counts are counts of calls in the code.
- The `x/ansi` v0.8.0 that lipgloss v1 pins, `termenv`, `ultraviolet`, `uniseg`, `displaywidth`, `uax29`, goldmark, and chroma were not read.
  Statements about layers, canvas, and `uv.ReadStyle` rest on the call sites only.
- The pinned dependency versions differ from the latest tags I read (section 0).
  Only `x/ansi` v0.11.8 versus v0.11.9 was diffed.
- Issues [#298][issue-lg-298], [#488][issue-lg-488], [#574][issue-lg-574], [#729][issue-lg-729], and [#764][issue-lg-764] were seen as titles only.
  I read the bodies of [#86][issue-lg-86], [#113][issue-lg-113], [#562][issue-lg-562], [#721][issue-lg-721], and [fang #104][issue-fang-104].
  Their claims are the reporters' and are not reproduced.
- The fang light-palette consequences in 3.5 are derived from the code.
  I did not run fang under a pipe.
- The `NO_COLOR` with `CLICOLOR_FORCE` result, the `UnsetUnderline` inheritance effect, the `cut` under `WcWidth` behaviour, and the nested-reset behaviour are readings of the source.
- Tree rendering calls `NodeChildren.Remove`, which uses `slices.Delete` on a slice that may share its backing array with the tree ([renderer.go:69-78][lg-tree-render], [children.go:23-30][lg-tree-remove]).
  Whether this changes the tree after a render was not tested.
- I did not read the lipgloss `examples/`, glamour's link, image, list, heading, and table-link elements, the glamour gallery, fang's example, the fang Windows file, or cellbuf's scroll and hash optimisers.
- No survey of Rust crates was done.

<!-- Link definitions -->

[lg-gomod]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/go.mod#L1-L29
[lg1-gomod]: https://github.com/charmbracelet/lipgloss/blob/f0e45475a64ee60d712b81145172d3739db36a93/go.mod#L1-L27
[gl-gomod]: https://github.com/charmbracelet/glamour/blob/95c93db04489cebf252ec856584445c7e85ee276/go.mod#L1-L37
[cp-gomod]: https://github.com/charmbracelet/colorprofile/blob/d085584efb48f2ad470e96cd0f3dcb8cc68a034b/go.mod#L1-L18
[fg-gomod]: https://github.com/charmbracelet/fang/blob/d89b30af5d6d3a6e3871631d0b62afe2e8c97739/go.mod#L1-L40
[ansi-gomod]: https://github.com/charmbracelet/x/blob/faa4adf95555331dfb439b716cc83767ecdadeb4/ansi/go.mod#L1-L11
[cb-gomod]: https://github.com/charmbracelet/x/blob/a1c614051099a0a065aaf1c8fa98f6110e492cbe/cellbuf/go.mod#L1-L21

[lg-style-props]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/style.go#L17-L109
[lg-style-struct]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/style.go#L141-L195
[lg-newstyle]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/style.go#L137-L139
[lg-bold]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/set.go#L194-L198
[lg-set]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/set.go#L9-L108
[lg-getasbool]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/get.go#L504-L509
[lg-inherit]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/style.go#L233-L265
[lg-unset]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/unset.go#L3-L6
[lg-unset-underline]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/unset.go#L20-L23
[lg-upgrade-renderer]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/UPGRADE_GUIDE_V2.md#L216-L243
[lg-upgrade-print]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/UPGRADE_GUIDE_V2.md#L245-L249
[lg-upgrade-tea]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/UPGRADE_GUIDE_V2.md#L274-L276
[lg1-style-struct]: https://github.com/charmbracelet/lipgloss/blob/f0e45475a64ee60d712b81145172d3739db36a93/style.go#L116-L121
[lg1-renderer]: https://github.com/charmbracelet/lipgloss/blob/f0e45475a64ee60d712b81145172d3739db36a93/renderer.go#L12-L29

[lg-render]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/style.go#L267-L526
[lg-render-prep]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/style.go#L397-L405
[lg-render-wrap]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/style.go#L407-L415
[lg-render-core]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/style.go#L417-L447
[lg-render-padding]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/style.go#L449-L478
[lg-render-align]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/style.go#L480-L498
[lg-render-border]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/style.go#L500-L503
[lg-render-max]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/style.go#L505-L523
[lg-margins]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/style.go#L543-L580
[lg-align]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/align.go#L12-L59
[lg1-render-wrap]: https://github.com/charmbracelet/lipgloss/blob/f0e45475a64ee60d712b81145172d3739db36a93/style.go#L365-L369
[lg-underline-flags]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/style.go#L310-L318
[lg-per-rune]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/style.go#L428-L436
[lg1-underline-flags]: https://github.com/charmbracelet/lipgloss/blob/f0e45475a64ee60d712b81145172d3739db36a93/style.go#L276-L284
[lg1-per-rune]: https://github.com/charmbracelet/lipgloss/blob/f0e45475a64ee60d712b81145172d3739db36a93/style.go#L377-L388
[lg-underline-doc]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/set.go#L207-L209
[lg-underline-spaces-doc]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/set.go#L786-L791

[lg-size]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/size.go#L9-L40
[lg-getlines]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/get.go#L630-L643
[lg-join-h]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/join.go#L28-L96
[lg-join-v]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/join.go#L116-L175
[lg-position]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/position.go#L19-L32
[lg-place]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/position.go#L36-L134
[lg-whitespace]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/whitespace.go#L24-L59
[lg-wrap]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/wrap.go#L11-L19
[lg-wrapwriter]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/wrap.go#L21-L113
[lg-wrapwriter-write]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/wrap.go#L72-L93

[lg-border-struct]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/borders.go#L14-L30
[lg-apply-border]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/borders.go#L327-L424
[lg-apply-border-render]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/borders.go#L431-L524
[lg-border-blend]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/borders.go#L541-L579

[lg-table-data]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/table/rows.go#L3-L13
[lg-table-stylefunc]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/table/table.go#L11-L37
[lg-table-style]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/table/table.go#L115-L121
[lg-table-row-cell]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/table/table.go#L508-L518
[lg-table-row]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/table/table.go#L484-L553
[lg-table-string]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/table/table.go#L356-L367
[lg-table-truncate]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/table/table.go#L555-L568
[lg-resize]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/table/resizing.go#L11-L108
[lg-resize-style]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/table/resizing.go#L84-L97
[lg-resize-new]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/table/resizing.go#L145-L187
[lg-resize-detect]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/table/resizing.go#L197-L200
[lg-resize-expand]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/table/resizing.go#L203-L231
[lg-resize-shrink]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/table/resizing.go#L241-L331
[lg-resize-heights]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/table/resizing.go#L334-L352
[lg-resize-content-height]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/table/resizing.go#L439-L448
[lg-resize-visible]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/table/resizing.go#L457-L512

[lg-list]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/list/list.go#L30-L55
[lg-list-enum]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/list/enumerator.go#L44-L128
[lg-tree-node]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/tree/tree.go#L35-L43
[lg-tree-setvalue]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/tree/tree.go#L69-L79
[lg-tree-render]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/tree/renderer.go#L47-L165
[lg-tree-enum]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/tree/enumerator.go#L15-L74
[lg-tree-remove]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/tree/children.go#L23-L30

[lg-layer]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/layer.go#L11-L28
[lg-layer-z]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/layer.go#L68-L72
[lg-layer-add]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/layer.go#L89-L101
[lg-layer-draw]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/layer.go#L152-L156
[lg-compositor]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/layer.go#L224-L284
[lg-hit]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/layer.go#L286-L303
[lg-canvas]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/canvas.go#L17-L29
[lg-canvas-render]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/canvas.go#L85-L89

[lg-query-bg]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/query.go#L25-L47
[lg-query-timeout]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/query.go#L49
[lg-query-terminal]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/query.go#L63-L124
[lg-terminal-raw]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/terminal.go#L12-L26
[lg-terminal-unix]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/terminal.go#L56-L66
[lg-terminal-hasdark]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/terminal.go#L69-L92
[lg-isdark]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/color.go#L225-L233
[lg-lightdark]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/color.go#L163-L212
[lg-complete]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/color.go#L235-L277
[lg-compat]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/compat/color.go#L11-L17
[lg-writer]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/writer.go#L12-L14
[lg-writer-fprint]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/writer.go#L67-L97
[lg-writer-sprint]: https://github.com/charmbracelet/lipgloss/blob/733ce53541bb688fd735524b665e0d96e35433bb/writer.go#L110-L118
[lg1-adaptive]: https://github.com/charmbracelet/lipgloss/blob/f0e45475a64ee60d712b81145172d3739db36a93/color.go#L94-L104
[lg1-hasdark]: https://github.com/charmbracelet/lipgloss/blob/f0e45475a64ee60d712b81145172d3739db36a93/renderer.go#L138-L151

[ansi-width]: https://github.com/charmbracelet/x/blob/faa4adf95555331dfb439b716cc83767ecdadeb4/ansi/width.go#L11-L119
[ansi-cluster]: https://github.com/charmbracelet/x/blob/faa4adf95555331dfb439b716cc83767ecdadeb4/ansi/parser_decode.go#L433-L451
[ansi-wcwidth]: https://github.com/charmbracelet/x/blob/faa4adf95555331dfb439b716cc83767ecdadeb4/ansi/wcwidth.go#L9-L71
[ansi-wcwidth-why]: https://github.com/charmbracelet/x/blob/faa4adf95555331dfb439b716cc83767ecdadeb4/ansi/wcwidth.go#L73-L99
[ansi-method]: https://github.com/charmbracelet/x/blob/faa4adf95555331dfb439b716cc83767ecdadeb4/ansi/method.go#L11-L35
[ansi-bytegraph]: https://github.com/charmbracelet/x/blob/faa4adf95555331dfb439b716cc83767ecdadeb4/ansi/truncate.go#L267-L290
[ansi-states]: https://github.com/charmbracelet/x/blob/faa4adf95555331dfb439b716cc83767ecdadeb4/ansi/parser/const.go#L37-L60
[ansi-table-doc]: https://github.com/charmbracelet/x/blob/faa4adf95555331dfb439b716cc83767ecdadeb4/ansi/parser/transition_table.go#L75-L80
[ansi-table]: https://github.com/charmbracelet/x/blob/faa4adf95555331dfb439b716cc83767ecdadeb4/ansi/parser/transition_table.go#L1-L64
[ansi-osc]: https://github.com/charmbracelet/x/blob/faa4adf95555331dfb439b716cc83767ecdadeb4/ansi/parser/transition_table.go#L259-L270
[ansi-parser]: https://github.com/charmbracelet/x/blob/faa4adf95555331dfb439b716cc83767ecdadeb4/ansi/parser.go#L19-L64
[ansi-advance]: https://github.com/charmbracelet/x/blob/faa4adf95555331dfb439b716cc83767ecdadeb4/ansi/parser.go#L158-L240
[ansi-advance-utf8]: https://github.com/charmbracelet/x/blob/faa4adf95555331dfb439b716cc83767ecdadeb4/ansi/parser.go#L181-L193
[ansi-handler]: https://github.com/charmbracelet/x/blob/faa4adf95555331dfb439b716cc83767ecdadeb4/ansi/parser_handler.go#L34-L60
[ansi-pool]: https://github.com/charmbracelet/x/blob/faa4adf95555331dfb439b716cc83767ecdadeb4/ansi/parser_sync.go#L9-L29
[ansi-decode]: https://github.com/charmbracelet/x/blob/faa4adf95555331dfb439b716cc83767ecdadeb4/ansi/method.go#L102-L148
[ansi-wrap]: https://github.com/charmbracelet/x/blob/faa4adf95555331dfb439b716cc83767ecdadeb4/ansi/wrap.go#L293-L463
[ansi-truncate]: https://github.com/charmbracelet/x/blob/faa4adf95555331dfb439b716cc83767ecdadeb4/ansi/truncate.go#L66-L166
[ansi-truncate-left]: https://github.com/charmbracelet/x/blob/faa4adf95555331dfb439b716cc83767ecdadeb4/ansi/truncate.go#L177-L265
[ansi-cut]: https://github.com/charmbracelet/x/blob/faa4adf95555331dfb439b716cc83767ecdadeb4/ansi/truncate.go#L30-L46
[ansi-style-styled]: https://github.com/charmbracelet/x/blob/faa4adf95555331dfb439b716cc83767ecdadeb4/ansi/style.go#L51-L58
[ansi-fgcolor]: https://github.com/charmbracelet/x/blob/faa4adf95555331dfb439b716cc83767ecdadeb4/ansi/style.go#L505-L559
[ansi-convert256]: https://github.com/charmbracelet/x/blob/faa4adf95555331dfb439b716cc83767ecdadeb4/ansi/color.go#L174-L247
[ansi-convert16]: https://github.com/charmbracelet/x/blob/faa4adf95555331dfb439b716cc83767ecdadeb4/ansi/color.go#L249-L265

[cb-wrap-todo]: https://github.com/charmbracelet/x/blob/a1c614051099a0a065aaf1c8fa98f6110e492cbe/cellbuf/wrap.go#L14-L32
[cb-pen]: https://github.com/charmbracelet/x/blob/a1c614051099a0a065aaf1c8fa98f6110e492cbe/cellbuf/pen.go#L9-L37
[cb-newcell]: https://github.com/charmbracelet/x/blob/a1c614051099a0a065aaf1c8fa98f6110e492cbe/cellbuf/buffer.go#L12-L76
[cb-line]: https://github.com/charmbracelet/x/blob/a1c614051099a0a065aaf1c8fa98f6110e492cbe/cellbuf/buffer.go#L78-L84
[cb-cell]: https://github.com/charmbracelet/x/blob/a1c614051099a0a065aaf1c8fa98f6110e492cbe/cellbuf/cell.go#L16-L35
[cb-style]: https://github.com/charmbracelet/x/blob/a1c614051099a0a065aaf1c8fa98f6110e492cbe/cellbuf/cell.go#L138-L182
[cb-screen]: https://github.com/charmbracelet/x/blob/a1c614051099a0a065aaf1c8fa98f6110e492cbe/cellbuf/screen.go#L363-L386
[cb-style-convert]: https://github.com/charmbracelet/x/blob/a1c614051099a0a065aaf1c8fa98f6110e492cbe/cellbuf/style.go#L7-L31
[cb-update-pen]: https://github.com/charmbracelet/x/blob/a1c614051099a0a065aaf1c8fa98f6110e492cbe/cellbuf/screen.go#L713-L726

[cp-profile]: https://github.com/charmbracelet/colorprofile/blob/d085584efb48f2ad470e96cd0f3dcb8cc68a034b/profile.go#L10-L26
[cp-detect]: https://github.com/charmbracelet/colorprofile/blob/d085584efb48f2ad470e96cd0f3dcb8cc68a034b/env.go#L18-L54
[cp-colorprofile]: https://github.com/charmbracelet/colorprofile/blob/d085584efb48f2ad470e96cd0f3dcb8cc68a034b/env.go#L74-L111
[cp-envprofile]: https://github.com/charmbracelet/colorprofile/blob/d085584efb48f2ad470e96cd0f3dcb8cc68a034b/env.go#L141-L202
[cp-term-substr]: https://github.com/charmbracelet/colorprofile/blob/d085584efb48f2ad470e96cd0f3dcb8cc68a034b/env.go#L157-L166
[cp-terminfo]: https://github.com/charmbracelet/colorprofile/blob/d085584efb48f2ad470e96cd0f3dcb8cc68a034b/env.go#L204-L229
[cp-tmux]: https://github.com/charmbracelet/colorprofile/blob/d085584efb48f2ad470e96cd0f3dcb8cc68a034b/env.go#L231-L262
[cp-writer]: https://github.com/charmbracelet/colorprofile/blob/d085584efb48f2ad470e96cd0f3dcb8cc68a034b/writer.go#L13-L51
[cp-newwriter]: https://github.com/charmbracelet/colorprofile/blob/d085584efb48f2ad470e96cd0f3dcb8cc68a034b/writer.go#L23-L28
[cp-downsample]: https://github.com/charmbracelet/colorprofile/blob/d085584efb48f2ad470e96cd0f3dcb8cc68a034b/writer.go#L54-L80
[cp-handlesgr]: https://github.com/charmbracelet/colorprofile/blob/d085584efb48f2ad470e96cd0f3dcb8cc68a034b/writer.go#L87-L171
[cp-convert]: https://github.com/charmbracelet/colorprofile/blob/d085584efb48f2ad470e96cd0f3dcb8cc68a034b/profile.go#L49-L108

[gl-new]: https://github.com/charmbracelet/glamour/blob/95c93db04489cebf252ec856584445c7e85ee276/glamour.go#L24-L97
[gl-env]: https://github.com/charmbracelet/glamour/blob/95c93db04489cebf252ec856584445c7e85ee276/glamour.go#L279-L286
[gl-stylepath]: https://github.com/charmbracelet/glamour/blob/95c93db04489cebf252ec856584445c7e85ee276/glamour.go#L126-L143
[gl-register]: https://github.com/charmbracelet/glamour/blob/95c93db04489cebf252ec856584445c7e85ee276/ansi/renderer.go#L39-L89
[gl-rendernode]: https://github.com/charmbracelet/glamour/blob/95c93db04489cebf252ec856584445c7e85ee276/ansi/renderer.go#L91-L137
[gl-blockfinish]: https://github.com/charmbracelet/glamour/blob/95c93db04489cebf252ec856584445c7e85ee276/ansi/blockelement.go#L31-L66
[gl-paragraph-finish]: https://github.com/charmbracelet/glamour/blob/95c93db04489cebf252ec856584445c7e85ee276/ansi/paragraph.go#L36-L63
[gl-blockstack-width]: https://github.com/charmbracelet/glamour/blob/95c93db04489cebf252ec856584445c7e85ee276/ansi/blockstack.go#L60-L66
[gl-margin]: https://github.com/charmbracelet/glamour/blob/95c93db04489cebf252ec856584445c7e85ee276/ansi/margin.go#L15-L51
[gl-padding]: https://github.com/charmbracelet/glamour/blob/95c93db04489cebf252ec856584445c7e85ee276/ansi/margin.go#L93-L128
[gl-indent]: https://github.com/charmbracelet/glamour/blob/95c93db04489cebf252ec856584445c7e85ee276/ansi/margin.go#L159-L216
[gl-rendertext]: https://github.com/charmbracelet/glamour/blob/95c93db04489cebf252ec856584445c7e85ee276/ansi/baseelement.go#L39-L88
[gl-format]: https://github.com/charmbracelet/glamour/blob/95c93db04489cebf252ec856584445c7e85ee276/ansi/baseelement.go#L24-L37
[gl-styleprimitive]: https://github.com/charmbracelet/glamour/blob/95c93db04489cebf252ec856584445c7e85ee276/ansi/style.go#L38-L58
[gl-styleconfig]: https://github.com/charmbracelet/glamour/blob/95c93db04489cebf252ec856584445c7e85ee276/ansi/style.go#L96-L138
[gl-cascade]: https://github.com/charmbracelet/glamour/blob/95c93db04489cebf252ec856584445c7e85ee276/ansi/style.go#L140-L252
[gl-dark-json]: https://github.com/charmbracelet/glamour/blob/95c93db04489cebf252ec856584445c7e85ee276/styles/dark.json#L1-L12
[gl-styles-names]: https://github.com/charmbracelet/glamour/blob/95c93db04489cebf252ec856584445c7e85ee276/styles/styles.go#L28-L37
[gl-styles-map]: https://github.com/charmbracelet/glamour/blob/95c93db04489cebf252ec856584445c7e85ee276/styles/styles.go#L676-L687
[gl-chroma-consts]: https://github.com/charmbracelet/glamour/blob/95c93db04489cebf252ec856584445c7e85ee276/ansi/codeblock.go#L14-L23
[gl-chroma-register]: https://github.com/charmbracelet/glamour/blob/95c93db04489cebf252ec856584445c7e85ee276/ansi/codeblock.go#L84-L126
[gl-chroma-run]: https://github.com/charmbracelet/glamour/blob/95c93db04489cebf252ec856584445c7e85ee276/ansi/codeblock.go#L136-L141
[gl-table-render]: https://github.com/charmbracelet/glamour/blob/95c93db04489cebf252ec856584445c7e85ee276/ansi/table.go#L37-L73
[gl-table-style]: https://github.com/charmbracelet/glamour/blob/95c93db04489cebf252ec856584445c7e85ee276/ansi/table.go#L75-L124
[gl-table-cell]: https://github.com/charmbracelet/glamour/blob/95c93db04489cebf252ec856584445c7e85ee276/ansi/table.go#L175-L206

[fg-execute]: https://github.com/charmbracelet/fang/blob/d89b30af5d6d3a6e3871631d0b62afe2e8c97739/fang.go#L109-L179
[fg-help-writer]: https://github.com/charmbracelet/fang/blob/d89b30af5d6d3a6e3871631d0b62afe2e8c97739/fang.go#L130-L133
[fg-error-writer]: https://github.com/charmbracelet/fang/blob/d89b30af5d6d3a6e3871631d0b62afe2e8c97739/fang.go#L173-L177
[fg-mustcolorscheme]: https://github.com/charmbracelet/fang/blob/d89b30af5d6d3a6e3871631d0b62afe2e8c97739/theme.go#L116-L122
[fg-defaultscheme]: https://github.com/charmbracelet/fang/blob/d89b30af5d6d3a6e3871631d0b62afe2e8c97739/theme.go#L44-L83
[fg-scheme]: https://github.com/charmbracelet/fang/blob/d89b30af5d6d3a6e3871631d0b62afe2e8c97739/theme.go#L17-L34
[fg-styles]: https://github.com/charmbracelet/fang/blob/d89b30af5d6d3a6e3871631d0b62afe2e8c97739/theme.go#L124-L200
[fg-width]: https://github.com/charmbracelet/fang/blob/d89b30af5d6d3a6e3871631d0b62afe2e8c97739/help.go#L30-L40
[fg-helpfn]: https://github.com/charmbracelet/fang/blob/d89b30af5d6d3a6e3871631d0b62afe2e8c97739/help.go#L42-L108
[fg-longshort]: https://github.com/charmbracelet/fang/blob/d89b30af5d6d3a6e3871631d0b62afe2e8c97739/help.go#L152-L158
[fg-error]: https://github.com/charmbracelet/fang/blob/d89b30af5d6d3a6e3871631d0b62afe2e8c97739/help.go#L110-L150
[fg-usage]: https://github.com/charmbracelet/fang/blob/d89b30af5d6d3a6e3871631d0b62afe2e8c97739/help.go#L162-L364
[fg-eval]: https://github.com/charmbracelet/fang/blob/d89b30af5d6d3a6e3871631d0b62afe2e8c97739/help.go#L366-L433
[fg-rendergroup]: https://github.com/charmbracelet/fang/blob/d89b30af5d6d3a6e3871631d0b62afe2e8c97739/help.go#L447-L466

[issue-lg-86]: https://github.com/charmbracelet/lipgloss/issues/86
[issue-lg-113]: https://github.com/charmbracelet/lipgloss/issues/113
[issue-lg-298]: https://github.com/charmbracelet/lipgloss/issues/298
[issue-lg-488]: https://github.com/charmbracelet/lipgloss/issues/488
[issue-lg-562]: https://github.com/charmbracelet/lipgloss/issues/562
[issue-lg-574]: https://github.com/charmbracelet/lipgloss/issues/574
[issue-lg-721]: https://github.com/charmbracelet/lipgloss/issues/721
[issue-lg-729]: https://github.com/charmbracelet/lipgloss/issues/729
[issue-lg-764]: https://github.com/charmbracelet/lipgloss/issues/764
[issue-fang-104]: https://github.com/charmbracelet/fang/issues/104
