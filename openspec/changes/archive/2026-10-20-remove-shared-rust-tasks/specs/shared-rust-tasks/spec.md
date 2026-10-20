## REMOVED Requirements

### Requirement: ctl-core ships the standard Rust tasks as one TOML task file

**Reason**: The catalog moved to the public actions repository, and no repository includes ctl-core's copy.
**Migration**: Include `tasks/rust.toml` from the actions repository by commit, as the template renders it.

### Requirement: verify depends on the repository's own check:features

**Reason**: The actions repository's catalog carries the same dependency, and ctl-core no longer ships a catalog.
**Migration**: Include the actions repository's `tasks/rust.toml`; its `verify` composition is the including repository's `mise.toml`.

### Requirement: verify depends on the repository's own check:repo

**Reason**: The actions repository's catalog carries the same dependency, and ctl-core no longer ships a catalog.
**Migration**: Include the actions repository's `tasks/rust.toml`; its `verify` composition is the including repository's `mise.toml`.
