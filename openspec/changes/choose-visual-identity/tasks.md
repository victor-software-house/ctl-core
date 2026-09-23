# Tasks

Compile, format, lint, and test tasks run on the build host with `mise run verify`.

## 1. Style options

- [x] 1.1 Add the record style, list style, and row separation to `RenderOptions`, and those plus the JSON layout to `View`; proof: unit tests per option
- [x] 1.2 Keep today's look as the default until section 3; proof: existing snapshots unchanged
- [x] 1.3 No Comfy Table type crosses the public surface; proof: `tests/architecture.rs` passes
- [ ] 1.4 Reach the options from `App`, and give each an environment override; proof: tests per override. Done with section 3, once the defaults are known

## 2. Gallery

- [x] 2.1 Add `examples/gallery.rs` rendering the fixed dataset under every candidate at any list of widths, coloured or colourless, including two consecutive records; proof: `cargo run --example gallery` on the build host
- [ ] 2.2 Show the gallery in a real terminal at several widths; proof: the operator reads every candidate with its code
- [ ] 2.3 Operator reviews the page and picks each axis; proof: the decisions table in `design.md` has no `pending` row

## 3. Defaults

- [ ] 3.1 Set the picked defaults; proof: updated snapshots show the new look
- [ ] 3.2 Raise `DEFAULT_COLUMN_BUFFER` to 2; proof: at `COLUMNS=119` no line exceeds 117
- [ ] 3.3 Add `RenderOptions::fallback_width` with its override and `None` disable path, leaving `minimum_automatic_width` unchanged; proof: piped `qctl show` fixture stays within 80 columns
- [ ] 3.4 Update `docs/presentation.md` and the automatic-width paragraph in `AGENTS.md` (buffer default 2, fallback width), and add a changeset; proof: `mise run verify` passes

## 4. Consumers

- [ ] 4.1 Bump the ctl-core pin in qctl, verctl, and forkctl, one pull request each; proof: each repo's `mise run verify` passes and its snapshots are reviewed
- [ ] 4.2 Close CTC-010's presentation acceptance items that this change proves; proof: `qctl check` passes on this repository
