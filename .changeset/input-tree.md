---
ctl-core: patch!
---

`ctl_core::input` gains the located tree, the input format trait and its registry, beside the code that reads input today.
`input::tree` holds `Node`, `NodeKind`, `Scalar`, `ScalarStyle`, `ScalarSyntax` and `Tag`: each node keeps its byte span, each scalar its processed value, its style and, for TOML, the type its syntax gave it, and each mapping its key and value nodes in source order.
`input::format` holds the `Format` trait with its `read` and `resolve`, `Reading`, `Wanted`, `Resolved`, `Mismatch`, `Refusal` and `Fault`, and the two built-in formats `YAML` and `TOML`.
`input::Formats` is the registry of formats, found by extension through `Formats::of`, and `input::set_default_formats` sets the process default.

`Limits` gains three YAML-only fields, `max_total_replayed_events`, `max_replay_stack_depth` and `max_alias_expansions_per_anchor`, which set serde-saphyr's alias replay limits through the same tiers as the other limits, `.ctl/input.yaml` included.
`use_repo_limits` reports one a repository raises, and a text that goes past one fails with a problem naming the field and where to raise it.
A `Limits` literal that lists every field, without `..Limits::NONE`, no longer compiles; `Limits` documents building it with `..Limits::NONE`, which keeps compiling.
