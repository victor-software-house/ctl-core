---
ctl-core: patch
---

usage-lib is no longer a dependency.
ctl-core writes the Usage KDL itself, so the `usage` feature drops usage-lib and the 19 crates it alone pulled in.

- The spec keeps every byte: ctl-core already turns on Clap's `usage` feature and strips help itself.
- usage-lib had turned on Clap's default features for every consumer.
  ctl-core now turns on only Clap's `help`, which keeps the `-h`/`--help` flag ctl-core owns.
  A consumer that wants Clap's error context, suggestions, or colour turns them on in its own `clap` entry.
