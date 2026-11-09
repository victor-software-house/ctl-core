---
ctl-core: patch
"verctl:group": changed
---

The `input` feature reads YAML through vsh-serde-saphyr 0.0.3 and yamled 0.0.12.
In 0.6 it used serde-saphyr 2.0.0 and yamled 0.0.8.

vsh-serde-saphyr keeps the library name `serde_saphyr`.
It is serde-saphyr 2.1.1, the latest release, with two patches.
It carries a `NOTICE` file and the fork's copyright line in `LICENSE-MIT`.

- serde-saphyr 2.1.1 reports the same problems in the same words, and allocates less while it parses.
  The lock moves granit-parser, its YAML parser, from 2.0.0 to 2.1.0.
- Upstream serde-saphyr formats an error's whole report while it deserializes, snippet renderer included.
  So every binary that loaded input carried annotate-snippets.
  The patched release reads the error's kind instead, and builds an alias error's message from the plain text.
- ctl-core builds each problem's text from serde-saphyr's message formatter, and the reports keep their wording.
  The toy CLI's input tier drops 100,840 bytes of `.text` on linux-x64.
- yamled 0.0.12 gives the location index, with its default features still off, and takes serde-saphyr from the same release.

A consumer whose own yamled reads values takes yamled 0.0.12 too, so it links one yamled and one serde-saphyr.
With an older yamled it links both serde-saphyr crates.
