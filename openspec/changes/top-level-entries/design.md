# Design

## Decisions

1. **The types come from the format's parser, the spans from its index.**
   serde-saphyr already resolves anchors, aliases, tags and strict booleans for `Input::parse`, so a value is a string exactly when `parse` would read it as one.
   yamled gives the byte range and the style of each value, which serde-saphyr does not.
   The alternative was a visitor that rebuilds keys and spans from events.
   It lost, because yamled holds the spans for every other caller already.
2. **Entry listing lives in yamled.**
   `Node::entries` is a method of the index, so ctl-core adds no walk of its own.
   The alternative was a walk over `Path` segments in ctl-core, which cannot list the keys of a mapping it does not know.
3. **YAML reads the root once through `Input::parse`.**
   A `Root` type implements `Deserialize` with a visitor that keeps each entry with a string key and what its value reads as.
   The limits, the wording of every problem and the duplicate-key and merge-key refusals are those of `parse`, not a second copy.
4. **The yamled document is shared with `place`.**
   `yaml_text::document` is the helper both use, with the verdict kept under the limits it was computed under.
   Listing a text and placing a problem in it parse it once.
5. **TOML parses the tree again.**
   `Input::parse` checks the limits and the syntax first, with its usual messages, and the tree is read again for the spans.
   The alternative was to make the tree walk of `toml_text` public, which would touch the file the key placing change edits.
6. **`Written` has one name for each way a value is written.**
   The YAML plain, quoted, literal, folded and alias forms and the TOML strings and collections fit one enum, so a caller decodes a value by one `match`.
   A TOML basic string is `DoubleQuoted` and a literal string is `SingleQuoted`.

## Overrides

None of this reads configuration of its own.
The limits follow the tiers `Input::parse` follows, and a caller turns them off with `Input::limits`.
