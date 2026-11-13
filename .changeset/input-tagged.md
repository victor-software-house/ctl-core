---
ctl-core: patch
---

`ctl_core::input::Tagged` re-exports serde-saphyr's `Tagged<T>`, so a config field keeps its YAML tag beside its value: `styles: !override [Acme]` reads into `Tagged<T>` as `[Acme]` with the tag `!override`.
ctl-core now requires vsh-serde-saphyr 0.0.4, whose `Tagged<T>` gives an untagged enum the plain value of a tagged node instead of the map `{override: [Acme]}`.
