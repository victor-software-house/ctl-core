# Rust crates for terminal styling, capability detection and progress: a source survey

This survey reads the published source of each crate at its latest stable version.
It serves the design of a rich-like rendering library for a family of clap command-line tools.
Nothing was compiled or run.
Every claim cites a line range in the source at that exact version.

## 1. Answers first

1. Use `anstyle` for the style type and `anstream` for the output adapter.
   clap already uses both, so a library built on them shares types with `clap::builder::styling`.
2. `AutoStream` does not need its own detection.
   `AutoStream::new(writer, choice)` with `Always`, `AlwaysAnsi` or `Never` reads no environment variable.
   Only `Auto` reads the environment.
3. Width of stdout and stderr needs only `terminal_size::terminal_size_of(std::io::stderr())`.
   It costs `rustix` with the `termios` feature on Unix and `windows-sys` on Windows.
   It does not read `COLUMNS`.
4. None of the surveyed crates converts RGB to 256 colors or 256 colors to 16.
   The library must write that conversion, or choose to emit only what `COLORTERM` allows.
5. `indicatif` depends on `console`, not on `anstream`.
   It redraws by moving the cursor up, clearing each line, and rewriting, at 20 Hz by default.

## 2. Versions read

All versions are the latest stable on crates.io on 2026-11-07.
Release dates are UTC.
Download figures come from the crates.io API: total, then the last 90 days.

| Crate | Version | Released | Total downloads | Last 90 days |
|:--|:--|:--|--:|--:|
| anstyle | 1.0.14 | 2026-03-13 | 892 M | 240 M |
| anstream | 1.0.0 | 2026-02-11 | 823 M | 220 M |
| anstyle-parse | 1.0.0 | 2026-02-11 | 808 M | 218 M |
| anstyle-query | 1.1.5 | 2025-11-13 | 787 M | 209 M |
| anstyle-wincon | 3.0.11 | 2025-11-13 | 395 M | 106 M |
| colorchoice | 1.0.5 | 2026-03-13 | 788 M | 212 M |
| owo-colors | 4.4.0 | 2026-08-27 | 194 M | 45 M |
| yansi | 1.0.1 | 2024-03-13 | 301 M | 67 M |
| nu-ansi-term | 0.50.3 | 2025-10-10 | 636 M | 169 M |
| console | 0.16.6 | 2026-09-10 | 412 M | 99 M |
| termcolor | 1.4.1 | 2024-01-10 | 574 M | 94 M |
| supports-color | 3.0.2 | 2024-11-26 | 79 M | 21 M |
| supports-hyperlinks | 3.2.0 | 2025-12-17 | 48 M | 12 M |
| terminal_size | 0.4.4 | 2026-03-23 | 238 M | 51 M |
| crossterm | 0.29.0 | 2025-04-05 | 229 M | 57 M |
| is-terminal | 0.4.17 | 2025-10-23 | 382 M | 70 M |
| indicatif | 0.18.6 | 2026-07-01 | 252 M | 56 M |
| clap_builder | 4.6.7 | 2026-09-14 | 910 M | 247 M |
| rustix | 1.1.6 | 2026-10-27 | 1.37 G | 349 M |

Also read, because they bear on the design: `anstyle-hyperlink` 1.0.2, `anstyle-progress` 0.1.6, and the manifest of `cargo` 0.100.0.

## 3. The three questions

### 3.1 What `AutoStream` detects, and whether a library can drive it

`AutoStream<S>` wraps a writer `S` and picks one of three inner modes: pass-through, strip, or (Windows only) console translation.
[The enum is at auto.rs L23-29][anstream-auto-enum].

`AutoStream::new(raw, choice)` is a plain match on the choice.
[auto.rs L62-72][anstream-auto-new].

| Choice | Result | Reads the environment? |
|:--|:--|:--|
| `Auto` | calls `AutoStream::auto`, which runs the detection below | yes |
| `AlwaysAnsi` | pass-through | no (Windows: enables VT mode if the stream is a terminal) |
| `Always` | pass-through on Unix; on Windows may pick console translation | no on Unix; `TERM` on Windows |
| `Never` | strip adapter | no |

[always_ansi, always and never are at auto.rs L92-133][anstream-auto-always].

The detection for `Auto` is one function.
[auto.rs L197-223][anstream-auto-choice].
In order:

1. `ColorChoice::global()`, an atomic set by `ColorChoice::write_global`. Anything but `Auto` wins. [colorchoice L31-59][colorchoice-global]
2. `NO_COLOR` non-empty: `Never`.
3. `CLICOLOR_FORCE` non-empty: `Always`.
4. `CLICOLOR` equal to `0`: `Never`.
5. The raw stream is a terminal, and (`TERM` is set and not `dumb`, or `CLICOLOR` is set and not `0`, or `CI` is set): `Always`.
6. Otherwise `Never`.

The helpers read the variables with `std::env::var_os`.
[anstyle-query lib.rs L23-135][anstyle-query-env].

What it does not do:

- It does not read `FORCE_COLOR`, `COLORTERM`, `TERM_PROGRAM` or `WT_SESSION`.
- It has no notion of color depth. `anstyle_query::truecolor()` exists but `AutoStream` never calls it. [lib.rs L115-119][anstyle-query-env]
- It checks `is_terminal` on the wrapped stream, not on stdout. A stderr stream is judged by stderr.
- It reads the environment again on every `AutoStream::auto` call. There is no cache.
  The `print!` family builds a fresh `anstream::stdout()` on each call. [_macros.rs L61-75][anstream-macros]

Can a library drive it with its own choice and writer?
Yes, with one limit.

- The choice is explicit. The crate's own docs show a custom `choice` function that skips `NO_COLOR`, `CLICOLOR_FORCE` and `CI`, then calls `AutoStream::new`. [auto.rs L35-60][anstream-auto-doc]
  So a library can run its own detection once, hold a `ColorChoice`, and pass `Always` or `Never` every time.
- The writer must implement `RawStream`, a sealed trait. [stream.rs L5-30][anstream-rawstream]
  Implementors are `Stdout`, `Stderr`, their locks, `File`, `Vec<u8>`, `&mut T`, `Box<T>`, and `dyn Write` with `Send` and `Sync` variants.
  A user type cannot implement it, but `&mut dyn Write` or `Box<dyn Write + Send>` works.
- A `dyn Write` always reports `is_terminal() == false`. [stream.rs L92-111][anstream-isterminal-dyn]
  With `Auto` that means `Never`. With an explicit choice this does not matter.
- For `Write` to work the writer must also implement `AsLockedWrite`. [stream.rs L145-258][anstream-aslocked]
  The same set of types qualifies.

### 3.2 Width of stdout and stderr without crossterm

`terminal_size` alone is enough.
`rustix` alone is also enough, at lower level.

`terminal_size::terminal_size_of(fd)` takes any `AsFd` on Unix.
It calls `isatty`, then `tcgetwinsize`, and returns `None` when rows or columns are zero.
[unix.rs L24-41][terminal_size-of].
`std::io::Stdout` and `Stderr` implement `AsFd`, so the call is `terminal_size_of(std::io::stderr())`.

`terminal_size()` with no argument tries stdout, then stderr, then stdin, and returns the first terminal.
[unix.rs L9-19][terminal_size-unix].
It does not say which stream answered, so a library that renders to stderr should call `terminal_size_of` itself.

The ioctl is `TIOCGWINSZ` on the given descriptor.
In rustix the Linux backend issues the raw syscall, [linux_raw syscalls.rs L25-31][rustix-linuxraw-winsize].
The libc backend calls `c::ioctl(.., TIOCGWINSZ, ..)`. [libc syscalls.rs L358-368][rustix-libc-winsize]
The public wrappers are [`tcgetwinsize`][rustix-tc] and [`isatty`][rustix-tty].

On Windows, `terminal_size_of` takes a handle, calls `GetConsoleScreenBufferInfo`, and returns the visible window size, not the buffer size.
[windows.rs L37-70][terminal_size-windows].
The no-argument form uses `GetStdHandle` for stdout, stderr and stdin in that order. [windows.rs L12-32][terminal_size-windows].

`COLUMNS` and `LINES` are not read by `terminal_size`.
clap reads them only as a fallback after `terminal_size` fails. [help_template.rs L1119-1134][clap-dimensions]
console reads neither; `Term::size` returns `(24, 80)` when the ioctl fails. [term.rs L414-426][console-size]

Dependency cost of `terminal_size` 0.4.4:

- Unix: `rustix` with features `termios` plus its default `std`. [Cargo.toml L45-47][terminal_size-cargo]
- Windows: `windows-sys` with `Win32_Foundation` and `Win32_System_Console`.
- On Linux x86_64, aarch64 and several other architectures, `rustix` uses the raw-syscall backend. Its dependencies are `bitflags` and `linux-raw-sys`, with no `libc`. [rustix Cargo.toml][rustix-cargo]
- On macOS and other targets, `rustix` uses the libc backend. It adds `libc` and `libc_errno`. I did not read `libc_errno`.
- Source size of `terminal_size`: 256 lines.

`crossterm::terminal::size()` is different.
It opens `/dev/tty` and queries that, falling back to stdout only when `/dev/tty` cannot be opened, then falls back to running `tput`.
[terminal/sys/unix.rs L59-105][crossterm-size].
So crossterm answers "how big is my controlling terminal", not "how big is the stream I render to".
For output that may be redirected, this is the wrong question.

