# served-baseline

## REMOVED Requirements

### Requirement: The baseline renders five whole files

**Reason**: The shared Copier template renders these files.
**Migration**: Take them from the template (kind `rust-crate`).

### Requirement: Append-only entries reach their file with their reason

**Reason**: The template's answers (`deny_licenses`, `deny_bans`,
`deny_ignore`, `deny_skip`, `clippy_disallowed_methods`,
`clippy_disallowed_types`) carry the entries and their reasons.
**Migration**: Record each entry in `.copier-answers.yml` through
`copier update --data`.

### Requirement: A marked block is replaced and the rest of the file kept

**Reason**: Copier merges the template's part of `Cargo.toml` and
`mise.toml` on update, with no markers.
**Migration**: Remove the `# ctl-core baseline:` markers.

### Requirement: A repository can own a file

**Reason**: A repository keeps a file by deleting it once; Copier then
leaves it out of later updates.
**Migration**: None.

### Requirement: ctl-core matches its own baseline

**Reason**: ctl-core renders from the template like every other repository,
and `check:template` fails when a newer tag exists.
**Migration**: None.
