---
ctl-core: patch
---

The shared `check:versions` passes with nothing to compare in a repository without `.ctl/ver.yaml` (or the file `VERCTL_CONFIG` names), so a repository that declares no releases needs neither verctl's `ver` task nor verctl.