### 3.3 How indicatif draws and redraws

**Depends on console: yes.**
`console` 0.16.4 or later is a non-optional dependency with `ansi-parsing` and `std`.
[Cargo.toml][indicatif-cargo].
It does not depend on `anstream` or `anstyle`.

**Draw target.**
`ProgressDrawTarget` wraps one of four kinds: a `Term`, a shared `MultiState`, `Hidden`, or a `TermLike`.
[draw_target.rs L259-282][indicatif-targetkind].
`stdout()` and `stderr()` build a buffered `console::Term` at 20 Hz. [L37-47][indicatif-target-ctor].
`ProgressBar::new` uses stderr. [progress_bar.rs L46-47][indicatif-pb-new].
`term()` returns `Hidden` when the stream is not a terminal or `TERM=dumb`. [L79-92][indicatif-target-term].
`TermLike` is a trait with `width`, `height`, `move_cursor_*`, `write_str`, `write_line`, `clear_line` and `flush`.
[term_like.rs][indicatif-termlike].
A library can implement it to draw to its own writer.

**Escape sequences.**
All of them come from `console`.
Cursor up is `ESC [ n A`, cursor down is `ESC [ n B`, and clear line is `CR ESC [ 2 K`.
[common_term.rs L5-19, L50-52][console-common-term].
Indicatif never switches to the alternate screen and never hides the cursor by itself.
Width comes from `Term::size`, which runs the ioctl from section 3.2 on the term's own descriptor.

**Redraw algorithm.**
[`draw_to_term`, draw_target.rs L513-590][indicatif-draw-to-term].

1. If lines were drawn last time, move up by the previous visual line count, minus one, and clear each line. In `move_cursor` mode it only moves up and writes `CR`.
2. Compute the visual height of the new lines at the current width. The count strips ANSI codes with `AnsiCodeIterator` and, with `unicode-width`, counts cells per character. [L649-700][indicatif-linecount]
3. Write each line, then pad with spaces to the right edge so later output starts on the next line.
4. Skip bar lines that would exceed the terminal height. Flush. Store the new line count.

**Rate limiting.**
Two layers.

- Per target: a token bucket of `1000 / hz` ms per token with a burst capacity of 20. A draw that is not forced and not allowed is dropped. [draw_target.rs L183-221, L446-495][indicatif-ratelimiter]
- Per position update: `AtomicPosition::allow` is a lock-free bucket with 1 ms tokens and a burst of 10. It lets `inc` skip taking the state lock on most calls. [state.rs L534-606][indicatif-atomicpos]

A finished bar forces a draw. [state.rs L193-217][indicatif-bar-draw]

**Steady tick.**
`enable_steady_tick` spawns one thread per bar that locks the bar state, calls `tick`, and waits on a `Condvar`.
[progress_bar.rs L181-220, L687-760][indicatif-ticker].
The thread holds only a `Weak` reference, so dropping the bar ends it.

**Multi-progress.**
`MultiProgress` holds `Arc<RwLock<MultiState>>` with a member list, an ordering vector, and `orphan_lines` for printed text.
[multi.rs L138, L327-355][indicatif-multi].
Each added bar gets a `TargetKind::Multi` that points back to the shared state.
A draw rebuilds the whole block: it collects the lines of every member in order, reaps finished bars from the top, and hands the combined lines to the one real target.
`println` and `suspend` clear the block, run, and redraw. [multi.rs L388-509][indicatif-multi-draw].

**Lessons for a new library.**
Indicatif owns the cursor model: it assumes it controls the last N lines.
Text printed to the same stream by other code breaks it, which is why `println` and `suspend` exist.
The `TermLike` seam is the cleanest hook for sharing one writer with a renderer.
A rich-like renderer can reuse the idea (count visual lines, move up, clear, rewrite) without depending on `console`.

## 4. Crate by crate

Each entry follows the seven points: what it does, style model, ANSI writing, detection, dependencies, maintenance, lessons.
Source size counts every `.rs` file under `src/`, including inline tests.

### 4.1 anstyle 1.0.14

1. **Purpose.** Data types for ANSI styles with no I/O and no detection. Designed to sit in public APIs.

   ```rust
   use anstyle::{AnsiColor, Style};
   const WARN: Style = AnsiColor::Yellow.on_default().bold();
   println!("{WARN}careful{WARN:#}");
   ```

2. **Style model.** `Style` is a `Copy` struct of `fg`, `bg`, `underline` (each `Option<Color>`) and `effects`. [style.rs L18-23][anstyle-style]
   `Color` is an enum of `Ansi(AnsiColor)`, `Ansi256(Ansi256Color)` and `Rgb(RgbColor)`. [color.rs L4-17][anstyle-color]
   `Effects` is a `u16` newtype with const flags for bold, dim, italic, five underline kinds, blink, invert, hidden and strikethrough. [effect.rs L9-38][anstyle-effects]
   Builders are `const fn`. `Ansi256Color::into_ansi` converts 0 to 15 losslessly to `AnsiColor`; nothing converts RGB. No hyperlinks.
3. **Writing ANSI.** `Display` for `Style` writes the start codes, and the alternate flag (`{style:#}`) writes the reset. [style.rs L411-419][anstyle-style-display]
   The reset is the fixed string `ESC [ 0 m`, elided when the style is plain. [style.rs L153-160][anstyle-style-reset], [reset.rs L22][anstyle-reset]
   Colors format through a 19-byte stack buffer, not `format!`. [color.rs L568-635][anstyle-dispbuf]
   It never parses ANSI.
4. **Detection.** None.
5. **Dependencies.** None. Feature `std` (default) adds `write_to` helpers. 1,648 lines. MSRV 1.66.
6. **Maintenance.** 2026-03-13. About 892 M downloads. clap_builder and cargo depend on it (`pub use anstyle::*` in [styling.rs L3][clap-styling-reexport]; [cargo Cargo.toml][cargo-cargo]).
7. **Lessons.** Take it as the style type. It is the shared vocabulary of clap, cargo and anstream. Missing: color-depth downgrade, hyperlinks, a style diff (to emit fewer codes).

### 4.2 anstream 1.0.0

1. **Purpose.** Writers that adapt ANSI text to the destination: pass through, strip, or translate for the legacy Windows console. Also `print!`/`println!` replacements.

   ```rust
   use anstyle::AnsiColor;
   use std::io::Write as _;
   let mut out = anstream::AutoStream::new(std::io::stderr().lock(), anstream::ColorChoice::Never);
   writeln!(out, "{}ok{:#}", AnsiColor::Green.on_default(), AnsiColor::Green.on_default())?;
   ```

2. **Style model.** None of its own. It accepts any text with ANSI and uses `anstyle::Style` when translating for Windows.
3. **Parsing ANSI.** Yes, two ways, both on `anstyle-parse`.
   - Strip: [`next_bytes`, adapter/strip.rs L284-329][anstream-strip-next].
     It runs the state machine over bytes until a printable one, copies the printable run as a slice, and drops the rest.
     Output is a sequence of borrowed slices, so there is no per-byte copy.
     UTF-8 continuation bytes bypass the table through `utf8parse`.
     A byte counts as printable if the action is `Print` (except DEL), `BeginUtf8`, or `Execute` on ASCII whitespace. [L358-367][anstream-strip-printable]
     This removes every CSI, OSC (including OSC 8 hyperlinks, leaving the link text), DCS and escape sequence.
   - Windows translation: `WinconBytes` drives the full `Parser` with a `Perform` impl that tracks SGR and splits text into `(Style, String)` runs. [adapter/wincon.rs L68-93][anstream-wincon-capture]
     `cap_wincon_color` keeps 4-bit colors, converts 256-color values 0 to 15, and drops RGB and the rest. [wincon.rs L175-181][anstream-cap]
   - Cost: one pass, O(bytes), two table lookups per byte. State carries across `write` calls, so split escapes work. I did not measure throughput.
4. **Detection.** See section 3.1. Uses `is_terminal_polyfill` for the tty check, on the wrapped stream.
5. **Dependencies.** Default features `auto` and `wincon`.
   Unix: `anstyle`, `anstyle-parse`, `anstyle-query`, `colorchoice`, `is_terminal_polyfill`, `utf8parse`. None pull `libc`, `rustix` or `windows-sys` on Unix.
   Windows adds `anstyle-wincon` (and through it `windows-sys`).
   Without `auto`, `anstyle-query` is dropped and `AutoStream::new(.., Auto)` becomes `Never`. [Cargo.toml features][anstream-cargo]
   2,562 lines. MSRV 1.66.
6. **Maintenance.** 1.0.0 on 2026-02-11. About 823 M downloads. Used by clap_builder (feature `color`, default) and cargo. [clap Cargo.toml][clap-cargo], [cargo Cargo.toml][cargo-cargo]
7. **Lessons.** Take it for the output boundary. It is the only crate here that strips ANSI from a stream and handles `Never` correctly for any writer. Write styled text once, with ANSI, and let the stream decide. Do not call `print!` in a loop: each call re-detects. Hold an `AutoStream` and lock it.

### 4.3 anstyle-parse 1.0.0

