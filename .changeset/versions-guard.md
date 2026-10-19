---
ctl-core: patch
---

The shared `verify` runs `check:versions`, verctl's hand-edit guard, and `semver` runs `verctl semver`, which checks the API at the release type the pending fragments allow. Both run through verctl's served `ver` task, so a repository includes it at verctl 0.5.0 or later, and its CI verify step sets `GITHUB_TOKEN` with `pull-requests: read`.
