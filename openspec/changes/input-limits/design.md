# Design: input limits

## Decisions

### 1. Fixed limits, not per-tool options

The operator decided the limits apply to every tool. An options type on
`Input` was considered. It lost because no current tool needs a different
value, and a knob would let one tool drift from the others.

### 2. forkctl's values

2 MiB, depth 32, and 100,000 nodes are forkctl's current limits, chosen for
a hand-written manifest. qctl, verctl, and ctl-core read files of the same
kind, far under each limit.

### 3. The size check runs before parsing

serde-saphyr's `max_reader_input_bytes` covers reader input only, and `Input`
parses text already in memory, so `parse` checks the length itself.
`max_total_scalar_bytes` still caps scalar text inside the budget.

### 4. Anchors and aliases stay

forkctl refused them because it rewrote the whole file. Edits now go through
yamled, which keeps the file as written, so the ban protected nothing.
serde-saphyr's default alias-to-anchor ratio guard stays on against expansion
bombs.

## Override and disable

None. A tool that needs a larger file raises the limit here, for every tool.
