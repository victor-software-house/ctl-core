# Proposal: input limits

Queue row: CTC-008 (forkctl onto the shared reader).

## Why

forkctl guarded its manifest with its own serde-saphyr budget. It now reads
through `ctl_core::input`, which already applies serde-saphyr 1.3.0's default
budget (depth 64, 250,000 nodes, 1,024 documents, alias and anchor caps). No
tool or repository can change those values, and an in-memory text has no size
cap at all, because `max_reader_input_bytes` applies only to reader input. A
limit error also does not say how to raise the limit.

## What Changes

1. `input::Limits` has one optional override for each of the 21 fields of
   serde-saphyr's `Budget`. `None` leaves the field to the next tier.
2. Overrides come from four tiers, decided field by field, highest first: the
   call (`Input::limits`), the repository (`limits:` in `.ctl/input.yaml`,
   installed by `input::use_repo_limits(root)`), the process
   (`input::set_default_limits`, or `App::input_limits`), and serde-saphyr's
   default. A repository beats a tool's default, as an operator's environment
   beats a tool's built-in table width.
3. `max_reader_input_bytes` also caps text already in memory.
4. A limit error names the file and line, the limit and its value, and where
   to raise it: `exceeds max_depth (64); raise limits.max_depth in
   .ctl/input.yaml`.
5. The defaults stay serde-saphyr's. Nothing is tightened.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `declared-input`: limits are overridable in tiers, and a limit error says
  how to raise it.

## Impact

1. A patch release of ctl-core. Without overrides, a parse behaves as before,
   except that an in-memory text over 256 MiB is now refused.
2. forkctl reads its manifest through `Input::parse` with the defaults, and
   calls `use_repo_limits` at startup so a repository's overrides apply.
