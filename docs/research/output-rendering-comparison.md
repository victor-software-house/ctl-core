# Output rendering: ctl-core against typer/rich and cobra/fang/lipgloss

Measured 2026-09-17 on macOS arm64. Two throwaway CLIs with an identical
surface (`status`, `show ID`, `list --format`, `add`) were built to compare
against the shipped ctl binaries. Versions: clap via ctl-core 0.5.2
(qctl 0.4.2, verctl 0.2.1, forkctl 0.0.21), typer 0.27.2 + rich 15.0.0,
cobra 1.10.2 + fang 1.0.0 + lipgloss 1.1.0.

Colour was forced with `CLICOLOR_FORCE=1` and `FORCE_COLOR=1`, width pinned
with `COLUMNS=104`, buffer `CTL_CORE_COLUMN_BUFFER=2`. No pseudo-terminal was
involved, which is the condition every agent capture and CI log runs under.

## What ctl-core already does better than both

1. **JSON never carries ANSI.** The contract is stated and it holds. rich's
   `JSON` renderer is a *display widget*: it wraps at console width, so a
   coloured payload captured from it is no longer valid JSON. lipgloss has no
   JSON support, so that tint had to be hand-written.
2. **Width is honoured.** With `COLUMNS=104` the ctl tables land at 104 and
   wrap their own cells. lipgloss ignored `COLUMNS` entirely and rendered a
   panel 136 columns wide; width awareness is the caller's problem there.
3. **Colour policy is explicit and total.** `--color auto|always|never` plus
   `--no-color`, one flag, documented, and `--no-color` wins. fang offers no
   colour flag at all, and its palette is hardcoded truecolor.
4. **Did-you-mean on flags.** `--frobnicate` produces
   `tip: a similar argument exists: '--format'`. Neither of the others
   suggested anything.
5. **Identifiers survive.** rich's automatic highlighter tints digits inside
   strings, so `QCTL-014` renders as `QCTL-` plus a differently-coloured `014`.
   Avoiding it needs `highlight=False` or an explicit `Text`. ctl-core styles
   whole tokens.

## What the other two do better

1. **Borderless key-value.** Both alternatives render a record as right-aligned
   keys and left-aligned values with no box. ctl-core puts a full bordered
   table around two cells, which is the heaviest possible frame for the
   lightest possible content. This is the single biggest visual win available.
2. **A header rule instead of a full grid.** rich's `SIMPLE_HEAD` box draws one
   horizontal rule under the header and nothing else; lipgloss can hide every
   border. The ctl list keeps `┌─┬─┐`, `╞═╪═╡`, and `└─┴─┘` for what is a
   simple list of rows.
3. **Row separation is a parameter.** rich exposes `show_lines`; the operator's
   complaint about rows running together is a one-flag change there, and has no
   equivalent knob in the current ctl renderer.
4. **Examples belong to the command.** cobra's `Long` and typer's docstring
   carry an Examples block that appears in `show --help` without a second help
   surface. The ctl CLIs answer this with a separate `instructions` command.
5. **Completions come free.** cobra ships `completion bash|zsh|fish|powershell`
   and typer ships `--install-completion`. In the ctl family only forkctl has a
   `completion` verb; qctl and verctl both answer
   `error: unrecognized subcommand 'completion'`. That is an inconsistency
   inside one product family, not a framework gap.
6. **Pretty JSON at all.** `qctl --format json status` emits a single
   unbroken line, so reading it requires piping to `jq`. Neither alternative
   forces that.

## Defects found in the alternatives, for the record

- **fang guesses a light background with no tty.** Its help renders
  `48;2;241;239;239` (near-white) panels and hot-pink error badges, which is
  unreadable on a dark terminal in exactly the non-tty case that matters.
- **fang rewrites error text.** A message containing a newline and indentation
  came back flattened onto one line, capitalised, with a period appended.
- **lipgloss emits per-character escapes for underline.** An underlined path
  produced one full SGR pair per character, inflating the line enormously.
- **typer's `--show-completion` failed** here regardless of `SHELL`, answering
  `Shell  not supported.` Cause not established.

## Recommended changes to ctl-core, in order

1. **Add a borderless record primitive** and make it the default for a
   key-value document. Keep the bordered table for genuine grids.
2. **Add a header-rule table style** (one rule under the header, no verticals,
   no outer frame) and make it the default for a list of rows.
3. **Expose row separation** as a document-level option, defaulting off.
4. **Pretty-print `--format json`** behind the existing format flag, and keep
   the no-ANSI guarantee. A `--color always` JSON tint is optional and must
   stay opt-in, never automatic, because the plain path is the pipe path.
5. **Give every ctl CLI a `completion` verb.** forkctl already has one; copy it
   to qctl and verctl so the family is consistent.
6. **Carry examples in command help** rather than only in `instructions`.

Items 1 to 3 are presentation-layer additions in this crate and affect every
consumer at once. Item 4 touches the view layer. Items 5 and 6 are per-CLI.

## Reproducing

The two comparison CLIs are throwaway and were not kept. Rebuild from this
document's version pins: a typer app with `rich_markup_mode="rich"` and a
`Console(force_terminal=…)` resolving `--color` before `CLICOLOR_FORCE`,
`FORCE_COLOR`, then `NO_COLOR`; and a cobra root executed through
`fang.Execute` with `lipgloss` styles and `termenv.ANSI256` forced.