1. **Purpose.** A VT100 state machine in the Paul Williams style, with a `Perform` trait of callbacks (`print`, `execute`, `csi_dispatch`, `osc_dispatch`, `esc_dispatch`, `hook`, `put`, `unhook`). [lib.rs L388-440][anstyle-parse-perform]
2. **Model.** Sixteen states. [definitions.rs L6-23][anstyle-parse-states]
   A `Parser` holds the state, up to 2 intermediates, a `Params` list of up to 32 values, and up to 16 OSC parameters in a raw buffer of 1,024 bytes (feature `core`) or a `Vec` (default). [lib.rs L54-76][anstyle-parse-parser], [params.rs L5][anstyle-parse-params]
3. **Mechanism.** `state_change(state, byte)` is a `const fn`. It looks in an "anywhere" row first, then in the row of the current state. [state/mod.rs L25-35][anstyle-parse-statechange]
   The table is `[[u8; 256]; 16]`, 4 KiB, packing state and action into one byte. [table.rs L4][anstyle-parse-table]
   `Parser::advance` handles UTF-8 out of band. [lib.rs L101-110][anstyle-parse-advance]
   Callers can use `state_change` alone, as the strip adapter does, and skip the `Parser` allocations.
4. **Detection.** None.
5. **Dependencies.** `utf8parse` (default feature `utf8`). `arrayvec` only with feature `core`. 1,378 lines.
6. **Maintenance.** 1.0.0 on 2026-02-11. About 808 M downloads. Pulled in by anstream, hence by clap.
7. **Lessons.** Reuse it if the library must read ANSI at all: measuring text width, wrapping styled text, or tests that compare output. Do not write a second parser. If the library only needs to strip, call `anstream::adapter::strip_str`.

### 4.4 anstyle-query 1.1.5

1. **Purpose.** Pure environment and console lookups, no style types.

   ```rust
   let color = !anstyle_query::no_color() && anstyle_query::term_supports_color();
   ```

2. **Style model.** None.
3. **ANSI.** Only on Windows: `enable_ansi_colors` sets `ENABLE_VIRTUAL_TERMINAL_PROCESSING` on stdout and stderr. [windows.rs L12-80][anstyle-query-windows]
4. **Detection.** `NO_COLOR` and `CLICOLOR_FORCE` are true when non-empty. `CLICOLOR` is `Some(value != "0")`. `TERM` is checked for unset and `dumb` (on Windows an unset `TERM` is allowed). `COLORTERM` is true only for `truecolor` or `24bit`. `CI` is true when present. [lib.rs L23-135][anstyle-query-env]
   It never checks a file descriptor.
5. **Dependencies.** None on Unix. `windows-sys` on Windows. 242 lines.
6. **Maintenance.** 2025-11-13. About 787 M downloads. Used by anstream.
7. **Lessons.** Take it for the individual predicates, or copy the rules. Its definition of "non-empty" for `NO_COLOR` follows the no-color.org text. It has no `FORCE_COLOR` and no depth level.

### 4.5 anstyle-wincon 3.0.11

1. **Purpose.** Write colored text to the legacy Windows console through `SetConsoleTextAttribute`, taking `anstyle` types. [lib.rs L1-9][anstyle-wincon-lib]
2. **Model.** 4-bit colors only, mapped to console attribute nibbles. [windows.rs L179-231][anstyle-wincon-nibble]
3. **ANSI.** `ansi::write_colored` writes SGR for non-Windows `Write`s. [ansi.rs][anstyle-wincon-ansi]
4. **Detection.** None.
5. **Dependencies.** `anstyle`; on Windows `windows-sys` and `once_cell_polyfill`. 473 lines.
6. **Maintenance.** 2025-11-13. About 395 M downloads. Used by anstream on Windows only.
7. **Lessons.** Do not depend on it directly. Get it through anstream. Windows 10 and later support VT mode, so this path matters only for old consoles.

### 4.6 colorchoice 1.0.5

1. **Purpose.** One enum and one process-wide atomic.

   ```rust
   colorchoice::ColorChoice::Never.write_global();
   ```

2. **Model.** `Auto`, `AlwaysAnsi`, `Always`, `Never`, stored in an `AtomicUsize`. [lib.rs L14-59][colorchoice-global]
3. **ANSI.** None.
4. **Detection.** None.
5. **Dependencies.** None; `no_std` outside tests. 115 lines.
6. **Maintenance.** 2026-03-13. About 788 M downloads. Used by anstream, which re-exports it.
7. **Lessons.** Take the enum through `anstream::ColorChoice`. The global atomic is process-wide, so a library that wants per-call control should keep its own value and not call `write_global`.
   Note that clap has a separate three-variant `ColorChoice` (no `AlwaysAnsi`). [util/color.rs L5-12][clap-colorchoice]
   `clap_builder` converts it field by field in `Colorizer::print`. [output/fmt.rs L36-60][clap-colorizer]

### 4.7 owo-colors 4.4.0

1. **Purpose.** Extension trait that wraps any value in a lazy `Display` wrapper.

   ```rust
   use owo_colors::OwoColorize;
   println!("{}", "error".red().bold());
   ```

2. **Style model.** Two forms. Compile-time: zero-sized color types (`fg::<Red>()`, `fg_rgb::<R, G, B>()`), whose escape bytes are const. [lib.rs L286-303][owo-fg], [dyn_styles.rs L429-455][owo-rgb]
   Runtime: `Style { fg, bg, bold, style_flags }` with `style_flags` a `u8` bit set of seven effects. [dyn_styles.rs L83-96][owo-style]
   Colors are 4-bit, xterm 256, CSS names and RGB. No downgrade.
3. **Writing ANSI.** Each wrapper writes its prefix, the value, then a targeted reset: `ESC [ 39 m` after a foreground, `ESC [ 49 m` after a background. [colors.rs L132-147][owo-reset]
   A runtime `Style` writes one combined `ESC [ a;b;c m` prefix and the full reset `ESC [ 0 m`. [dyn_styles.rs L505-585][owo-prefix]
   No hyperlinks. It never parses ANSI.
4. **Detection.** Optional feature `supports-colors` adds `if_supports_color(Stream, |v| v.red())`, backed by `supports-color` and cached per stream. [supports_colors.rs L35-76][owo-supports]
   A global `set_override(bool)` and `with_override` beat detection. [overrides.rs L27-76][owo-override]
5. **Dependencies.** None by default (`alloc`, `supports-colors` optional). With `supports-colors`: `supports-color` 3 and `supports-color` 2. 4,533 lines.
6. **Maintenance.** 2026-08-27. About 194 M downloads. Not used by clap or cargo directly. The anstream docs recommend it for runtime styling. [anstream lib.rs][anstream-lib]
7. **Lessons.** Good ergonomics for call sites. Its detection is per value and cached forever, which suits a CLI whose choice is fixed at startup, but it cannot follow an explicit `--color` flag except through the global override. A library that owns the writer should skip it.

### 4.8 yansi 1.0.1

1. **Purpose.** Paint trait with per-style conditions and optional hyperlinks.

   ```rust
   use yansi::Paint;
   println!("{}", "error".red().bold());
   ```

2. **Style model.** `Style { foreground, background, attributes: Set<Attribute>, quirks: Set<Quirk>, condition: Option<Condition> }`. [style.rs L23-40][yansi-style]
   `Set<T>` is a `u16` bit set. [set.rs L4][yansi-set]
   `Color` has `Primary`, `Fixed(u8)`, `Rgb(u8,u8,u8)` and the 16 named colors. [color.rs L9-28][yansi-color]
   Quirks (`Mask`, `Wrap`, `Linger`, `Clear`, `Resetting`) change reset behavior.
3. **Writing ANSI.** `fmt_prefix` and `fmt_suffix` write to a `dyn fmt::Write`. [style.rs L173, L252-260][yansi-prefix]
   The suffix is the full reset `ESC [ 0 m`, skipped for a default style or the `Linger` quirk.
   The condition is checked at format time, so a disabled style prints the plain value. [paint.rs L237-262][yansi-fmt-args]
   Hyperlinks: `PaintedLink` writes OSC 8 open and close with the ST terminator. [hyperlink.rs L63-99][yansi-link]
4. **Detection.** The `Condition` is a function pointer: global `yansi::whenever(Condition)` or per style. Built-ins, behind features: `stdout_is_tty`, `stderr_is_tty`, `stdin_is_tty`, `clicolor` (`CLICOLOR_FORCE` or `CLICOLOR` not `0`), `no_color`, and combos. Each has a cached and a live form. [condition.rs L77-87, L319-356][yansi-condition]
   On Windows the default condition enables VT mode once. [condition.rs L164-172][yansi-os-support]
   It does not read `COLORTERM`, `TERM` or `FORCE_COLOR`.
5. **Dependencies.** Default `std`. `is-terminal` only with `detect-tty`. 2,728 lines.
6. **Maintenance.** 2024-03-13, no release since. About 301 M downloads. Not used by clap or cargo.
7. **Lessons.** The `Condition` as a swappable function pointer is a tidy design. The "quirks" are a sign that reset strategy is a real design decision. Worth reading, not worth depending on.

### 4.9 nu-ansi-term 0.50.3

