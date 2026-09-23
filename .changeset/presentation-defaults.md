---
ctl-core: patch
---

Make the operator's picks the defaults: borderless records with right-aligned keys, two-space JSON, a two-column automatic-width buffer, and an 80-column fallback width when none is detected (`fallback_width(None)` disables it). `App` now reaches the style options, JSON layout, and fallback width.
