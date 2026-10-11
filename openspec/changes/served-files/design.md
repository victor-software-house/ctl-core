# Design: served files

## Decisions

### 1. One template per call, discovery stays with the CLI

`render` takes one template's name, text, suffix, and context. Which files are
templates is policy: verctl reads the git index and refuses untracked
templates. A ctl-core walker was considered. It lost because it would either
copy verctl's git rules into every CLI or drop them.

### 2. The context is any serializable value

verctl passes a versions map and its own `Surface`. A ctl-core versions type
was considered. It lost because each CLI's templates see different names, and
`Value::from_serialize` already takes any of them.

### 3. Errors are a plain struct, not anyhow

ctl-core's library features return their own error types, as `input` does
with `InputError`. `ServeError` carries the template name and the message, so
a caller can add its own context.

### 4. The write re-checks only what the filesystem can change

The schema already refuses an absolute or climbing target, so `write` does not
check it again. It checks the one thing that can change after rendering: a
symlink at the target.

## Override and disable

Nothing is automatic. A CLI opts in with the `served` feature.
