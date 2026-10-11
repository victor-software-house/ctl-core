---
ctl-core: patch
---

New `served` feature: `served::render(template, source, suffix, context)` renders a template that declares its own target through top-level `{%- set path / name / executable -%}` exports, checks those declarations once, and returns a `Served` file; `Served::write(root)` writes it with its directories and the 0755 or 0644 mode, refusing a symlinked target. Which templates a repository has and what context they see stay with the CLI.
