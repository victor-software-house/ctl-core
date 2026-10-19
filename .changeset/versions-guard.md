---
ctl-core: patch
---

The shared `verify` runs `check:versions`, verctl's hand-edit guard, and `semver` runs `verctl semver`, which checks the API at the release type the pending fragments allow. Both run through verctl's served `ver` task, so a repository includes it at verctl 0.5.1 or later, the first that fetches the compared branches in a shallow pull request checkout, and its CI verify step sets `GITHUB_TOKEN` with `pull-requests: read`.
