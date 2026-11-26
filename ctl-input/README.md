# ctl-input

Declared input for CLIs: parse a config file as YAML or TOML, validate it once, and report every problem on its file and line.

`ctl-input` is the one boundary where a CLI's config enters.
It rejects what the format's parser rejects, such as a duplicate key, and checks the rest with garde.
Each problem carries the line and column of the field it names, and downstream code keeps the validated type.

## What it provides

- `Input`, which reads a file or text in memory, in a format chosen by the extension or by the caller, and parses it into a serde type.
- `Input::check` and `Input::load`, which validate with garde and place each problem on its line.
- `FieldPath` and `Input::place`, which place a problem of the CLI's own the same way.
- `Limits`, the bounds on input size, depth, and node count, set by the call, the repository, or the process.
- `InputError` and `Problem`, one error that prints every problem as `file:line:column: field: message`.

## Example

```rust
use ctl_input::Input;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Config {
    name: String,
}

let config: Config = Input::new("toy.yaml", "name: release\n").parse().unwrap();
assert_eq!(config.name, "release");
```

## Features

| Feature | Adds |
|:--|:--|
| `yaml` | YAML, read by serde-saphyr and placed by yamled |
| `toml` | TOML, read and placed by the `toml` crate |
| `validate` | garde validation: `Input::check` and `Input::load` |

With no format feature the crate is empty.

## Documentation

The API reference is on [docs.rs](https://docs.rs/ctl-input).

## License

MIT.
