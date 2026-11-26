---
ctl-core: minor
---

`Input` reads every text into the located tree once, through its format, and keeps the tree.
`parse`, `place`, `place_key`, `check` and `top_level` all work on it, for YAML, TOML and any format a CLI adds.
A strict tree serves every one of them, and a listing tree, which `top_level` and placing read, serves only placing and listing.
yamled leaves ctl-core's graph: `input-yaml` links serde-saphyr only, and `input-toml` links the `toml` crate only.

`App::input_formats(Formats)` sets the formats `Input::open` reads, as `App::input_limits` sets the limits.
`Formats::open(path)` reads a file through a registry of the caller's, whatever the process default is.

Breaking changes:

1. `input::Format` is the format trait, and the enum is gone. `Format::Toml` becomes `&input::TOML`, and `Format::Yaml` becomes `&input::YAML`. A `match` on the enum has no replacement: ask the format for its `name()`.
2. `Format::of(path)` becomes `Formats::of(path)`, which returns the format as a `&'static dyn Format`. `Input::open` takes it from the process registry that `set_default_formats` or `App::input_formats` installs, else `Formats::builtin()`.
3. `Input::new_with_format` and `Input::format` take a `&'static dyn Format`, not the enum.
4. `input::Tagged` is ctl-input's own type, not serde-saphyr's, with the same spelling: `!override` for a local tag. A tag inside a value serde buffers, such as an untagged enum or a flattened field, is not passed on.
5. `Input` is compared by its format's name, and its `Debug` shows that name and no longer shows a parsed document.
6. A problem is worded by `Messages` whatever the format, so each problem type has one wording. Each changed one reads:
   - a repeated key: `duplicate mapping key: prefix, set DuplicateKeyPolicy in Options if acceptable` becomes `duplicate key \`prefix\``;
   - a merge key: `YAML merge keys are not allowed by configured policy` becomes `merge key \`<<\` is not allowed`;
   - a YAML 1.1 boolean: `invalid boolean (strict mode expects true/false)` becomes `expected true or false`;
   - a number field given a word: `invalid u16` in YAML, and `invalid type: string "…", expected u16` in TOML, become `expected an integer`;
   - an unknown field or variant: `unknown field \`x\`, expected one of name` becomes `unknown field \`x\`, expected one of \`name\``, with each name in backticks;
   - a problem in a node an alias replayed keeps `(defined at line L, column C)` after the new wording, and a limit a replay raised gains that suffix too;
   - a node budget breached inside a replay: `budget breached: Nodes { nodes: 10 }` becomes `exceeds max_nodes (9); raise limits.max_nodes in .ctl/input.yaml`.
7. Four positions move:
   - a field the root mapping lacks sits on line 1, column 1, where it sat on the last key of the root;
   - a field a nested mapping lacks sits on the key that holds the mapping, in YAML and TOML alike;
   - a YAML problem on line 1 of a file that starts with a byte-order mark sits one column further left, because the mark is no column;
   - a TOML text is read in the order it is written, so the first problem is the first in the file, where it was the first by key.

Migration: replace the enum with the built-in format values, call `Formats::builtin().of(path)` where `Format::of(path)` was called, and move each pinned message to the wording above.
A CLI with its own format implements `input::Format` and installs it with `Formats::builtin().add(&FORMAT)`, through `App::input_formats`, `set_default_formats`, or `Formats::open`.
