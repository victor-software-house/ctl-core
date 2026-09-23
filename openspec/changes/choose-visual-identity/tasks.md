# Tasks

Compile, format, lint, and test tasks run on the build host with `mise run verify`.

## 1. Style options

- [x] 1.1 Add the record style, list style, and row separation to `RenderOptions`, and those plus the JSON layout to `View`; proof: unit tests per option
- [x] 1.2 Keep today's look as the default until section 3; proof: existing snapshots unchanged
- [x] 1.3 No Comfy Table type crosses the public surface; proof: `layout` and `style` stay private modules, and no public signature names an engine type
- [x] 1.4 Reach the options from `App`; proof: `app_styles_and_fallback_reach_every_view`
- [ ] 1.5 Give each style option an environment override; proof: tests per override

## 2. Gallery

- [x] 2.1 Add `examples/gallery.rs` rendering the fixed dataset under every candidate at any list of widths, coloured or colourless, including two consecutive records; proof: `cargo run --example gallery` on the build host
- [x] 2.2 Show the gallery in a real terminal at several widths; proof: the operator reads every candidate with its code
- [x] 2.3 Operator reviews the page and picks each axis; proof: the decisions table in `design.md` records each axis

## 3. Defaults

- [x] 3.0 Add the identifier role and preview it in the gallery; proof: `ids_are_bold_without_a_colour` passes
- [x] 3.1 Set the picked defaults; proof: updated snapshots show the new look
- [x] 3.2 Raise `DEFAULT_COLUMN_BUFFER` to 2; proof: `automatic_width_reserves_the_buffer_above_the_minimum` lays `COLUMNS=119` out at 117
- [x] 3.3 Add `RenderOptions::fallback_width` with its override and `None` disable path, leaving `minimum_automatic_width` unchanged; proof: `defaults_are_the_operator_picks` and `app_styles_and_fallback_reach_every_view`. The piped `qctl show` fixture that stays within 80 columns is proven with 4.1
- [x] 3.4 Update `docs/presentation.md` and the automatic-width paragraph in `AGENTS.md` (buffer default 2, fallback width), and add a changeset; proof: `mise run verify` passes

## 4. Consumers

- [x] 4.1 Bump the ctl-core pin in qctl, verctl, and forkctl, one pull request each; proof: each repo's `mise run verify` passes and its snapshots are reviewed
- [x] 4.2 Close CTC-010's presentation acceptance items that this change proves; proof: `qctl check` passes on this repository. CTC-010 stays open: its last acceptance item removes the `go` entry point, which still ships
