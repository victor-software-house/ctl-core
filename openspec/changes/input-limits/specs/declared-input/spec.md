## ADDED Requirements

### Requirement: The reader enforces safety limits for every tool

`Input::parse` SHALL refuse a text longer than `MAX_BYTES` (2 MiB) before
parsing, with a problem that has no line. It SHALL refuse a text with more than
one document, sequences and mappings nested deeper than `MAX_DEPTH` (32), more
than `MAX_NODES` (100,000) nodes, or more than 2 MiB of scalar text. Anchors and
aliases SHALL stay readable.

#### Scenario: A text over the size limit

- **WHEN** a tool parses `big.yaml` holding `a: ` followed by 2 MiB of `x`
- **THEN** the error has one problem with no line, naming the 2 MiB limit

#### Scenario: Two documents

- **WHEN** a tool parses a text with `a: 1`, `---`, and `b: 2`
- **THEN** the parse fails

#### Scenario: Nesting past the limit

- **WHEN** a tool parses 33 nested mappings
- **THEN** the parse fails, and 31 nested mappings parse

#### Scenario: Too many nodes

- **WHEN** a tool parses a flow sequence of 100,000 scalars
- **THEN** the parse fails, and a sequence of 1,000 scalars parses

#### Scenario: An alias

- **WHEN** a tool parses `base: &base {flag: true}` and `copy: *base`
- **THEN** `copy.flag` reads as true
