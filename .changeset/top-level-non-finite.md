---
ctl-core: patch
---

`Input::top_level` reads a YAML `.inf`, `-.inf` or `.NaN` as a value that is no string.
It gave the error that `Input::parse` gives, so one such value hid every other entry of the text.
`Input::parse` is unchanged.
