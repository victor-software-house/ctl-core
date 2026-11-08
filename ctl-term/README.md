# ctl-term

Terminal output for command-line programs.
`ctl-term` decides, once per output stream, whether to write colour, at what colour depth, and how wide the output is.
It then writes styled lines as the bytes a terminal reads.
The crate works on the plain lines `ctl-text` produces, so layout never sees an escape byte.

## What it provides

- [`Terminal`](https://docs.rs/ctl-term/latest/ctl_term/struct.Terminal.html) holds the decision for one stream.
  It follows `NO_COLOR`, `CLICOLOR`, `CLICOLOR_FORCE`, `FORCE_COLOR`, `COLORTERM`, `TERM`, and `COLUMNS`, in a fixed order.
- [`Encoder`](https://docs.rs/ctl-term/latest/ctl_term/struct.Encoder.html) is the one place an escape sequence is written.
  It converts every colour to the nearest one the depth can show.
- [`Console`](https://docs.rs/ctl-term/latest/ctl_term/struct.Console.html) renders any [`ctl-text`](https://docs.rs/ctl-text) widget at the stream's width and writes it.
  A reader that closes the pipe early, as `head` does, gives [`Error::BrokenPipe`](https://docs.rs/ctl-term/latest/ctl_term/enum.Error.html), so the caller can exit quietly.

## Example

A program calls `Console::from_process(Stream::Stdout, ColorMode::Auto)` and then `print`.
This example states the stream's facts instead, so it runs the same with or without a terminal.

```rust
use ctl_term::{ColorMode, Console, DetectOptions, Probe, Stream, Terminal};
use ctl_text::anstyle::AnsiColor;
use ctl_text::{Style, Text};

let terminal = Terminal::from_probe(
    Probe::Pipe,
    ColorMode::Always,
    DetectOptions::new().width(40),
    &|_name: &str| None,
);
let console = Console::new(Stream::Stdout, terminal);

let red = Style::new().fg_color(Some(AnsiColor::Red.into()));
let mut out = Vec::new();
console.write_to(&mut out, &Text::styled("failed", red))?;

assert_eq!(out, b"\x1b[31mfailed\x1b[0m\n");
# Ok::<(), ctl_term::Error>(())
```

## Features

`ctl-term` has no optional features.
On Windows it enables virtual-terminal mode through `anstyle-query` when the stream is a terminal.

## Documentation

The API reference, including the colour, width, and closed-pipe rules, is on [docs.rs](https://docs.rs/ctl-term).

## License

MIT.
