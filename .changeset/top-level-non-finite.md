---
ctl-core: patch
---

`Input::top_level` refuses nothing that YAML allows.
A repeated key, a merge key (`<<`) and a `.inf`, `-.inf` or `.NaN` are entries, where each gave the error that `Input::parse` gives and hid every other entry of the text.
A key that is a number, a boolean or a null is listed by its text as written, and a sequence, mapping or alias key is still left out.
`Input::parse` is unchanged.
