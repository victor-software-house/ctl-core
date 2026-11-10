# Three size cuts before 0.7.0

## Why

[Decision 16][d16] measured what each dependency adds to a release binary.
In qctl, four crates stand out:

| Crate | `__text` bytes | Brought in by |
|:--|--:|:--|
| kdl | 117,572 | usage-lib |
| usage | 108,540 | the `usage` feature |
| winnow 0.7 | 43,436 | kdl |
| annotate-snippets | 103,692 | the YAML loader |

The first three hold 269,548 bytes, 4.9% of qctl.

usage-lib also turns on clap's default features. So the narrow clap
features ctl-core declares save nothing in a CLI that mounts a mise task.
In rawr, which has no `usage`, turning the defaults off cut 60,808 bytes.
clap_builder was 48,808 of them. In qctl the same change cut 0.

ctl-core needs only the writing half of usage-lib. A clap command goes in,
and Usage KDL text comes out. ctl-core never parses a spec.

This change belongs to [CTC-041][ledger].

## What Changes

1. ctl-core writes the Usage KDL spec of a clap command itself, with no
   new dependency. The output matches [usage-lib 7.3.0][usage-730], the
   latest release, byte for byte. It departs only where 7.3.0's output is
   not valid [KDL v2][kdl-v2], or does not read back as written.
   [design.md](design.md), decision 3, lists those cases.
2. usage-lib leaves the workspace. kdl 6.7.1, with winnow 0.7, enters as
   a dev-dependency only, to parse the writer's output back in tests.
   `deny.toml` bans usage-lib, and keeps kdl and winnow 0.7 to that one
   path, each with a reason. winnow 1.x stays, because toml needs it.
3. With usage-lib gone, nothing in ctl-core's graph turns clap's default
   features on. The `features` spec holds that for `usage` and `input`.
4. The YAML loader stops reaching annotate-snippets, if and only if every
   problem report keeps its file, line, and message text. If the cut would
   change a report, the work stops and the finding is reported instead.
5. The size gates cover the cuts: `check:size` gains `usage` and `input`
   tiers, and every budget is set to the `.text` CI measures.
6. Decision 16 gains a measurement of qctl and rawr with the cuts.

The public API does not change. `usage::spec`, `mount_line`,
`mount_directives`, `spec_bin`, and `take` keep their signatures.
`surface-model` no longer inherits Serde through usage-lib.

## Capabilities

### New Capabilities

- `usage-spec`: the Usage KDL text ctl-core writes for a clap command.
  It names the reference output, the strings that must stay intact, and
  what mise must do with the spec.

### Modified Capabilities

- `features`: no ctl-core feature, and no dependency of ctl-core, turns on
  clap's default features. Today the requirement holds only for `cli`,
  `help`, and `app`.

## Impact

1. Code: `ctl-core/src/usage.rs` and a new private writer under it,
   `ctl-core/src/input.rs` and `ctl-core/src/input/limits.rs` where they
   render serde-saphyr errors, and `ctl-core/examples/size.rs`.
2. Manifests: the root `Cargo.toml` (`[workspace.dependencies]` and
   `[workspace.metadata.size]`), `ctl-core/Cargo.toml`, `deny.toml`,
   `Cargo.lock`, and `mise-tasks/check/tree.sh`.
3. Tests: a new corpus test with committed snapshots of `--usage-spec`
   output, and tests for hostile strings.
4. Docs: `ctl-core/AGENTS.md`, `docs/presentation.md`, the crate docs of
   the `usage` module, and [decision 16][d16].
5. Consumers: none has to change code. forkctl pins
   `usage-lib = "=6.1.1"` directly ([forkctl Cargo.toml][forkctl-pin]),
   so it keeps usage-lib until it drops that pin.

[d16]: ../2026-11-10-terminal-render-engine/design.md#16-feature-tiers-and-their-measured-sizes
[ledger]: ../../../../tasks.yaml
[usage-730]: https://docs.rs/crate/usage-lib/7.3.0
[kdl-v2]: https://github.com/kdl-org/kdl/blob/2.0.0/SPEC.md
[forkctl-pin]: https://github.com/victor-software-house/forkctl/blob/7b987070f9069842c2e9afdd17fca28353b81bc5/Cargo.toml#L96
