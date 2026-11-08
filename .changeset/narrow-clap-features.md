---
ctl-core: patch
---

Ask Clap only for the features ctl-core calls.
`clap` is now declared with `default-features = false`: `cli` turns on `std`, `derive`, `env`, and `string`, and `help` adds `usage`.

- A consumer that keeps Clap's default features sees no change.
- A consumer that wants Clap's error context or suggestions turns them on in its own `clap` entry: `clap = { version = "4", default-features = false, features = ["std", "derive", "error-context", "suggestions"] }`.
- When a command misses a required argument or subcommand, `App` renders the help Clap stops for.
  It writes that help on stderr with exit code 2, in place of Clap's own.
- The new `unsafe-chars` feature exports `unsafe_chars::is_unsafe` and `unsafe_chars::strip_unsafe`, which `Document::verbatim` filters with.
  A CLI can drop the same terminal, bidi, and separator characters from text it prints.
- `unicode-general-category` is no longer a dependency.