1. **Purpose.** Fork of `ansi_term` used by nushell. Style then `paint(text)`.

   ```rust
   use nu_ansi_term::Color;
   println!("{}", Color::Red.bold().paint("error"));
   ```

2. **Style model.** `Style` has `foreground`, `background` and one `bool` per effect. [style.rs L17-60][nu-style]
   `Color` has 16 named, `Fixed(u8)` and `Rgb(u8,u8,u8)`. [style.rs L309-389][nu-color]
   `Difference` computes the minimal codes to go from one style to another. [difference.rs L6-14][nu-difference]
   A `Gradient` helper exists.
3. **Writing ANSI.** `write_prefix` and `write_suffix`, with full reset `ESC [ 0 m` when the style is not plain. [ansi.rs L8, L110-120][nu-ansi]
   Hyperlinks: `.hyperlink(url)` wraps in OSC 8; the open sequence goes between style prefix and text. [display.rs L189-196, L300-308][nu-link]
   A `Difference`-based `AnsiStrings` writer avoids redundant resets between adjacent pieces.
4. **Detection.** None. `enable_ansi_support()` on Windows. [windows.rs L10][nu-windows]
5. **Dependencies.** `windows` (the big crate) on Windows. `serde` optional. 2,891 lines.
6. **Maintenance.** 2025-10-10. About 636 M downloads. Not used by clap or cargo.
7. **Lessons.** The style-diff idea (`Difference`) is the one piece worth borrowing if a renderer wants minimal output. Do not depend on it: no detection, and a heavy Windows dependency.

### 4.10 console 0.16.6

1. **Purpose.** Terminal toolkit from the indicatif and dialoguer family: styles, tty and width detection, cursor moves, key reading, ANSI-aware width and stripping.

   ```rust
   use console::{style, Term};
   let term = Term::stderr();
   eprintln!("{}", style("error").red().bold());
   let (_rows, cols) = term.size();
   ```

2. **Style model.** `Style { fg, bg, fg_bright, bg_bright, attrs, force, for_stderr }`. [utils.rs L283-291][console-style]
   `Color` is 8 named plus `Color256(u8)` and `TrueColor(r,g,b)`. [utils.rs L139-150][console-color]
   Bright is a flag, emitted as `38;5;n+8`, not as `90..97`. No downgrade.
3. **Writing ANSI.** `StyledObject` implements each fmt trait through one macro: write start codes, the value, then `ESC [ 0 m` if anything was written. [utils.rs L791-841][console-fmt]
   It parses ANSI too, with a hand-written scanner (no regex): `strip_ansi_codes`, `AnsiCodeIterator`, `measure_text_width`. [ansi.rs L274-291, L311, L352-395][console-ansi-scan]
   The scanner recognizes CSI, OSC (BEL or ST end) and DCS. Tests cover OSC 8. [ansi.rs L552-560][console-osc8]
