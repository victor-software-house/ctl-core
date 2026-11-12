# Design

## Context

`place` answers with the value of the last step.
yamled's `Node::value` is where the value starts, and `Node::key` is where the key is written, for an entry of a mapping.
In the `toml` tree each entry of a table is a key span and a value span.

## Decisions

### 1. A separate method, not a flag on `place`

`place_key` has its own name, so a call reads as what it asks for.
Alternative: a `Place` that holds both the key and the value.
It lost because `Place` is built inside ctl-core for every call, the key is wanted by few, and a TOML text would need a second walk for each placing.

### 2. A step that has no key gives its value

A sequence item, an array item and the root are not entries of a mapping.
`place_key` gives their value rather than `None`, so a caller that places a mixed path gets a line.
A fault about one item of a list takes the index step and gets that item.

### 3. A missing path gives the key of its nearest ancestor

`place` gives the value of the last step that exists.
`place_key` gives the key of that step, so a fault about a key the text lacks lands on the entry that should hold it.
YAML walks the path back one step at a time until `Document::node` finds a node.

### 4. The limits and the document are shared

`places` already checks the size and the verdict of the limits in force, and builds the document once.
`place_key` goes through it, so a text the limits refuse gives `None` and no second parse happens.
