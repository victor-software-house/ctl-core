# ctl-text

Styled text for terminals: safe spans, cell widths, wrapping, and truncation.
This guide covers this crate only.
The repository's root [`AGENTS.md`](../AGENTS.md) holds the workspace rules, and they apply here too.

## What it owns

- `Span`, `Line`, and `Text`, and `overlay` for combining two `anstyle::Style` values.
- Cell widths per extended grapheme cluster: `text_width`, `cluster_width`, `prefix_len`, and `UNICODE_VERSIONS`.
- Wrapping and truncation: `Overflow`, `Line::fit`, and `Line::truncate`.
- The `Renderable` protocol with `Context` and `Measurement`.
- The `unsafe_chars` module, the one rule for which characters never reach a terminal, with `strip_unsafe` to apply it.

It depends on `anstyle`, `unicode-width`, `unicode-segmentation`, and `unicode-bidi`, and on nothing else in this repository.

## What it must not own

The crate does no terminal I/O.
It has no detection of a terminal, a colour depth, or a width.
It writes no escape byte and no byte to any stream.
Those belong to `ctl-term`.
Widgets such as tables and records belong to `ctl-layout`.
A change that needs one of these in `ctl-text` belongs in the other crate.

## Public API

A public item is a promise to every crate that depends on this one.

- Give every public item a doc comment that adds something beyond its name.
  Say what it is for, a unit, an invariant, an edge case, or what it returns.
- Keep `# Errors` and `# Panics` sections true.
- Mark a public enum `#[non_exhaustive]`, and derive `Debug` on a public type.
- Do not make a function take a `bool`; use an enum.
- A breaking change needs a minor changeset at the repository root, in `.changeset/`.
  The repository versions all crates together.
  Removing or renaming a public item, or changing what one returns, is a break.
- Change the docs, the tests, and the README example in the same commit as the behaviour.

`README.md` is the crates.io page.
Its example runs as a doctest through `ReadmeDoctests` in `src/lib.rs`, so keep it compiling.

## Tests

All tests live in `tests/it/`, one target built from `main.rs`:

| File | Covers |
|:--|:--|
| `styled_text.rs` | Spans, lines, text, wrapping, and truncation examples. |
| `protocol.rs` | The `Renderable` protocol and `Measurement`. |
| `properties.rs` | `proptest` properties over random text and widths. |
| `unicode_version.rs` | The pinned Unicode versions. |

The unit tests of the character rule sit in `src/unsafe_chars.rs`.
The crate has no snapshot files.

The properties generate text with wide, combining, emoji, and control characters, at widths from 1 to 200.
They build each text from several pushes in several styles, so segments and neighbouring styles vary.

They assert that lines fit and that no cluster is split.
They also assert that every non-space character appears once and in order, and that no line ends in a space.
The oracles list the break spaces and the line breaks, so they do not share the library's rule.

Proptest records a failing case in `tests/proptest-regressions/properties.txt`; commit that file with the fix.

`unicode_version.rs` fails when a dependency upgrade moves the Unicode version of the width or cluster tables.
Re-read the width and cluster tests, then update the pin in that file.

Run them alone with `cargo test --locked -p ctl-text`; `mise run verify` runs them with the rest of the workspace.

## Decisions that bind this crate

These sections of [`design.md`](../openspec/changes/archive/2026-11-10-terminal-render-engine/design.md) state the reasons; change the code and the decision together.

- [4.
  The style type is `anstyle::Style`](../openspec/changes/archive/2026-11-10-terminal-render-engine/design.md#4-the-style-type-is-anstylestyle)
- [6.
  Width is counted in terminal cells per grapheme cluster](../openspec/changes/archive/2026-11-10-terminal-render-engine/design.md#6-width-is-counted-in-terminal-cells-per-grapheme-cluster)
- [7.
  Text from outside the program is filtered when a span is built](../openspec/changes/archive/2026-11-10-terminal-render-engine/design.md#7-text-from-outside-the-program-is-filtered-when-a-span-is-built)
- [8.
  Every widget measures, then renders](../openspec/changes/archive/2026-11-10-terminal-render-engine/design.md#8-every-widget-measures-then-renders)
- [15.
  Public API](../openspec/changes/archive/2026-11-10-terminal-render-engine/design.md#15-public-api)
- [17.
  Tests](../openspec/changes/archive/2026-11-10-terminal-render-engine/design.md#17-tests)

Where the code and `design.md` differ, the code is current.
For example, `Span` carries no link.
