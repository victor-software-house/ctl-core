---
ctl-core: patch
---

Fix `--usage-spec` for help text that holds four quotes and a hash, `""""#`.
The spec wrote such text as a raw string opened with one hash.
The `"""#` inside it closed the string early, so mise could not read the spec.
The string now opens with one hash more than any closing sequence the text holds.

ctl-core depends on usage-lib 7.3.0.
It pulls in none of `kdl`, `miette`, `nom`, `strum`, `versions`, and `winnow` 0.7.
Every other spec ctl-core writes keeps its bytes.
