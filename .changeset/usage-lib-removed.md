---
ctl-core: patch
---

usage-lib is no longer a dependency.
ctl-core writes the Usage KDL itself, so the `usage` feature drops usage-lib and the 19 crates it alone pulled in.

- The spec keeps every byte: ctl-core already turns on Clap's `usage` feature and strips help itself.
- usage-lib had turned on Clap's default features for every consumer.
  Nothing in ctl-core turns them on now.
  A consumer that wants Clap's error context or suggestions turns them on in its own `clap` entry.
