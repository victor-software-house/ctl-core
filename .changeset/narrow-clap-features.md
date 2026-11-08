---
ctl-core: patch
---

Ask Clap only for the features ctl-core calls. `clap` is now declared with `default-features = false`: `cli` turns on `std`, `derive`, `env`, and `string`, and `help` adds `usage`. A consumer that wants Clap's error context or suggestions turns them on in its own `clap` entry, for example `clap = { version = "4", default-features = false, features = ["std", "derive", "error-context", "suggestions"] }`; one that keeps Clap's default features sees no change. `App` now renders the help Clap stops for when a command misses a required argument or subcommand, on stderr with exit code 2, instead of printing Clap's own help. The new `verbatim` feature exports `verbatim::is_unsafe`, the predicate `Document::verbatim` filters with, so a CLI can drop the same terminal, bidi, and separator characters from text it prints. `unicode-general-category` is no longer a dependency.
