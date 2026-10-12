---
ctl-core: patch
---

Remove the public `baseline` module and the `baseline` feature. The shared Copier template now renders the shared Rust configuration, so ctl-core keeps no second generator. No consumer enables the feature: verctl, qctl, and forkctl pin earlier releases without it. A crate that enabled `ctl-core/baseline` takes the files from the template instead. The removal is a breaking API change released as a patch by operator decision, and CI's semver check allows it only against the 0.6.14 baseline.
