---
ctl-core: patch
---

The `input` feature now builds on yamled 0.0.11, for its location index, with yamled's default features still off.
yamled 0.0.11 takes serde-saphyr from vsh-serde-saphyr 0.0.2, the release ctl-core already uses.
So a consumer that reads values through its own yamled 0.0.11 links one yamled and one serde-saphyr.
