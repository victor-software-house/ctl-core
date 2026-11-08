---
ctl-core: patch
---

New crate on crates.io: `ctl-term`, terminal facts and the escape encoder.

- `Terminal::detect` decides, for one stream and one environment lookup, whether to write colour.
  It reads `NO_COLOR`, `CLICOLOR_FORCE`, `FORCE_COLOR`, `CLICOLOR` and `TERM=dumb`, then whether the stream is a terminal.
- It picks the colour depth from `COLORTERM`, `TERM` and `FORCE_COLOR`.
  It picks the width from an explicit width, then the stream's window, then `COLUMNS`, then a fallback of 80.
- `DetectOptions` and `Terminal::with_depth` override each default and can turn it off.
- `Encoder` turns `ctl_text::Line`s into bytes.
  It writes a style sequence only where the style changes, and leaves no style open at the end of a line.
  Each colour becomes the nearest one the depth can show.
  With colour off it writes the plain text byte for byte.
- `Console` renders any `Renderable` at the width of its stream and writes it.
  It reports a closed pipe as `Error::BrokenPipe`.
- The `detect` feature, on by default, holds what asks the operating system.
  Without it the crate holds only the encoder, the colour conversion, and `Terminal::from_probe`.
- The crate supports and tests Unix only; Windows is to come with a Windows CI job.
