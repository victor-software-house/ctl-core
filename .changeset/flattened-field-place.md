---
ctl-core: patch
---

`Input::flattened` declares a schema field that `#[serde(flatten)]` puts directly under its parent in the file.
`Input::check` drops that field's step from each garde path, so a problem prints as the file holds it, such as `assets.linux-x64.files`, and sits on that line.
A path through no declared field prints and sits as before.
