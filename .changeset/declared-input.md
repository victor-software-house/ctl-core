---
ctl-core: patch
---

Add the declared-input layer. The `validate` feature carries the garde rules every schema shares; the `input` feature reads a file, parses it with serde-saphyr (strict booleans, duplicate and merge keys refused), validates it once with garde, and reports every problem with its file and line, placed through yamled. `Input::frontmatter` splits YAML frontmatter from a Markdown body and keeps the file's line numbers.
