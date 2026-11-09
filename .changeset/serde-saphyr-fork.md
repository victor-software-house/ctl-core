---
ctl-core: patch
---

The `input` feature reads YAML through vsh-serde-saphyr 0.0.2, under the same library name, `serde_saphyr`.
That release is serde-saphyr 2.1.1 with two patches.

Upstream serde-saphyr formats an error's whole report while it deserializes, snippet renderer included.
So every binary that loaded input carried annotate-snippets.
The patched release reads the error's kind instead, and builds an alias error's message from the plain text.

ctl-core now builds each problem's text from serde-saphyr's message formatter, and the reports keep their wording.
The toy CLI's input tier drops 100,840 bytes of `.text` on linux-x64.
A consumer whose own yamled reads values must take a yamled release on the same fork, or it links both serde-saphyr crates.
