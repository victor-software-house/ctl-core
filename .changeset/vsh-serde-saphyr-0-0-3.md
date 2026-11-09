---
ctl-core: patch
---

Move the `input` feature onto two new releases, with no change in behaviour:

- vsh-serde-saphyr `0.0.3`, which adds the fork's license attribution: a `NOTICE` file and copyright lines in `LICENSE-MIT`.
- yamled `0.0.12`, which only moves to that release.

A consumer that reads values through its own yamled should take `0.0.12` too.
It then still links one yamled and one serde-saphyr.
