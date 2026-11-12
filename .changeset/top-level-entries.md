---
ctl-core: patch
---

`Input::top_level` lists the entries of a YAML or TOML text's root mapping or table in the order written.
Each `Entry` holds the key, the byte range of the value, a `Written` style and the string the parser reads, if any.
A caller that reads a mapping it did not write, such as the front matter of a Markdown file, needs no scanner of its own: serde-saphyr resolves anchors, aliases and tags, yamled gives the spans, and the `toml` tree holds the TOML ones.
A key that is not a string is left out, a root that is no mapping gives no entries, and a text past a limit gives the error `Input::parse` gives.
yamled is now 0.0.13.