4. **Detection.**
   - Color enabled for a stream: `(is_a_color_terminal && CLICOLOR != "0") || CLICOLOR_FORCE != "0"`. [utils.rs L15-22][console-colors]
   - `is_a_color_terminal`: stream is a tty (libc `isatty` on the Term's fd), `NO_COLOR` not set at all (even empty), `TERM` not `dumb` and set (an unset `TERM` counts as dumb on Unix). [unix_term.rs L25-35][console-color-terminal], [term.rs L579-590][console-dumb]
   - True color: also `COLORTERM` is `truecolor` or `24bit`. [unix_term.rs L37-42][console-color-terminal]
   - Results are cached in a `OnceLock<AtomicBool>` per stream and can be overridden with `set_colors_enabled` and `set_colors_enabled_stderr`. [utils.rs L25-40, L65][console-colors]
   - Width: `ioctl(fd, TIOCGWINSZ)` on the Term's own fd, only if it is a tty. [unix_term.rs L53-70][console-ioctl]
5. **Dependencies.** Defaults `std`, `ansi-parsing`, `unicode-width`. `libc` on Unix (direct `isatty`, `ioctl`, `tcgetattr`). On Windows `windows-sys` and `encode_unicode`. 4,423 lines. MSRV 1.71.
   No `rustix`, `parking_lot`, `mio` or `signal-hook`.
6. **Maintenance.** 2026-09-10. About 412 M downloads. Used by indicatif; not by clap or cargo.
7. **Lessons.** Its useful parts are the ANSI-aware width measure and the cursor helpers; both are small. Its color logic is global and per-process, which fits poorly with a library that takes a writer and a choice. Do not take it unless indicatif comes along.

### 4.11 termcolor 1.4.1

1. **Purpose.** Cross-platform colored writer from the BurntSushi family. Writes through a `WriteColor` trait with `set_color(&ColorSpec)` and `reset()`.

   ```rust
   use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
   use std::io::Write as _;
   let mut err = StandardStream::stderr(ColorChoice::Auto);
   err.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))?;
   writeln!(err, "error")?;
   err.reset()?;
   ```

2. **Style model.** `ColorSpec` has `fg`, `bg`, and booleans for bold, intense, underline, dimmed, italic, reset, strikethrough. [lib.rs L1826-1836][termcolor-spec]
   `Color` has 8 named, `Ansi256(u8)`, `Rgb(u8,u8,u8)`. [lib.rs L2071-2083][termcolor-color]
   On the Windows console path, 256-color and RGB values outside 0 to 15 return `None`. No downgrade.
3. **Writing ANSI.** Imperative: `set_color` writes `ESC [ 1 m`, `2`, `3`, `4`, `9` and color codes; `reset` writes `ESC [ 0 m`. [lib.rs L1518-1557][termcolor-ansi]
   Hyperlinks: `set_hyperlink(&HyperlinkSpec)` writes OSC 8 with ST. [lib.rs L1547-1553][termcolor-ansi]
   The caller must reset. It never parses ANSI.
4. **Detection.** `Auto` checks only `TERM` (unset or `dumb` means no color on Unix) and `NO_COLOR` (set means no color). It does not check for a tty. [lib.rs L292-335][termcolor-detect]
   The crate's docs tell callers to check `stdin().is_terminal()` themselves. [lib.rs L91-104][termcolor-docs]
5. **Dependencies.** `winapi-util` on Windows only. None on Unix. 2,572 lines.
6. **Maintenance.** 2024-01-10, no release since. About 574 M downloads. I did not verify which downstream tools use it.
7. **Lessons.** The imperative `WriteColor` model forces callers to pair set and reset. A `Display`-based model is safer. The missing tty check in `Auto` is a trap. Do not take it.

### 4.12 supports-color 3.0.2

1. **Purpose.** Color level detection, a port of the Node package.

   ```rust
   if let Some(level) = supports_color::on(supports_color::Stream::Stderr) {
       let _truecolor = level.has_16m;
   }
   ```

2. **Model.** `ColorLevel { level, has_basic, has_256, has_16m }`. [lib.rs L185-199][supports-color-level]
3. **ANSI.** None.
4. **Detection.** The one crate here that returns a depth. Order, in `supports_color`: [lib.rs L90-117][supports-color-detect]
   1. `FORCE_COLOR` (`true` or empty is 1, `false` is 0, a number is clamped to 3), else `CLICOLOR_FORCE` not `0` is 1. [L36-52][supports-color-force] A positive value wins over everything.
   2. `NO_COLOR` set and not `0`, or `TERM=dumb`, or the stream is not a tty (unless `IGNORE_IS_TERMINAL` is set and not `0`): level 0.
   3. Level 3 if `COLORTERM` is `truecolor` or `24bit`, `TERM` ends in `direct` or `truecolor`, or `TERM_PROGRAM=iTerm.app`. [L142-148][supports-color-16m]
   4. Level 2 if `TERM_PROGRAM=Apple_Terminal`, or `TERM` ends in `256` or `256color`. [L150-152][supports-color-16m]
   5. Level 1 if `COLORTERM` is set, `TERM` is set and not `dumb` (Windows: also unset), `CLICOLOR` not `0`, or a CI system is detected through `is_ci`.
   6. Otherwise none.
   The tty check is `std::io::IsTerminal` on `stdout()` or `stderr()` as named by `Stream`. [L82-88][supports-color-tty]
   Results can be cached per stream with `on_cached`. [L178-183][supports-color-cache]
5. **Dependencies.** `is_ci`. 276 lines.
6. **Maintenance.** 2024-11-26. About 79 M downloads. Used by owo-colors (optional). Not by clap or cargo.
7. **Lessons.** The best source for rules of thumb on depth: copy the ladder, since it is small, or take the crate for `has_256` and `has_16m`. Note it treats `NO_COLOR` as "set and not `0`", while anstyle-query treats it as "non-empty". Pick one and test it. It gives a depth, but `FORCE_COLOR` and `CLICOLOR_FORCE` return level 1 or the number, not a real depth probe.

### 4.13 supports-hyperlinks 3.2.0

1. **Purpose.** Whether the terminal renders OSC 8 links.

   ```rust
   let links = supports_hyperlinks::on(supports_hyperlinks::Stream::Stdout);
   ```

2. **Model.** A `bool`.
3. **ANSI.** None.
4. **Detection.** `FORCE_HYPERLINK` (anything but `0`) decides first. Then: `DOMTERM`; `VTE_VERSION` at least 5000; `TERM_PROGRAM` one of `Hyper`, `iTerm.app`, `terminology`, `WezTerm`, `vscode`, `ghostty`, `zed`; `TERM` one of `xterm-kitty`, `alacritty`, `alacritty-direct`; `COLORTERM=xfce4-terminal`; `WT_SESSION`; `KONSOLE_VERSION`. [lib.rs L12-53][supports-hyperlinks-detect]
   `on(stream)` adds a tty check on the named stream, bypassed when `FORCE_HYPERLINK` is set. [L55-67][supports-hyperlinks-on]
5. **Dependencies.** None. 67 lines.
6. **Maintenance.** 2025-12-17. About 48 M downloads. Not used by clap or cargo.
7. **Lessons.** The allow-list is the value; it is 40 lines and out of date the day it ships. Copy it into the library with a documented override (`FORCE_HYPERLINK`) rather than take a dependency. It does not detect tmux or screen, which swallow OSC 8 unless configured. I did not check that against the code beyond noting the lack of a `TMUX` branch.
   Related: `anstyle-hyperlink` 1.0.2 provides a `Hyperlink<D: Display>` that writes OSC 8 and builds `file://` URLs with hostname and percent-encoding. [hyperlink.rs L9-62][anstyle-hyperlink]

### 4.14 terminal_size 0.4.4

See section 3.2.

1. **Purpose.** Terminal size of a given stream or the first of stdout, stderr, stdin.

   ```rust
   use terminal_size::{terminal_size_of, Width};
   let cols = terminal_size_of(std::io::stderr()).map(|(Width(w), _)| w);
   ```

2. **Model.** `Width(u16)` and `Height(u16)`.
3. **ANSI.** None.
4. **Detection.** Per-fd `isatty` plus `TIOCGWINSZ`, or `GetConsoleScreenBufferInfo`. No environment variable. [unix.rs L24-41][terminal_size-of]
5. **Dependencies.** See section 3.2. 256 lines. MSRV 1.71.
6. **Maintenance.** 2026-03-23. About 238 M downloads. clap_builder uses it behind feature `wrap_help` (not default). [clap Cargo.toml][clap-cargo]
7. **Lessons.** Take it. Pass the stream you render to. Add the `COLUMNS` fallback in the library, since this crate does not.

### 4.15 crossterm 0.29.0 (size, tty and style only)

1. **Purpose.** Full terminal control: raw mode, events, cursor, style. Commands (`SetForegroundColor`, `Print`) queue into any `Write`.

   ```rust
   use crossterm::style::Stylize;
   println!("{}", "error".red().bold());
   let (cols, rows) = crossterm::terminal::size()?;
   ```

2. **Style model.** `ContentStyle { foreground_color, background_color, underline_color, attributes }`. [content_style.rs L9-18][crossterm-contentstyle]
   `Attributes` is a `u32` bit set. [attributes.rs L7][crossterm-attrs]
   `Color` has `Reset`, 16 named (named `Dark*` and light pairs), `Rgb { r, g, b }` and `AnsiValue(u8)`. [types/color.rs][crossterm-color]
   Colors are written as `38;5;n` for the 16 named values too, not `30..37`. [colored.rs L96-141][crossterm-colored]
   No downgrade. `available_color_count()` guesses depth from `COLORTERM` then `TERM`, returning 8, 256 or `u16::MAX`. [style.rs L163-181][crossterm-colorcount]
3. **Writing ANSI.** `StyledContent`'s `Display` goes through `execute_fmt`, which formats commands as ANSI. [styled_content.rs L67-77][crossterm-styled]
   Colors format to nothing when `NO_COLOR` is non-empty, memoized once in a `parking_lot::Once`. `force_color_output(bool)` overrides. [colored.rs L75-93][crossterm-nocolor], [style.rs L183-192][crossterm-force]
   No tty check on the styling path. No hyperlinks.
4. **Detection.** Size: `/dev/tty`, then stdout, then `tput`. [terminal/sys/unix.rs L59-105][crossterm-size]
   Tty: `isatty` on a descriptor via `rustix` or `libc`. [tty.rs L24-45][crossterm-tty]
5. **Dependencies.** Default features `bracketed-paste`, `events`, `windows`, `derive-more`. [Cargo.toml features][crossterm-cargo]
   Always: `bitflags`, `document-features`, `parking_lot`; on Unix `rustix` (`std`, `stdio`, `termios`).
   With `events` (default): `mio`, `signal-hook`, `signal-hook-mio`.
   Optional: `libc`, `filedescriptor`, `serde`, `base64`, `futures-core`.
   Windows: `winapi` and `crossterm_winapi` (feature `windows`).
   There is no feature that removes the style module. The minimal set is `default-features = false`, which still pulls `parking_lot`. 11,691 lines.
6. **Maintenance.** 2025-04-05. About 229 M downloads. Not used by clap or cargo.
7. **Lessons.** Do not take it for styling, size or tty. For those three jobs it brings `parking_lot` and, by default, `mio` and `signal-hook`. Its `/dev/tty` size is the wrong source for redirected output. Take it only if raw-mode input is ever in scope.

### 4.16 is-terminal 0.4.17 and std `IsTerminal`

1. **Purpose.** `is_terminal()` on a file descriptor or handle. [lib.rs L51-110][is-terminal-lib]

   ```rust
   use std::io::IsTerminal as _;
   let tty = std::io::stderr().is_terminal();
   ```

2. **Style.** None.
3. **ANSI.** None.
4. **Detection.** `isatty` on Unix and a console-mode query on Windows.
5. **Dependencies.** `libc` on Unix, `windows-sys` on Windows. 382 lines. MSRV 1.63.
6. **Maintenance.** 2025-10-23. About 382 M downloads. yansi uses it behind `detect-tty`.
7. **Lessons.** The trait has been in `std` since Rust 1.70, and the crate's README says it matches what is in std. [std docs][std-isterminal]
   Use `std::io::IsTerminal` and do not add this crate. anstream wraps the same std call through `is_terminal_polyfill`. [stream.rs L60-67][anstream-isterminal-polyfill]

### 4.17 indicatif 0.18.6

See section 3.3.

1. **Purpose.** Progress bars and spinners with templates, ETA, and multi-bar layout.

   ```rust
   use indicatif::{ProgressBar, ProgressStyle};
   let pb = ProgressBar::new(100);
   pb.set_style(ProgressStyle::with_template("{bar:40} {pos}/{len}")?);
   pb.inc(1);
   pb.finish();
   ```

2. **Style model.** Template strings with `{bar}`, `{spinner}`, `{msg}` and colored placeholders such as `{bar:40.cyan/blue}`. Colors come from `console::Style`. [style.rs L60-70][indicatif-style]
3. **ANSI.** Writes cursor moves and clears through `console`. Parses ANSI to measure width. Never strips. Does not use OSC 9;4 progress.
4. **Detection.** Hidden if not a terminal or `TERM=dumb`. [draw_target.rs L79-92][indicatif-target-term]
   Color follows `console::colors_enabled` and `colors_enabled_stderr`, so it follows the `console` rules in section 4.10.
5. **Dependencies.** `console` (with `ansi-parsing`, `std`), `portable-atomic`, `unit-prefix`, `unicode-width` (default). `web-time` on wasm. Optional: `rayon`, `tokio`, `futures-core`, `unicode-segmentation`, `vt100`. [Cargo.toml][indicatif-cargo]
   Through `console`: `libc` on Unix, `windows-sys` on Windows. 6,572 lines. MSRV 1.85.
6. **Maintenance.** 2026-07-01. About 252 M downloads. Not used by clap. The cargo manifest lists no `indicatif`; it lists `anstyle-progress`. [cargo Cargo.toml][cargo-cargo]
7. **Lessons.** Use it as is if bars are needed, behind a feature. Do not mix its writes with a renderer that owns the same stream, except through `MultiProgress::suspend` or `println`. For a rich-like library, the draw model is worth copying; the dependency on `console` is the cost.
   Related: `anstyle-progress` 0.1.6 writes the OSC 9;4 terminal progress sequence and decides support from `TERM_FEATURES`, `TERM_PROGRAM`, `TERM`, `WT_SESSION` and `ConEmuANSI`. [query.rs L1-45][anstyle-progress-query]

### 4.18 clap_builder 4.6.7 styled help

1. **Purpose.** The help and error text of clap. `Styles` assigns an `anstyle::Style` to each role. `StyledStr` holds text with embedded ANSI.

   ```rust
   use clap::builder::styling::{AnsiColor, Styles};
   let styles = Styles::styled()
       .header(AnsiColor::Yellow.on_default().bold())
       .literal(AnsiColor::Green.on_default());
   let cmd = clap::Command::new("tool").styles(styles);
   ```

2. **Style model.** `Styles` holds nine `anstyle::Style` fields: header, error, usage, literal, placeholder, valid, invalid, context, context_value. [styling.rs L23-33][clap-styles]
   `pub use anstyle::*` re-exports the types. [styling.rs L3][clap-styling-reexport]
   `Styles::styled()` is bold-underline headers, bold literals, red bold errors, green valid and yellow invalid values. `Styles::plain()` is empty. [L37-75][clap-styles-default]
   Only 4-bit colors by default.
3. **Writing ANSI.** Help is built into a `StyledStr(String)`, a plain `String` with `{style}text{style:#}` already inlined. [styled_str.rs L25][clap-styledstr]
   `ansi()` returns the text with codes. `Display` strips them with `anstream::adapter::strip_str`. [L35-37, L133-136, L213-221][clap-styledstr-display]
   Printing goes through `anstream::AutoStream::new(stdout().lock(), choice)`, with the clap choice mapped onto the anstream choice. [output/fmt.rs L36-60][clap-colorizer]
   The help layout wraps by word with ANSI-aware lengths.
4. **Detection.** Whatever `AutoStream` does for `Auto` (section 3.1), on stdout for help and stderr for errors. The `--color` value only sets `Always`, `Never` or `Auto`. [command.rs L3905-3921][clap-getcolor]
   Width: `terminal_size()` (stdout, stderr, stdin in order), else `COLUMNS` and `LINES`, else 100. Only with feature `wrap_help`; without it width is unlimited. [help_template.rs L122-136, L1119-1127][clap-width]
5. **Dependencies.** Defaults: `anstream` (via `color`), `anstyle`, `clap_lex`, plus `strsim` for suggestions. `terminal_size` only with `wrap_help`. `unicode-width` only with `unicode`. [Cargo.toml][clap-cargo]
   28,992 lines (the whole parser).
6. **Maintenance.** 2026-09-14. About 910 M downloads. It is clap itself.
7. **Lessons.** A library for clap CLIs should accept `clap::builder::styling::Styles`, or at least convert to and from it, because `anstyle::Style` is already the common type. It can emit ANSI into `StyledStr` for `after_help`, and let clap strip it. The same text then works for `--color never`. The roles in `Styles` (header, literal, placeholder, error, valid, invalid) are a ready-made semantic palette.

### 4.19 rustix 1.1.6 (as the width and tty backend)

1. **Purpose.** Safe wrappers over POSIX and Linux syscalls. Here only `termios::isatty` and `termios::tcgetwinsize`. [tc.rs L50][rustix-tc], [tty.rs L22][rustix-tty]
2. **Style.** None; it is not a styling crate.
3. **ANSI.** None.
4. **Detection.** Both functions take `AsFd`.
5. **Dependencies.** With the `termios` feature: `bitflags`, and either `linux-raw-sys` (Linux raw backend) or `libc` and `libc_errno` (other targets or `use-libc`). The source tree has 72,389 lines but only the enabled modules compile. [Cargo.toml][rustix-cargo]
6. **Maintenance.** 2026-10-27. About 1.37 billion downloads, the highest of any crate here. Used by terminal_size and crossterm.
7. **Lessons.** Depend on it only through `terminal_size`. A direct dependency is justified only if the library needs more termios calls.

## 5. Cross-cutting findings

### 5.1 Environment variables by crate

`Y` means read. `n` means not read. Order of precedence is in the crate's section.

| Variable | anstream | console | supports-color | termcolor | crossterm | yansi | clap help |
|:--|:--|:--|:--|:--|:--|:--|:--|
| `NO_COLOR` | Y, non-empty | Y, any value | Y, set and not `0` | Y, any value | Y, non-empty | opt-in | via anstream |
| `CLICOLOR` | Y | Y | Y | n | n | Y | via anstream |
| `CLICOLOR_FORCE` | Y, non-empty | Y, not `0` | Y, not `0` | n | n | Y | via anstream |
| `FORCE_COLOR` | n | n | Y | n | n | n | n |
| `COLORTERM` | n | Y, true color | Y | n | Y, depth | n | n |
| `TERM` | Y | Y | Y | Y | Y, depth | n | via anstream |
| `TERM_PROGRAM` | n | n | Y | n | n | n | n |
| `CI` | Y | n | Y, via `is_ci` | n | n | n | via anstream |
| `COLUMNS`, `LINES` | n | n | n | n | n | n | Y, fallback |
| tty check | on the stream | on the stream | stdout or stderr as asked | none | none | stdout and stderr, opt-in | on the stream |

### 5.2 Reset strategy

| Crate | After a styled value |
|:--|:--|
| anstyle | `ESC [ 0 m`, only if the style is non-plain |
| owo-colors | `ESC [ 39 m` or `ESC [ 49 m` for a single color; `ESC [ 0 m` for a combined style |
| yansi | `ESC [ 0 m`, unless the `Linger` quirk is set |
| nu-ansi-term | `ESC [ 0 m`, with a diff writer to avoid redundant ones |
| console | `ESC [ 0 m` when anything was written |
| termcolor | caller calls `reset()` |
| crossterm | commands; `ResetColor` and `SetAttribute(Reset)` are explicit |

### 5.3 Color depth

No surveyed crate maps RGB to 256 colors or 256 colors to 16.
The only lossy path is anstream's Windows console translation, which drops RGB and 256-color values above 15.
The detection that gives a depth is in `supports-color` (levels 1 to 3), `console` (true color yes or no), and `crossterm` (8, 256, or all).
A library that supports `--color` and depth must write the downgrade, in `anstyle` types, as a pure function.

### 5.4 Hyperlinks (OSC 8)

Writers: yansi, nu-ansi-term, termcolor, anstyle-hyperlink.
Detection: supports-hyperlinks only.
Stripping: anstream (the state machine ignores OSC) and console (`AnsiCodeIterator`).
anstyle has no hyperlink type.

## 6. What to reuse and what to avoid

### 6.1 Strong candidates to take as dependencies

| Need | Take | Why | Cost on Unix |
|:--|:--|:--|:--|
| Style data type | `anstyle` | Same type as clap and cargo; `const` builders; `Copy` | none |
| Output adapter, strip | `anstream` (`AutoStream`, `strip_str`) | Correct stripping for any writer; explicit choice; Windows handled | `anstyle-parse`, `anstyle-query`, `colorchoice`, `utf8parse`, `is_terminal_polyfill` |
| Choice enum | `anstream::ColorChoice` (from `colorchoice`) | Re-exported; has `AlwaysAnsi` | none |
| Parse ANSI (width, wrap, tests) | `anstyle-parse` | Table-driven, `const` | `utf8parse` |
| Terminal width | `terminal_size` | Per-stream; tiny | `rustix` (`termios`) |
| Tty check | `std::io::IsTerminal` | In std | none |
| Display width of text | `unicode-width` | Already in clap's `unicode` feature and in cargo | none |
| OSC 8 hyperlinks | `anstyle-hyperlink` | `Display` wrapper with `file://` URL helpers | `percent-encoding` and `libc` behind features |
| Bars | `indicatif`, behind a feature | Mature; accept its `console` dependency | `console`, `libc` |

### 6.2 Write in the library

- Detection that returns a depth, from `COLORTERM`, `TERM` and `FORCE_COLOR`. Copy the supports-color ladder and test it.
- The RGB to 256 to 16 downgrade.
- The `COLUMNS` fallback.
- The hyperlink allow-list, with `FORCE_HYPERLINK`.
- One function that turns the process environment into a `ColorChoice` once, stored in the library's own context rather than a process global. Then pass `Always` or `Never` to `AutoStream::new`.

### 6.3 Do not take

- `crossterm` for style, size or tty: `parking_lot` always, `mio` and `signal-hook` by default, and the size comes from `/dev/tty`.
- `termcolor`: no tty check in `Auto`, imperative reset, no release since 2024-01.
- `is-terminal`: redundant with std.
- `console` for styling: global per-process color state, `libc` direct calls. Acceptable only as indicatif's dependency.
- `nu-ansi-term`: pulls the `windows` crate on Windows and has no detection.
- `owo-colors` and `yansi` as the core: they write ANSI at format time and decide by global state. Fine for an application. A library that owns a writer is better served by `anstyle` plus `anstream`.

## 7. Comparison

Columns: **Style** is the style representation. **Depth** is 4-bit, 8-bit and RGB support in the type. **OSC 8** is hyperlink writing. **Parses ANSI** is whether the crate reads escape sequences. **Detects** is what it decides.
Deps are the direct dependencies on Unix with default features.

| Crate | Version | Role | Style | Depth | OSC 8 | Parses ANSI | Detects | Direct deps (Unix, default) | src lines | Released |
|:--|:--|:--|:--|:--|:--|:--|:--|:--|--:|:--|
| anstyle | 1.0.14 | style type | `Copy` struct, `u16` effects | 4, 8, RGB | no | no | no | none | 1,648 | 2026-03 |
| anstream | 1.0.0 | adapter | none | n/a | strips | yes (strip, Windows) | env and tty (Auto) | 5 | 2,562 | 2026-02 |
| anstyle-parse | 1.0.0 | parser | none | n/a | skips | yes | no | 1 | 1,378 | 2026-02 |
| anstyle-query | 1.1.5 | env | none | n/a | no | no | env only | none | 242 | 2025-11 |
| anstyle-wincon | 3.0.11 | Windows console | uses anstyle | 4 | no | no | no | `anstyle` | 473 | 2025-11 |
| colorchoice | 1.0.5 | choice | none | n/a | no | no | global atomic | none | 115 | 2026-03 |
| owo-colors | 4.4.0 | paint | const types, `u8` flags | 4, 8, RGB, CSS | no | no | optional | none | 4,533 | 2026-08 |
| yansi | 1.0.1 | paint | struct, `u16` sets | 4, 8, RGB | yes | no | conditions | none (`is-terminal` opt.) | 2,728 | 2024-03 |
| nu-ansi-term | 0.50.3 | paint | struct of bools | 4, 8, RGB | yes | no | no | none | 2,891 | 2025-10 |
| console | 0.16.6 | toolkit | struct, bright flag | 4, 8, RGB | scans | yes (scanner) | env, tty, size | `libc`, `unicode-width` | 4,423 | 2026-09 |
| termcolor | 1.4.1 | writer | `ColorSpec` bools | 4, 8, RGB | yes | no | `TERM`, `NO_COLOR` | none | 2,572 | 2024-01 |
| supports-color | 3.0.2 | depth | `ColorLevel` | levels 1 to 3 | no | no | full env ladder, tty | `is_ci` | 276 | 2024-11 |
| supports-hyperlinks | 3.2.0 | links | bool | n/a | detects | no | env allow-list, tty | none | 67 | 2025-12 |
| terminal_size | 0.4.4 | size | `Width`, `Height` | n/a | no | no | per-fd size | `rustix` | 256 | 2026-03 |
| crossterm | 0.29.0 | terminal | struct, `u32` attrs | 4, 8, RGB | no | no | size, depth guess | `bitflags`, `parking_lot`, `rustix`, `mio`, `signal-hook`, others | 11,691 | 2025-04 |
| is-terminal | 0.4.17 | tty | none | n/a | no | no | tty | `libc` | 382 | 2025-10 |
| indicatif | 0.18.6 | progress | templates | via console | no | scans (width) | tty, `TERM` | `console`, `portable-atomic`, `unit-prefix`, `unicode-width` | 6,572 | 2026-07 |
| clap_builder | 4.6.7 | help | `Styles` of `anstyle::Style` | 4 default | no | strips | via anstream | `anstream`, `anstyle`, `clap_lex` | 28,992 | 2026-09 |
| rustix | 1.1.6 | syscalls | none | n/a | no | no | tty, size | `bitflags`, `linux-raw-sys` or `libc` | 72,389 | 2026-10 |

## 8. Not verified

- Nothing was compiled or run. Dependency lists come from the published manifests, not from `cargo tree`. Feature unification in a real build may add crates.
- The permalinks were built from the extracted source at the stated version and were not opened on docs.rs.
- Throughput of the strip adapter and the cost of a parser step were not measured.
- Usage by clap and cargo comes from their manifests only. Usage by other tools (for example, which tools use termcolor) was not checked.
- I did not read `libc_errno`, the Windows branches of console, or the Windows paths of crossterm beyond the files cited.
- Behavior inside tmux or screen for OSC 8 was not checked in code.
- Download counts are crates.io figures on 2026-11-07 and include transitive installs, so they measure reach, not direct choice.

[anstream-auto-enum]: https://docs.rs/crate/anstream/1.0.0/source/src/auto.rs#L23-29
[anstream-auto-new]: https://docs.rs/crate/anstream/1.0.0/source/src/auto.rs#L62-72
[anstream-auto-always]: https://docs.rs/crate/anstream/1.0.0/source/src/auto.rs#L92-133
[anstream-auto-choice]: https://docs.rs/crate/anstream/1.0.0/source/src/auto.rs#L197-223
[anstream-auto-doc]: https://docs.rs/crate/anstream/1.0.0/source/src/auto.rs#L35-60
[anstream-macros]: https://docs.rs/crate/anstream/1.0.0/source/src/_macros.rs#L61-75
[anstream-rawstream]: https://docs.rs/crate/anstream/1.0.0/source/src/stream.rs#L5-30
[anstream-isterminal-dyn]: https://docs.rs/crate/anstream/1.0.0/source/src/stream.rs#L92-111
[anstream-isterminal-polyfill]: https://docs.rs/crate/anstream/1.0.0/source/src/stream.rs#L60-67
[anstream-aslocked]: https://docs.rs/crate/anstream/1.0.0/source/src/stream.rs#L145-258
[anstream-strip-next]: https://docs.rs/crate/anstream/1.0.0/source/src/adapter/strip.rs#L284-329
[anstream-strip-printable]: https://docs.rs/crate/anstream/1.0.0/source/src/adapter/strip.rs#L358-367
[anstream-wincon-capture]: https://docs.rs/crate/anstream/1.0.0/source/src/adapter/wincon.rs#L68-93
[anstream-cap]: https://docs.rs/crate/anstream/1.0.0/source/src/wincon.rs#L175-181
[anstream-cargo]: https://docs.rs/crate/anstream/1.0.0/source/Cargo.toml
[anstream-lib]: https://docs.rs/crate/anstream/1.0.0/source/src/lib.rs#L1-20
[colorchoice-global]: https://docs.rs/crate/colorchoice/1.0.5/source/src/lib.rs#L14-59
[anstyle-style]: https://docs.rs/crate/anstyle/1.0.14/source/src/style.rs#L18-23
[anstyle-style-display]: https://docs.rs/crate/anstyle/1.0.14/source/src/style.rs#L411-419
[anstyle-style-reset]: https://docs.rs/crate/anstyle/1.0.14/source/src/style.rs#L153-160
[anstyle-reset]: https://docs.rs/crate/anstyle/1.0.14/source/src/reset.rs#L22
[anstyle-effects]: https://docs.rs/crate/anstyle/1.0.14/source/src/effect.rs#L9-38
[anstyle-color]: https://docs.rs/crate/anstyle/1.0.14/source/src/color.rs#L4-17
[anstyle-dispbuf]: https://docs.rs/crate/anstyle/1.0.14/source/src/color.rs#L568-635
[anstyle-parse-perform]: https://docs.rs/crate/anstyle-parse/1.0.0/source/src/lib.rs#L388-440
[anstyle-parse-states]: https://docs.rs/crate/anstyle-parse/1.0.0/source/src/state/definitions.rs#L6-23
[anstyle-parse-parser]: https://docs.rs/crate/anstyle-parse/1.0.0/source/src/lib.rs#L54-76
[anstyle-parse-params]: https://docs.rs/crate/anstyle-parse/1.0.0/source/src/params.rs#L5
[anstyle-parse-statechange]: https://docs.rs/crate/anstyle-parse/1.0.0/source/src/state/mod.rs#L25-35
[anstyle-parse-table]: https://docs.rs/crate/anstyle-parse/1.0.0/source/src/state/table.rs#L4
[anstyle-parse-advance]: https://docs.rs/crate/anstyle-parse/1.0.0/source/src/lib.rs#L101-110
[anstyle-query-env]: https://docs.rs/crate/anstyle-query/1.1.5/source/src/lib.rs#L23-135
[anstyle-query-windows]: https://docs.rs/crate/anstyle-query/1.1.5/source/src/windows.rs#L12-80
[anstyle-wincon-lib]: https://docs.rs/crate/anstyle-wincon/3.0.11/source/src/lib.rs#L1-9
[anstyle-wincon-nibble]: https://docs.rs/crate/anstyle-wincon/3.0.11/source/src/windows.rs#L179-231
[anstyle-wincon-ansi]: https://docs.rs/crate/anstyle-wincon/3.0.11/source/src/ansi.rs#L1-23
[owo-fg]: https://docs.rs/crate/owo-colors/4.4.0/source/src/lib.rs#L286-303
[owo-rgb]: https://docs.rs/crate/owo-colors/4.4.0/source/src/dyn_styles.rs#L429-455
[owo-style]: https://docs.rs/crate/owo-colors/4.4.0/source/src/dyn_styles.rs#L83-96
[owo-reset]: https://docs.rs/crate/owo-colors/4.4.0/source/src/colors.rs#L132-147
[owo-prefix]: https://docs.rs/crate/owo-colors/4.4.0/source/src/dyn_styles.rs#L505-585
[owo-supports]: https://docs.rs/crate/owo-colors/4.4.0/source/src/supports_colors.rs#L35-76
[owo-override]: https://docs.rs/crate/owo-colors/4.4.0/source/src/overrides.rs#L27-76
[yansi-style]: https://docs.rs/crate/yansi/1.0.1/source/src/style.rs#L23-40
[yansi-set]: https://docs.rs/crate/yansi/1.0.1/source/src/set.rs#L4
[yansi-color]: https://docs.rs/crate/yansi/1.0.1/source/src/color.rs#L9-28
[yansi-prefix]: https://docs.rs/crate/yansi/1.0.1/source/src/style.rs#L173-260
[yansi-fmt-args]: https://docs.rs/crate/yansi/1.0.1/source/src/paint.rs#L237-262
[yansi-link]: https://docs.rs/crate/yansi/1.0.1/source/src/hyperlink.rs#L63-99
[yansi-condition]: https://docs.rs/crate/yansi/1.0.1/source/src/condition.rs#L77-87
[yansi-os-support]: https://docs.rs/crate/yansi/1.0.1/source/src/condition.rs#L164-172
[nu-style]: https://docs.rs/crate/nu-ansi-term/0.50.3/source/src/style.rs#L17-60
[nu-color]: https://docs.rs/crate/nu-ansi-term/0.50.3/source/src/style.rs#L309-389
[nu-difference]: https://docs.rs/crate/nu-ansi-term/0.50.3/source/src/difference.rs#L6-14
[nu-ansi]: https://docs.rs/crate/nu-ansi-term/0.50.3/source/src/ansi.rs#L110-120
[nu-link]: https://docs.rs/crate/nu-ansi-term/0.50.3/source/src/display.rs#L189-196
[nu-windows]: https://docs.rs/crate/nu-ansi-term/0.50.3/source/src/windows.rs#L10
[console-style]: https://docs.rs/crate/console/0.16.6/source/src/utils.rs#L283-291
[console-color]: https://docs.rs/crate/console/0.16.6/source/src/utils.rs#L139-150
[console-fmt]: https://docs.rs/crate/console/0.16.6/source/src/utils.rs#L791-841
[console-ansi-scan]: https://docs.rs/crate/console/0.16.6/source/src/ansi.rs#L274-291
[console-osc8]: https://docs.rs/crate/console/0.16.6/source/src/ansi.rs#L552-560
[console-colors]: https://docs.rs/crate/console/0.16.6/source/src/utils.rs#L15-40
[console-color-terminal]: https://docs.rs/crate/console/0.16.6/source/src/unix_term.rs#L25-42
[console-dumb]: https://docs.rs/crate/console/0.16.6/source/src/term.rs#L579-590
[console-ioctl]: https://docs.rs/crate/console/0.16.6/source/src/unix_term.rs#L53-70
[console-size]: https://docs.rs/crate/console/0.16.6/source/src/term.rs#L414-426
[console-common-term]: https://docs.rs/crate/console/0.16.6/source/src/common_term.rs#L5-52
[termcolor-spec]: https://docs.rs/crate/termcolor/1.4.1/source/src/lib.rs#L1826-1836
[termcolor-color]: https://docs.rs/crate/termcolor/1.4.1/source/src/lib.rs#L2071-2083
[termcolor-ansi]: https://docs.rs/crate/termcolor/1.4.1/source/src/lib.rs#L1518-1557
[termcolor-detect]: https://docs.rs/crate/termcolor/1.4.1/source/src/lib.rs#L292-335
[termcolor-docs]: https://docs.rs/crate/termcolor/1.4.1/source/src/lib.rs#L91-104
[supports-color-level]: https://docs.rs/crate/supports-color/3.0.2/source/src/lib.rs#L185-199
[supports-color-detect]: https://docs.rs/crate/supports-color/3.0.2/source/src/lib.rs#L90-117
[supports-color-force]: https://docs.rs/crate/supports-color/3.0.2/source/src/lib.rs#L36-52
[supports-color-16m]: https://docs.rs/crate/supports-color/3.0.2/source/src/lib.rs#L142-152
[supports-color-tty]: https://docs.rs/crate/supports-color/3.0.2/source/src/lib.rs#L82-88
[supports-color-cache]: https://docs.rs/crate/supports-color/3.0.2/source/src/lib.rs#L178-183
[supports-hyperlinks-detect]: https://docs.rs/crate/supports-hyperlinks/3.2.0/source/src/lib.rs#L12-53
[supports-hyperlinks-on]: https://docs.rs/crate/supports-hyperlinks/3.2.0/source/src/lib.rs#L55-67
[anstyle-hyperlink]: https://docs.rs/crate/anstyle-hyperlink/1.0.2/source/src/hyperlink.rs#L9-62
[terminal_size-of]: https://docs.rs/crate/terminal_size/0.4.4/source/src/unix.rs#L24-41
[terminal_size-unix]: https://docs.rs/crate/terminal_size/0.4.4/source/src/unix.rs#L9-19
[terminal_size-windows]: https://docs.rs/crate/terminal_size/0.4.4/source/src/windows.rs#L12-70
[terminal_size-cargo]: https://docs.rs/crate/terminal_size/0.4.4/source/Cargo.toml#L45-55
[rustix-linuxraw-winsize]: https://docs.rs/crate/rustix/1.1.6/source/src/backend/linux_raw/termios/syscalls.rs#L25-31
[rustix-libc-winsize]: https://docs.rs/crate/rustix/1.1.6/source/src/backend/libc/termios/syscalls.rs#L358-368
[rustix-tc]: https://docs.rs/crate/rustix/1.1.6/source/src/termios/tc.rs#L50
[rustix-tty]: https://docs.rs/crate/rustix/1.1.6/source/src/termios/tty.rs#L22
[rustix-cargo]: https://docs.rs/crate/rustix/1.1.6/source/Cargo.toml
[crossterm-size]: https://docs.rs/crate/crossterm/0.29.0/source/src/terminal/sys/unix.rs#L59-105
[crossterm-contentstyle]: https://docs.rs/crate/crossterm/0.29.0/source/src/style/content_style.rs#L9-18
[crossterm-attrs]: https://docs.rs/crate/crossterm/0.29.0/source/src/style/attributes.rs#L7
[crossterm-color]: https://docs.rs/crate/crossterm/0.29.0/source/src/style/types/color.rs#L28-95
[crossterm-colored]: https://docs.rs/crate/crossterm/0.29.0/source/src/style/types/colored.rs#L96-141
[crossterm-colorcount]: https://docs.rs/crate/crossterm/0.29.0/source/src/style.rs#L163-181
[crossterm-styled]: https://docs.rs/crate/crossterm/0.29.0/source/src/style/styled_content.rs#L67-77
[crossterm-nocolor]: https://docs.rs/crate/crossterm/0.29.0/source/src/style/types/colored.rs#L75-93
[crossterm-force]: https://docs.rs/crate/crossterm/0.29.0/source/src/style.rs#L183-192
[crossterm-tty]: https://docs.rs/crate/crossterm/0.29.0/source/src/tty.rs#L24-45
[crossterm-cargo]: https://docs.rs/crate/crossterm/0.29.0/source/Cargo.toml
[is-terminal-lib]: https://docs.rs/crate/is-terminal/0.4.17/source/src/lib.rs#L51-110
[std-isterminal]: https://doc.rust-lang.org/std/io/trait.IsTerminal.html
[indicatif-cargo]: https://docs.rs/crate/indicatif/0.18.6/source/Cargo.toml
[indicatif-targetkind]: https://docs.rs/crate/indicatif/0.18.6/source/src/draw_target.rs#L259-282
[indicatif-target-ctor]: https://docs.rs/crate/indicatif/0.18.6/source/src/draw_target.rs#L37-47
[indicatif-target-term]: https://docs.rs/crate/indicatif/0.18.6/source/src/draw_target.rs#L79-92
[indicatif-pb-new]: https://docs.rs/crate/indicatif/0.18.6/source/src/progress_bar.rs#L46-47
[indicatif-termlike]: https://docs.rs/crate/indicatif/0.18.6/source/src/term_like.rs#L1-23
[indicatif-draw-to-term]: https://docs.rs/crate/indicatif/0.18.6/source/src/draw_target.rs#L513-590
[indicatif-linecount]: https://docs.rs/crate/indicatif/0.18.6/source/src/draw_target.rs#L649-700
[indicatif-ratelimiter]: https://docs.rs/crate/indicatif/0.18.6/source/src/draw_target.rs#L183-221
[indicatif-atomicpos]: https://docs.rs/crate/indicatif/0.18.6/source/src/state.rs#L534-606
[indicatif-bar-draw]: https://docs.rs/crate/indicatif/0.18.6/source/src/state.rs#L193-217
[indicatif-ticker]: https://docs.rs/crate/indicatif/0.18.6/source/src/progress_bar.rs#L687-760
[indicatif-multi]: https://docs.rs/crate/indicatif/0.18.6/source/src/multi.rs#L327-355
[indicatif-multi-draw]: https://docs.rs/crate/indicatif/0.18.6/source/src/multi.rs#L388-509
[indicatif-style]: https://docs.rs/crate/indicatif/0.18.6/source/src/style.rs#L60-70
[anstyle-progress-query]: https://docs.rs/crate/anstyle-progress/0.1.6/source/src/query.rs#L1-45
[clap-styles]: https://docs.rs/crate/clap_builder/4.6.7/source/src/builder/styling.rs#L23-33
[clap-styling-reexport]: https://docs.rs/crate/clap_builder/4.6.7/source/src/builder/styling.rs#L3
[clap-styles-default]: https://docs.rs/crate/clap_builder/4.6.7/source/src/builder/styling.rs#L37-75
[clap-styledstr]: https://docs.rs/crate/clap_builder/4.6.7/source/src/builder/styled_str.rs#L25
[clap-styledstr-display]: https://docs.rs/crate/clap_builder/4.6.7/source/src/builder/styled_str.rs#L133-136
[clap-colorchoice]: https://docs.rs/crate/clap_builder/4.6.7/source/src/util/color.rs#L5-12
[clap-colorizer]: https://docs.rs/crate/clap_builder/4.6.7/source/src/output/fmt.rs#L36-60
[clap-getcolor]: https://docs.rs/crate/clap_builder/4.6.7/source/src/builder/command.rs#L3905-3921
[clap-width]: https://docs.rs/crate/clap_builder/4.6.7/source/src/output/help_template.rs#L122-136
[clap-dimensions]: https://docs.rs/crate/clap_builder/4.6.7/source/src/output/help_template.rs#L1119-1134
[clap-cargo]: https://docs.rs/crate/clap_builder/4.6.7/source/Cargo.toml
[cargo-cargo]: https://docs.rs/crate/cargo/0.100.0/source/Cargo.toml#L64-72
