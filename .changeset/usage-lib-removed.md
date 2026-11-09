---
ctl-core: patch
---

usage-lib is no longer a dependency.
ctl-core writes the Usage KDL itself, so the `usage` feature drops usage-lib and the 19 crates it alone pulled in.

- The spec keeps every byte: ctl-core already turns on Clap's `usage` feature and strips help itself.
- usage-lib had turned on Clap's default features for every consumer.
  Nothing in ctl-core turns them on now.
  A consumer that wants Clap's error context or suggestions turns them on in its own `clap` entry.
- ctl-core declares `-h`/`--help` and the `help` subcommand itself, so they work without Clap's `help` feature.
  A CLI that parses through `parser::apply_defaults` itself calls `help::try_emit` first, since the derive does not know the `help` subcommand.
  A plain `Cli::parse()` gets Clap's minimal output and no `help` subcommand.
- `Surface`'s Usage KDL keeps the `-h`/`--help` flag and the `help` subcommand on every command, as it did with Clap's `help` feature.
  It records a setting that turns help off only where the CLI declared one.
