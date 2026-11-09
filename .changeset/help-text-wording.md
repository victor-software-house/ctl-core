---
ctl-core: patch
---

The help text of the shared colour flags changes wording:

- `--color` reads "Pretty-output colour policy".
- `--no-color` reads "Force colourless pretty output".
- The `pretty` format reads "Human text, optionally coloured".
- The description of the `auto` colour value, which shell completion shows, names what it checks: whether the stream is a terminal and the environment allows colour.

A CLI that compares its help with a snapshot or a committed file updates that file when it takes this release.
