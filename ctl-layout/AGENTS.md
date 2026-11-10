# ctl-layout

Terminal widgets built on `ctl-text`: `Table`, `Record`, `Definitions`, `Breakpoint`, `Rule`, `Stack`, and `Indent`.
This guide covers this crate only.
The repository's [`AGENTS.md`](../AGENTS.md) holds the workspace-wide rules.

## What it owns

- The layout rules of the widgets.
  They cover how a table chooses column widths and how a record and a rule fill a width.
  They also cover how a stack and an indent compose children, and how a breakpoint picks the widget for a width.
- The characters a frame is drawn with, for Unicode and for ASCII.

It depends only on `ctl-text` and on nothing else in this repository.

## What it must not own

- Text, cell widths, wrapping, and unsafe characters.
  A rule about them changes in `ctl-text`.
- Escape bytes.
  A widget returns lines of styled spans, and the encoder in `ctl-term` writes the escape sequences.
- Terminal facts.
  Whether a stream is a terminal, its colour depth, and its width come from `ctl-term`.
  A widget reads its width and box characters from the `Context` it is given.
- Colour and theme.
  A role becomes a style in ctl-core, before layout.

## Public API

- A public item is a promise.
  Add one only when a consumer needs it, and keep a helper `pub(crate)`.
- Every public item has a doc that adds something beyond its name.
  It states what the item is for, a unit, an invariant, an edge case, or what it returns.
  `missing_docs` is denied.
- Public enums are `#[non_exhaustive]`, public types implement `Debug`, and no method takes a `bool` where an enum reads better.
- Keep examples as doctests.
  The README's example runs as a doctest through `ReadmeDoctests` in `src/lib.rs`, so change the README and the crate docs together.
- A change that breaks the public API needs a `minor` changeset at the repository root.
  The crate is below 1.0, so a break bumps the minor version.
  `mise run semver` checks the change against the last release in CI.
  A change that adds an item or fixes behaviour needs a `patch` changeset.

## Tests

- `tests/it/` is one integration target, `it`, with `main.rs` as its root.
  `snapshots.rs` renders each widget, `frames.rs` and `scenarios.rs` hold targeted cases, and `properties.rs` holds the proptest properties over random text and widths.
  Shared fixtures live in `common.rs`.
- A unit test lives beside the code it tests, as in `src/widths.rs`.
- `tests/snapshots/` holds the insta file snapshots.
  Each is one plain-text file per widget and width, named `<case>_<width>.snap`, at widths 20, 40, 80, and 120.
  A widget draws no colour, so there is no colour variant.
- Add a widget case to `snapshots.rs` with its four widths in the same commit.

## Changing a snapshot

A snapshot is the review evidence for a layout change.

1. CI sets `INSTA_UPDATE=no`, so it never writes a snapshot: a changed render fails the test with a diff.
2. Accept a change with `cargo insta review` (`cargo-insta` is in `mise.dev.toml`), and commit the changed `.snap` files with the code that changed them.
3. Read the diff of every changed file before you accept it, at all four widths.
   A change you did not intend is a bug, not a snapshot to refresh.
4. Say in the pull request why each differing render is right.

## Decisions that bind it

These are in [`design.md`](../openspec/changes/archive/2026-11-10-terminal-render-engine/design.md) of the terminal render engine change.
Change the decision there before changing the code that follows it.

- [8.
  Every widget measures, then renders](../openspec/changes/archive/2026-11-10-terminal-render-engine/design.md#8-every-widget-measures-then-renders): a widget returns a `Measurement` and lines no wider than the width, except a word kept whole.
  The trait is dyn compatible, and a container holds `Box<dyn Renderable + 'a>`, so it can borrow its children.
- [9.
  The table chooses column widths in six steps](../openspec/changes/archive/2026-11-10-terminal-render-engine/design.md#9-the-table-chooses-column-widths-in-six-steps): the column algorithm, including the final narrowing of each column to its widest rendered line.
  `src/widths.rs` solves the widths, and `Table` documents the steps.
- [5.
  Escape sequences are written once, by the sink](../openspec/changes/archive/2026-11-10-terminal-render-engine/design.md#5-escape-sequences-are-written-once-by-the-sink): no escape byte exists before `ctl-term`'s encoder.
- [15.
  Public API](../openspec/changes/archive/2026-11-10-terminal-render-engine/design.md#15-public-api): the rules every public item follows.
- [17.
  Tests](../openspec/changes/archive/2026-11-10-terminal-render-engine/design.md#17-tests): snapshots at four widths, and the properties every widget keeps.
