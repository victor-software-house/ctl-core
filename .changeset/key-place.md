---
ctl-core: patch
---

`Input::place_key` returns where the last step of a `FieldPath` is written as a key, where `Input::place` returns where its value starts.
A fault about the key itself, such as a glob that is no glob, lands on the key and not on the first line of a block value.
An item of a list and the root have no key, so they give the place of their value.
