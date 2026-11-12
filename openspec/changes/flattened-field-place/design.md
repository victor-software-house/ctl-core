# Design

## Context

`Input::check` turns each garde path into a `FieldPath` and places it with `places`, the same walk `Input::place` uses.  
The printed field is garde's own string.  
garde builds a path from Rust field names, so a `#[serde(flatten)]` field adds a step the file never writes.

## Decisions

### 1. The schema declares the flattened field

The caller names each flattened field once, as the `FieldPath` of the Rust field: `Input::flattened(FieldPath::root().key("assets").key("targets"))`.  
`check` drops the step at that position from every garde path whose steps start with it.  
Alternative: skip a step the file lacks when the step after it resolves.  
It lost because it guesses.

Take a struct field `b` that the file leaves out, so serde gives its default, whose field `c` fails a rule.  
If `a` also has a key `c` of its own, the problem would land on that key and print `a.c`, a field that holds no fault.

The schema author knows which fields are flattened, so the declaration costs one line and never guesses.

### 2. The rewrite happens before placing and printing

`check` rewrites the `FieldPath`, then places it and prints it.  
The walk of YAML and of TOML does not change, so both formats follow the same rule.  
`Input::place`, `place_key` and `problem_at` take a path the caller wrote in the file's terms, so the declaration does not touch them.

### 3. A path with no declared step prints garde's string

When `check` drops no step, the printed field is garde's string, as today.  
When it drops one, the printed field is the rewritten `FieldPath`'s `Display`.  
Alternative: always print the `FieldPath`.  
It lost because `FieldPath` keeps a non-numeric index as a key, so a rare path would print differently and the snapshot would change.

### 4. A declaration matches its steps exactly

A declared path matches a garde path whose leading steps are equal to it, indexes included.  
Each call adds one declaration, so a schema with two flattened fields calls `flattened` twice.  
A declaration that matches no garde path changes nothing.

A key with the flattened field's name, such as a target named `targets`, still works: only the step at the declared position is dropped.

## Non-Goals

- A flattened field under every item of a list, or under every entry of a map.  
  It needs a wildcard step.  
  `Step` is a public enum without `#[non_exhaustive]`, so a new variant breaks every exhaustive match on it, a breaking change.  
  No caller has such a field, so the wildcard waits for one.

Override and disable path: an input that declares no flattened field checks exactly as before.
