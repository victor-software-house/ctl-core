# Binary-size cost of terminal rendering crates

Measurements for choosing table, styling and terminal dependencies by cost.
All numbers come from release builds on one darwin-arm64 host, with one profile (`opt-level = "s"`, `lto = true`, `codegen-units = 1`, `panic = "abort"`, `strip = true`), so every figure compares like with like.
`__text` is the code section of the Mach-O file and is the fine measure; file sizes move in 16 KiB steps.
The Method section lists exact versions, features and scripts, and every toy's `main.rs` is at the end.

## Findings

1. **Table crates, over a plain `println!` baseline (`__text` bytes).**
   cli-table +19,076; comfy-table without styling +34,068; prettytable-rs +57,084; comfy-table default +67,936; ratatui Table into a Buffer +71,828; comfy-table with `custom_styling` and `tty` (ctl-core main's setting) +80,288; tabled default +115,160; tabled with `ansi` +148,012.
   cli-table and prettytable-rs do not wrap to a width, so they do less work than the others.

2. **Styling crates are close to free.**
   Each of anstyle, owo-colors, yansi, nu-ansi-term, console and termcolor adds 568 to 5,176 bytes over the plain baseline; anstyle with anstream adds 7,208; crossterm's style module adds 9,356 and brings 13 built packages.

3. **Width, wrap and terminal detection.**
   unicode-width with a hand-written wrap adds 4,984; with unicode-segmentation 10,232; textwrap without default features 10,256; textwrap with default features 27,060.
   `rustix` with `termios` (256) and `terminal_size` (788) cost almost nothing; crossterm's `terminal::size` costs 23,396.
   `std::io::IsTerminal` costs nothing measurable.

4. **Progress and diagnostics.**
   indicatif adds 69,820; miette with `fancy` adds 115,448.

5. **Rich ports.**
   Eight crates on crates.io port Python's Rich.
   fast-rich (+121,728) and rusty-rich (+91,588, but its table truncated the long cell) are the small ones.
   rich_rust, rs-rich, rich-rs and richrs add 513,768 to 823,564 bytes; richterm, with styled text only, adds 562,924.
   wealthy has no table or console.

6. **ctl-core, main against the prototype (`__text`).**
   cli -62,016 (-17.2%); help -105,996 (-22.2%); view -117,264 (-25.4%); app -105,624 (-21.2%).
   File size falls by 66,368 at `cli` and about 200,500 at `help`, `view` and `app`.
   The built package count falls by 7 at `cli` and by 18 or 19 at the other tiers; the lockfile falls from 64 to 40 packages.

7. **Only part of that gain is the table engine.**
   The prototype also narrows clap's features.
   Main with only that clap change saves 62,016 at `cli` (the whole `cli` gain), 48,560 at `help`, 60,212 at `view` and 48,244 at `app`.
   The remainder, 57,436 at `help`, 57,052 at `view` and 57,380 at `app`, is the table engine swap (comfy-table and its dependencies out, `terminal_size` in).
   This agrees with `cargo bloat` on main's `app` tier, where comfy_table, crossterm, ansitok, unicode_segmentation, parking_lot, console and ansi_str sum to about 55 KiB.

8. **Build time.**
   Clean release wall time is within noise: -2% to -7% on the prototype, and the host load changed between runs.
   CPU time (user plus system) is clearly lower: -16% at `cli` and -27% to -35% at `help`, `view` and `app`.
   The dev profile `app` build is -8% wall and about -25% CPU.
   The wall time is dominated by the single-threaded link-time optimisation step, which does not shrink much with fewer crates.

9. **Consumers, replacing comfy-table (patched to the prototype, `__text`).**
   With each consumer's own profile: verctl -44,096 (-0.36%), qctl -45,632 (-0.82%), forkctl -44,544 (-0.98%), rawr -28,524 (-3.0%).
   With the common profile: verctl -29,972, qctl -31,512, forkctl -31,068, rawr -24,180.
   All four compiled against the prototype, including the three pinned to 0.6.23 and 0.6.24 once the export's version field was set to the pin.
   Packages that exist in the build only through comfy-table: verctl 10, qctl 13, forkctl 14, rawr 8.
   rawr keeps crossterm and unicode-segmentation because ratatui needs them, so it loses 8 packages instead of 10 to 14, but the saving is the largest share of its binary.

10. **What dominates the binaries.**
    The consumers are 1.2 to 15.7 MB.
    The common profile alone changes verctl's `__text` from 12,293,148 to 7,625,564 (-38%), which is far larger than any table-engine choice.

## Not verified

- The patched consumer binaries were compiled but not run, and their tests were not run.
- The "narrow clap" variant removes only the `.color(...)` calls that the narrowed clap cannot compile. It measures the size effect of the clap features, not the prototype's behaviour.
- The patched verctl and forkctl also moved `yamled` from 0.0.7 to 0.0.8, because the prototype requires 0.0.8. That adds a small unmeasured amount to their deltas.
- Wall times come from a host shared with other users; the load averages are listed with each build. Two runs per cell is too few for a confidence statement.
- Workloads are comparable, not identical. See "Matrix A workloads" in the Method section for the differences per crate.
- `cargo bloat` splits code by symbol names and its own note calls the numbers guesswork.

## Method

### Host and toolchain

- Host: a darwin-arm64 machine (Apple M4 Pro, 12 cores, 24 GiB, macOS 26.7.1), shared with other users, so wall times carry noise. Every build ran on this one host.
- Toolchain: `RUSTUP_TOOLCHAIN=1.99.1`, `cargo` called straight from the rustup toolchain (no wrapper, no shared build cache), target `aarch64-apple-darwin`.
- `cargo 1.99.1 (5f94df478 2026-08-27)`
- `cargo-bloat 0.12.1`

```text
rustc 1.99.1 (58cf8f9c3 2026-10-13)
binary: rustc
commit-hash: 58cf8f9c35b355199727627d07ae34a4e2846d67
commit-date: 2026-10-13
host: aarch64-apple-darwin
release: 1.99.1
LLVM version: 23.1.1
```

### Profile

Every toy sets this profile. Consumers were built twice: once with their own profile and once with these values passed as `CARGO_PROFILE_RELEASE_OPT_LEVEL=s`, `_LTO=true`, `_CODEGEN_UNITS=1`, `_PANIC=abort`, `_STRIP=true`.

```toml
[profile.release]
opt-level = "s"
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

The consumers' own profiles: verctl, qctl and forkctl use `codegen-units = 1`, `lto = "thin"`, `panic = "abort"`, `strip = "symbols"` and the default `opt-level` (3). rawr uses `codegen-units = 1`, `lto = true`, `panic = "abort"`, `strip = true` and the default `opt-level` (3).

### What is recorded per binary

- File bytes: `stat -f %z <bin>`.
- `__text` bytes: the `Section __text:` line of `size -m <bin>`. Mach-O pads segments to 16 KiB, so `__text` is the fine measure and the file size moves in steps.
- Lock pkgs: the number of `[[package]]` entries in the toy's `Cargo.lock`, minus the toy itself. This counts every platform, so it includes Windows and Redox crates that never reach a darwin binary. For consumers it is the number of entries that have a registry or git source.
- Built pkgs: distinct `name version` lines of `cargo tree --prefix none -e normal,build --target aarch64-apple-darwin`, minus the toy (for Matrix B the count includes ctl-core, which is a path dependency). This is the set that can reach the binary on the measured target.
- Strip proof: `nm <bin> | wc -l` printed 66 to 229 lines for each of the 70 binaries (dynamic-linking stubs only; the larger counts are the consumers), and `nm -U <bin>` showed 0 symbols matching `_ZN` or `__R` (Rust legacy or v0 mangling) in all 70.
- Every toy was also run once to check that it prints the workload. The styling toys print raw escape bytes, except anstream's `AutoStream`, which strips them because stdout was not a terminal.

### Crate versions

The newest stable version of each crate was read from the crates.io API on the day of the run. The dependency table of each toy is under "Toy sources". The resolved versions in each `Cargo.lock` equal the listed versions.

### Matrix A workloads

The workloads are comparable, not identical, because the crates differ in what they can do.

- Baseline: `base` prints two formatted lines (padding and a float). `base-plain` prints an integer and a literal. Both are given because `base` pulls in float formatting and padding code that the plain `println!` of several other toys does not; some deltas against `base` are negative for that reason. Prefer `base-plain` when judging a crate's own cost.
- Tables: header `id`, `name`, `note`, three rows, one cell with a CJK word, one with an emoji, one pre-styled bold red, laid out to width 40.
  - comfy-table and tabled receive the red cell as a raw escape string (`\x1b[1;31mfailed\x1b[0m`) and wrap at 40. Without `custom_styling` (comfy-table) or `ansi` (tabled) the escape bytes count toward the cell width.
  - ratatui has no cell wrapping, so the long cell is split into two lines by hand. The red cell uses ratatui's own `Style`. The buffer is printed as plain symbols, so no escape bytes reach stdout.
  - cli-table and prettytable-rs have no width limit, so nothing wraps. Their red cell uses each crate's own style call.
- Styling: one bold red word, one 256-colour word (208) and one RGB word.
- Width and wrap: display width of `Hello 日本語 🚀 e\u{301} 👨‍👩‍👧` and a wrap of one paragraph to 40 columns. The width values differ by crate (20 for unicode-width, 24 for textwrap) because the crates treat combining marks and ZWJ sequences differently. The wrapped text is identical.
- Terminal size and detection: each toy prints the result of one call on stdout. Over ssh without a terminal these print the "not a terminal" branch.
- Progress: a 10-step bar to a stderr draw target.
- Diagnostics: one `MietteDiagnostic` with a code, help text and one labelled span, rendered by `GraphicalReportHandler`.
- Rich ports: the crates.io API search for `rich` and `rich port` returned eight libraries that port Python's Rich. Each toy prints one styled line and, where the crate has a table, a 3-row table. richterm has no table and wealthy has neither a table nor a console; their rows are not comparable and are listed for completeness. The crates `rs-rich-*` other than `rs-rich` itself (art, cli, data, ext, and so on) are extensions or applications of `rs-rich` and were not built.

### Matrix B

- `ctl-core-main`: a clone of the ctl-core repository at origin/main, commit [`7dc8060`][main-sha] (version 0.6.28).
- Prototype: commit `56fa7044d1ace59ecdf175445f2383ecc7647a13` (an unpublished prototype, one commit on top of the main commit above, read for this research and not to be shipped), exported with `git archive`. It replaces comfy-table with its own grid engine, removes `unicode-bidi` and `unicode-general-category`, adds `terminal_size`, and also narrows clap to `default-features = false, features = ["std", "derive", "env", "string"]` (plus `clap/usage` for the `help` feature) with help and errors rendered by ctl-core. It also adds `shell` and `completion` features that no tier here enables.
- The clap change is not part of the table engine, so a third variant isolates it: `ctl-core-main-clapnarrow` is main with the same clap features and with only the `.color(...)` calls removed (clap's `color` feature is gone). Behaviour of Clap's own help in that variant is not equal to the prototype's; only the dependency and size effect is comparable.
- The toy is `toy-ctl`, copied unchanged except for the `path` of the `ctl-core` dependency (shown below with its original path). Tiers are `--no-default-features -F cli|help|view|app`.
- One `Cargo.lock` was created for the main toy and copied to the other two toys, so all three resolve the same versions; cargo then prunes it per variant.

### Matrix C

- Each consumer was cloned from its default branch and built with `cargo build --release --bin <name>`, with its lockfile as committed.
- Packages reachable only through comfy-table: `cargo tree -p <name> -e normal,build --target aarch64-apple-darwin` with and without `--prune comfy-table`. The difference is the set of packages that exist in the binary only because of comfy-table. The `cargo tree -i` views for crossterm, ansi-str, ansitok, nom, vte, unicode-segmentation, console, unicode-width, unicode-bidi and unicode-general-category with `--target all` were also read; they agree with the prune result.
- Patched builds: a copy of the consumer with `[patch.crates-io] ctl-core = { path = <prototype export> }`. For a consumer pinned below 0.6.28 the export was copied and only the `version` field was rewritten to the consumer's pin, so the `=0.6.x` requirement matches. Source differences between the pin and the prototype were not reconciled; the builds either compiled or did not.

### cargo bloat

`cargo bloat --release --crates -n 20` with the toy's profile. cargo-bloat builds an unstripped binary, so the stripped `__text` and the `.text` total agree within 1 KiB. Its crate split is a heuristic.

### Build times

Per Matrix B variant and tier: delete the target directory, then `/usr/bin/time -p cargo build -q --release --no-default-features -F <tier>`, run twice, alternating main and prototype. The dev builds use `cargo build -q` with the same flags. The `uptime` load averages before each build are listed.

## Matrix A: one binary per crate

Columns: `Lock pkgs` is the number of packages in the toy's `Cargo.lock` minus the toy. `Built pkgs` is the number of distinct packages in `cargo tree -e normal,build --target aarch64-apple-darwin`, minus the toy. Each binary was run once to check its output.

### A1 Baselines

| Toy | Crate, version, features | File bytes | __text bytes | Δ __text vs base | Δ __text vs base-plain | Lock pkgs | Built pkgs |
|:--|:--|--:|--:|--:|--:|--:|--:|
| base | std only, `println!` with padding and a float | 302,576 | 202,596 | 0 | +12,288 | 0 | 0 |
| base-plain | std only, `println!` with one integer and no padding or float | 285,936 | 190,308 | -12,288 | 0 | 0 | 0 |

### A2 Tables

| Toy | Crate, version, features | File bytes | __text bytes | Δ __text vs base | Δ __text vs base-plain | Lock pkgs | Built pkgs |
|:--|:--|--:|--:|--:|--:|--:|--:|
| tab-comfy-a | comfy-table 8.0.1, default features (`tty`) | 403,920 | 258,244 | +55,648 | +67,936 | 24 | 16 |
| tab-comfy-b | comfy-table 8.0.1, `default-features = false` | 352,144 | 224,376 | +21,780 | +34,068 | 3 | 3 |
| tab-comfy-c | comfy-table 8.0.1, `custom_styling`, `tty` (as ctl-core main) | 420,480 | 270,596 | +68,000 | +80,288 | 33 | 24 |
| tab-tabled-a | tabled 0.22.0, default features (`derive`, `macros`, `assert`) | 418,592 | 305,468 | +102,872 | +115,160 | 12 | 12 |
| tab-tabled-b | tabled 0.22.0, `default-features = false`, `std` | 418,592 | 305,428 | +102,832 | +115,120 | 6 | 5 |
| tab-tabled-c | tabled 0.22.0, `default-features = false`, `std`, `ansi` | 451,696 | 338,320 | +135,724 | +148,012 | 13 | 12 |
| tab-ratatui | ratatui-core 0.1.2 (`std`) + ratatui-widgets 0.3.2 (`default-features = false`, `std`); Table into a Buffer | 401,744 | 262,136 | +59,540 | +71,828 | 58 | 39 |
| tab-clitable | cli-table 0.5.0, default features (`csv`, `derive`) | 319,040 | 209,384 | +6,788 | +19,076 | 19 | 14 |
| tab-prettytable | prettytable-rs 0.10.0, default features | 385,856 | 247,392 | +44,796 | +57,084 | 35 | 16 |

### A3 Styling

| Toy | Crate, version, features | File bytes | __text bytes | Δ __text vs base | Δ __text vs base-plain | Lock pkgs | Built pkgs |
|:--|:--|--:|--:|--:|--:|--:|--:|
| sty-anstyle | anstyle 1.0.14, `Style::render` and `render_reset` | 285,936 | 191,928 | -10,668 | +1,620 | 1 | 1 |
| sty-anstyle-anstream | anstyle 1.0.14 + anstream 1.0.0, `AutoStream::auto(stdout())` | 302,544 | 197,516 | -5,080 | +7,208 | 11 | 7 |
| sty-owo | owo-colors 4.4.0 | 285,936 | 190,876 | -11,720 | +568 | 1 | 1 |
| sty-yansi | yansi 1.0.1 | 285,984 | 195,484 | -7,112 | +5,176 | 1 | 1 |
| sty-nuansi | nu-ansi-term 0.50.3 | 285,952 | 193,500 | -9,096 | +3,192 | 3 | 1 |
| sty-console | console 0.16.6, `Style` | 286,032 | 194,576 | -8,020 | +4,268 | 6 | 3 |
| sty-termcolor | termcolor 1.4.1 | 285,968 | 195,176 | -7,420 | +4,868 | 4 | 1 |
| sty-crossterm | crossterm 0.29.0, `default-features = false`, `style` module | 302,848 | 199,664 | -2,932 | +9,356 | 17 | 13 |

### A4 Width and wrap

| Toy | Crate, version, features | File bytes | __text bytes | Δ __text vs base | Δ __text vs base-plain | Lock pkgs | Built pkgs |
|:--|:--|--:|--:|--:|--:|--:|--:|
| wrap-uwidth | unicode-width 0.2.3, hand-written greedy wrap | 302,448 | 195,292 | -7,304 | +4,984 | 1 | 1 |
| wrap-uwidth-seg | unicode-width 0.2.3 + unicode-segmentation 1.13.3, grapheme-aware width and wrap | 335,504 | 200,540 | -2,056 | +10,232 | 2 | 2 |
| wrap-textwrap-default | textwrap 0.16.4, default features | 352,192 | 217,368 | +14,772 | +27,060 | 34 | 31 |
| wrap-textwrap-uw | textwrap 0.16.4, `default-features = false`, `unicode-width` | 302,496 | 200,564 | -2,032 | +10,256 | 2 | 2 |

### A5 Terminal size and detection

| Toy | Crate, version, features | File bytes | __text bytes | Δ __text vs base | Δ __text vs base-plain | Lock pkgs | Built pkgs |
|:--|:--|--:|--:|--:|--:|--:|--:|
| term-terminal-size | terminal_size 0.4.4 | 286,032 | 191,096 | -11,500 | +788 | 8 | 5 |
| term-rustix | rustix 1.1.6, `default-features = false`, `termios`, `stdio` (`tcgetwinsize` on `stdio::stdout()`) | 285,984 | 190,564 | -12,032 | +256 | 7 | 4 |
| term-rustix-std | rustix 1.1.6, `default-features = false`, `termios`, `std` (`tcgetwinsize(std::io::stdout())`) | 285,984 | 190,628 | -11,968 | +320 | 7 | 4 |
| term-crossterm | crossterm 0.29.0, `default-features = false`, `terminal::size` | 320,752 | 213,704 | +11,108 | +23,396 | 17 | 13 |
| term-std | std `IsTerminal` only | 285,984 | 190,264 | -12,332 | -44 | 0 | 0 |
| term-supports-color | supports-color 3.0.2 | 286,000 | 194,276 | -8,320 | +3,968 | 2 | 2 |
| term-anstyle-query | anstyle-query 1.1.5 | 285,936 | 190,904 | -11,692 | +596 | 3 | 1 |

### A6 Progress

| Toy | Crate, version, features | File bytes | __text bytes | Δ __text vs base | Δ __text vs base-plain | Lock pkgs | Built pkgs |
|:--|:--|--:|--:|--:|--:|--:|--:|
| prog-indicatif | indicatif 0.18.6, default features, 10 steps to a stderr target | 369,456 | 260,128 | +57,532 | +69,820 | 27 | 6 |

### A7 Diagnostics

| Toy | Crate, version, features | File bytes | __text bytes | Δ __text vs base | Δ __text vs base-plain | Lock pkgs | Built pkgs |
|:--|:--|--:|--:|--:|--:|--:|--:|
| diag-miette | miette 7.6.0, `fancy`, `GraphicalReportHandler` rendering one labelled span | 484,720 | 305,756 | +103,160 | +115,448 | 60 | 54 |

### A8 Rich ports found on crates.io

| Toy | Crate, version, features | File bytes | __text bytes | Δ __text vs base | Δ __text vs base-plain | Lock pkgs | Built pkgs |
|:--|:--|--:|--:|--:|--:|--:|--:|
| rich-fast-rich | fast-rich 0.4.0, default features (none) | 470,336 | 312,036 | +109,440 | +121,728 | 38 | 19 |
| rich-rich_rust | rich_rust 0.2.3, default features (none) | 1,661,520 | 981,992 | +779,396 | +791,684 | 68 | 55 |
| rich-rs-rich-default | rs-rich 0.0.15, default features (`syntax`, `markdown`) | 1,891,184 | 1,013,872 | +811,276 | +823,564 | 55 | 50 |
| rich-rs-rich-minimal | rs-rich 0.0.15, `default-features = false` | 1,791,904 | 925,232 | +722,636 | +734,924 | 37 | 28 |
| rich-richrs | richrs 0.2.1, default features (none) | 1,279,088 | 704,076 | +501,480 | +513,768 | 56 | 37 |
| rich-rich-rs | rich-rs 1.3.0 (no features) | 1,396,864 | 835,940 | +633,344 | +645,632 | 99 | 76 |
| rich-rusty-rich-default | rusty-rich 0.4.3, default features (`syntax-highlighting`, `markdown`) | 402,048 | 281,896 | +79,300 | +91,588 | 123 | 92 |
| rich-rusty-rich-minimal | rusty-rich 0.4.3, `default-features = false` | 402,048 | 281,976 | +79,380 | +91,668 | 87 | 53 |
| rich-richterm | richterm 0.5.0, styled text only (no table) | 1,344,960 | 753,232 | +550,636 | +562,924 | 6 | 6 |
| rich-wealthy | wealthy 0.2.0, `cells::cell_len` only (no table or console) | 335,792 | 211,860 | +9,264 | +21,552 | 57 | 56 |

## Matrix B: ctl-core tiers

| Tier | Variant | File bytes | __text bytes | Built pkgs |
|:--|:--|--:|--:|--:|
| cli | main | 468,624 | 361,288 | 22 |
| cli | clapnarrow | 402,256 | 299,272 | 15 |
| cli | proto | 402,256 | 299,272 | 15 |
| help | main | 685,760 | 476,592 | 46 |
| help | clapnarrow | 636,080 | 428,032 | 45 |
| help | proto | 485,200 | 370,596 | 27 |
| view | main | 669,232 | 462,376 | 52 |
| view | clapnarrow | 619,472 | 402,164 | 51 |
| view | proto | 468,640 | 345,112 | 34 |
| app | main | 718,864 | 499,076 | 53 |
| app | clapnarrow | 669,200 | 450,832 | 52 |
| app | proto | 518,320 | 393,452 | 35 |

Lock packages (all tiers share one lockfile per variant, minus the toy): main 64, prototype 40.

### Packages that leave the build graph, main to prototype

Packages in the toy's `Cargo.lock` that the prototype lockfile no longer has, then the darwin-arm64 build graph difference per tier (`only in main`, then `only in prototype`). The `strsim`, `anstream`, `anstyle-query` and related entries at the `cli` tier come from the clap feature change, not from comfy-table.

```text
lock removed:
name = "ansi-str"
name = "ansitok"
name = "arrayvec"
name = "cfg-if"
name = "comfy-table"
name = "console"
name = "crossterm_winapi"
name = "crossterm"
name = "encode_unicode"
name = "lock_api"
name = "minimal-lexical"
name = "nom"
name = "parking_lot_core"
name = "parking_lot"
name = "redox_syscall"
name = "scopeguard"
name = "smallvec"
name = "strsim"
name = "unicode-bidi"
name = "unicode-general-category"
name = "unicode-segmentation"
name = "vte"
name = "winapi-i686-pc-windows-gnu"
name = "winapi-x86_64-pc-windows-gnu"
name = "winapi"
lock added:
name = "terminal_size"
== tier cli: only in main / only in proto
anstream v1.0.0;anstyle-parse v1.0.0;anstyle-query v1.1.5;colorchoice v1.0.5;is_terminal_polyfill v1.70.2;strsim v0.11.1;utf8parse v0.2.2;

== tier help: only in main / only in proto
ansi-str v0.9.0;ansitok v0.3.0;arrayvec v0.7.8;cfg-if v1.0.5;comfy-table v8.0.1;console v0.16.6;crossterm v0.29.0;lock_api v0.4.14;memchr v2.8.3;minimal-lexical v0.2.1;nom v7.1.3;parking_lot v0.12.5;parking_lot_core v0.9.12;scopeguard v1.2.0;smallvec v1.16.5;strsim v0.11.1;unicode-bidi v0.3.18;unicode-general-category v1.1.0;unicode-segmentation v1.13.3;vte v0.14.1;
terminal_size v0.4.4;
== tier view: only in main / only in proto
ansi-str v0.9.0;ansitok v0.3.0;arrayvec v0.7.8;cfg-if v1.0.5;comfy-table v8.0.1;console v0.16.6;crossterm v0.29.0;lock_api v0.4.14;minimal-lexical v0.2.1;nom v7.1.3;parking_lot v0.12.5;parking_lot_core v0.9.12;scopeguard v1.2.0;smallvec v1.16.5;strsim v0.11.1;unicode-bidi v0.3.18;unicode-general-category v1.1.0;unicode-segmentation v1.13.3;vte v0.14.1;
terminal_size v0.4.4;
== tier app: only in main / only in proto
ansi-str v0.9.0;ansitok v0.3.0;arrayvec v0.7.8;cfg-if v1.0.5;comfy-table v8.0.1;console v0.16.6;crossterm v0.29.0;lock_api v0.4.14;minimal-lexical v0.2.1;nom v7.1.3;parking_lot v0.12.5;parking_lot_core v0.9.12;scopeguard v1.2.0;smallvec v1.16.5;strsim v0.11.1;unicode-bidi v0.3.18;unicode-general-category v1.1.0;unicode-segmentation v1.13.3;vte v0.14.1;
terminal_size v0.4.4;
```

### Deltas

`narrow clap` is main with only the clap feature change the prototype also makes. `engine` is prototype minus narrow clap: the own table engine, the removal of `unicode-bidi` and `unicode-general-category`, and the added `terminal_size`.

| Tier | Δ file, prototype vs main | Δ __text, prototype vs main | Δ __text, narrow clap vs main | Δ __text, engine (prototype vs narrow clap) | Δ built pkgs, prototype vs main |
|:--|--:|--:|--:|--:|--:|
| cli | -66,368 | -62,016 | -62,016 | 0 | -7 |
| help | -200,560 | -105,996 | -48,560 | -57,436 | -19 |
| view | -200,592 | -117,264 | -60,212 | -57,052 | -18 |
| app | -200,544 | -105,624 | -48,244 | -57,380 | -18 |

### Clean build wall time

Each build starts from a removed target directory; dependencies are already in the local registry cache, so no network time is included. Times are `real` seconds from `/usr/bin/time -p`, with the profile of the Method section for release and cargo's default for dev. The host is shared and was not idle (see the load table below), so differences under about 10 percent are noise.

| Profile | Tier | main run 1 | main run 2 | prototype run 1 | prototype run 2 | Δ mean (prototype - main) | Mean CPU s (user+sys) main | Mean CPU s prototype | Δ CPU |
|:--|:--|--:|--:|--:|--:|--:|--:|--:|--:|
| release | cli | 5.58 | 5.07 | 5.13 | 4.75 | -0.39 s (-7%) | 8.5 | 7.1 | -16% |
| release | help | 6.54 | 6.38 | 6.35 | 5.90 | -0.33 s (-5%) | 15.5 | 10.1 | -35% |
| release | view | 7.13 | 7.01 | 7.12 | 6.47 | -0.28 s (-4%) | 19.0 | 13.3 | -30% |
| release | app | 7.73 | 7.42 | 7.91 | 7.00 | -0.12 s (-2%) | 19.6 | 14.2 | -27% |
| dev | app | 5.59 | 3.85 | 4.59 | 4.12 | -0.36 s (-8%) | 15.0 | 11.3 | -25% |

Load averages (1, 5, 15 minutes) read just before each build:

| Profile | Run | Variant | Tier | Load before |
|:--|:--|:--|:--|:--|
| release | 1 | main | cli | 4.72 7.95 6.73 |
| release | 1 | prototype | cli | 4.42 7.83 6.70 |
| release | 1 | main | help | 4.15 7.72 6.67 |
| release | 1 | prototype | help | 4.30 7.69 6.66 |
| release | 1 | main | view | 4.19 7.61 6.64 |
| release | 1 | prototype | view | 4.07 7.47 6.60 |
| release | 1 | main | app | 3.83 7.37 6.57 |
| release | 1 | prototype | app | 4.05 7.30 6.56 |
| release | 2 | main | cli | 3.89 7.21 6.53 |
| release | 2 | prototype | cli | 3.82 7.14 6.51 |
| release | 2 | main | help | 4.15 7.16 6.52 |
| release | 2 | prototype | help | 4.30 7.14 6.51 |
| release | 2 | main | view | 3.79 6.94 6.45 |
| release | 2 | prototype | view | 3.81 6.89 6.44 |
| release | 2 | main | app | 3.74 6.82 6.41 |
| release | 2 | prototype | app | 5.83 7.19 6.55 |
| dev | 1 | main | app | 5.44 7.08 6.52 |
| dev | 1 | prototype | app | 5.56 7.08 6.52 |
| dev | 2 | main | app | 5.60 7.06 6.52 |
| dev | 2 | prototype | app | 5.71 7.06 6.52 |

## Matrix C: consumers

`Pkgs` is the number of `Cargo.lock` entries with a registry or git source (workspace members and path packages excluded). `Built pkgs` is as in Matrix A.

| Consumer | SHA | ctl-core pin | Profile | File bytes | __text bytes | Pkgs | Built pkgs |
|:--|:--|:--|:--|--:|--:|--:|--:|
| verctl | [4175701520][verctl-sha] | =0.6.23 | own | 15,727,712 | 12,293,148 | 407 | 314 |
| verctl | [4175701520][verctl-sha] | =0.6.23 | common | 9,910,016 | 7,625,564 | 407 | 314 |
| qctl | [33a74b56af][qctl-sha] | =0.6.24 | own | 6,863,728 | 5,533,776 | 218 | 166 |
| qctl | [33a74b56af][qctl-sha] | =0.6.24 | common | 4,494,704 | 3,458,456 | 218 | 166 |
| forkctl | [7b987070f9][forkctl-sha] | =0.6.23 | own | 5,536,656 | 4,523,776 | 136 | 118 |
| forkctl | [7b987070f9][forkctl-sha] | =0.6.23 | common | 4,146,832 | 3,310,064 | 136 | 118 |
| rawr | [40abc93a3e][rawr-sha] | =0.6.28 | own | 1,233,216 | 950,920 | 120 | 93 |
| rawr | [40abc93a3e][rawr-sha] | =0.6.28 | common | 1,035,328 | 758,716 | 120 | 93 |

### Packages removed from the build graph by the patch

Darwin-arm64 normal and build dependencies, before and after the patch. `yamled` moved from 0.0.7 to 0.0.8 in verctl and forkctl because the prototype export requires 0.0.8; that is not part of the table engine change.

```text
rawr removed: ansi-str v0.9.0, ansitok v0.3.0, arrayvec v0.7.8, comfy-table v8.0.1, console v0.16.6, minimal-lexical v0.2.1, nom v7.1.3, unicode-bidi v0.3.18, unicode-general-category v1.1.0, vte v0.14.1
rawr added: terminal_size v0.4.4
verctl removed: ansi-str v0.9.0, ansitok v0.3.0, arrayvec v0.7.8, comfy-table v8.0.0, console v0.16.4, crossterm v0.29.0, minimal-lexical v0.2.1, nom v7.1.3, unicode-bidi v0.3.18, unicode-general-category v1.1.0, unicode-segmentation v1.13.3, vte v0.14.1, yamled v0.0.7
verctl added: terminal_size v0.4.4, yamled v0.0.8
qctl removed: ansi-str v0.9.0, ansitok v0.3.0, arrayvec v0.7.8, comfy-table v8.0.0, console v0.16.4, crossterm v0.29.0, minimal-lexical v0.2.1, nom v7.1.3, unicode-bidi v0.3.18, unicode-segmentation v1.13.3, vte v0.14.1
qctl added: terminal_size v0.4.4
forkctl removed: ansi-str v0.9.0, ansitok v0.3.0, arrayvec v0.7.8, comfy-table v8.0.0, console v0.16.4, crossterm v0.29.0, lock_api v0.4.14, minimal-lexical v0.2.1, nom v7.1.3, parking_lot v0.12.5, parking_lot_core v0.9.12, scopeguard v1.2.0, unicode-bidi v0.3.18, unicode-general-category v1.1.0, unicode-segmentation v1.13.3, vte v0.14.1
forkctl added: terminal_size v0.4.4, yamled v0.0.8
```

### Patched to the prototype

Each consumer was copied and given `[patch.crates-io] ctl-core = { path = ... }` pointing at the prototype export. A consumer pinned below 0.6.28 used a copy of the export with only its `version` rewritten to the pin.

| Consumer | Profile | Result | File bytes | __text bytes | Δ file | Δ __text | Pkgs | Δ built pkgs |
|:--|:--|:--|--:|--:|--:|--:|--:|--:|
| verctl | own | built | 15,594,080 | 12,249,052 | -133,632 | -44,096 | 390 | -11 |
| verctl | common | built | 9,810,848 | 7,595,592 | -99,168 | -29,972 | 390 | -11 |
| qctl | own | built | 6,729,984 | 5,488,144 | -133,744 | -45,632 | 205 | -10 |
| qctl | common | built | 4,428,544 | 3,426,944 | -66,160 | -31,512 | 205 | -10 |
| forkctl | own | built | 5,402,352 | 4,479,232 | -134,304 | -44,544 | 118 | -14 |
| forkctl | common | built | 4,047,360 | 3,278,996 | -99,472 | -31,068 | 118 | -14 |
| rawr | own | built | 1,150,656 | 922,396 | -82,560 | -28,524 | 109 | -9 |
| rawr | common | built | 952,672 | 734,536 | -82,656 | -24,180 | 109 | -9 |

## cargo bloat --crates -n 20

Built unstripped by cargo-bloat; the `.text` total matches `__text` of the stripped build to within 1 KiB.

### ctl-core main, `app` tier

```text
 File  .text     Size Crate
22.4%  43.7% 212.6KiB std
14.1%  27.4% 133.5KiB clap_builder
 5.9%  11.4%  55.6KiB ctl_core
 2.9%   5.7%  27.6KiB comfy_table
 0.9%   1.8%   8.7KiB crossterm
 0.7%   1.4%   6.7KiB toy
 0.6%   1.2%   6.0KiB unicode_width
 0.5%   0.9%   4.5KiB ansitok
 0.5%   0.9%   4.5KiB [Unknown]
 0.4%   0.8%   3.8KiB unicode_segmentation
 0.4%   0.7%   3.5KiB parking_lot_core
 0.4%   0.7%   3.5KiB litrs
 0.3%   0.7%   3.2KiB console
 0.2%   0.5%   2.3KiB parking_lot
 0.2%   0.4%   1.9KiB anstream
 0.2%   0.4%   1.9KiB serde_json
 0.2%   0.3%   1.6KiB proc_macro2
 0.2%   0.3%   1.4KiB anstyle
 0.1%   0.3%   1.4KiB heck
 0.1%   0.3%   1.3KiB ansi_str
 0.6%   1.1%   5.4KiB And 13 more crates. Use -n N to show more.
51.3% 100.0% 486.6KiB .text section size, the file size is 948.4KiB
```

### Prototype, `app` tier

```text
 File  .text     Size Crate
27.9%  51.9% 199.7KiB std
12.6%  23.5%  90.6KiB clap_builder
 7.9%  14.7%  56.4KiB ctl_core
 3.0%   5.6%  21.6KiB toy
 0.6%   1.2%   4.4KiB [Unknown]
 0.5%   0.9%   3.3KiB unicode_width
 0.4%   0.7%   2.7KiB proc_macro2
 0.3%   0.5%   2.0KiB serde_json
 0.3%   0.5%   1.9KiB anstream
 0.2%   0.4%   1.5KiB heck
 0.2%   0.4%   1.4KiB anstyle
 0.2%   0.4%   1.4KiB litrs
 0.1%   0.2%     960B clap_lex
 0.0%   0.1%     280B __rustc
 0.0%   0.1%     252B syn
 0.0%   0.0%      56B anyhow
 0.0%   0.0%      48B itoa
 0.0%   0.0%      20B bitflags
 0.0%   0.0%      20B serde_core
53.7% 100.0% 384.8KiB .text section size, the file size is 716.1KiB
```

### comfy-table (c)

```text
 File  .text     Size Crate
36.5%  75.6% 199.9KiB std
 5.1%  10.6%  28.1KiB comfy_table
 1.6%   3.4%   8.9KiB crossterm
 0.8%   1.7%   4.5KiB ansitok
 0.8%   1.7%   4.4KiB [Unknown]
 0.7%   1.4%   3.8KiB unicode_segmentation
 0.7%   1.4%   3.6KiB parking_lot_core
 0.6%   1.2%   3.3KiB unicode_width
 0.6%   1.2%   3.2KiB litrs
 0.6%   1.2%   3.2KiB console
 0.4%   0.9%   2.3KiB parking_lot
 0.2%   0.4%   1.2KiB toy
 0.2%   0.4%   1.0KiB ansi_str
 0.1%   0.2%     496B vte
 0.1%   0.2%     420B smallvec
 0.1%   0.1%     308B __rustc
 0.0%   0.1%     168B nom
48.3% 100.0% 264.2KiB .text section size, the file size is 547.0KiB
```

### tabled (a)

```text
 File  .text     Size Crate
32.7%  61.4% 183.1KiB std
 9.4%  17.7%  52.8KiB tabled
 7.9%  14.9%  44.5KiB papergrid
 2.4%   4.6%  13.7KiB toy
 0.8%   1.4%   4.3KiB [Unknown]
 0.2%   0.5%   1.4KiB heck
 0.2%   0.4%   1.1KiB proc_macro2
 0.1%   0.2%     560B unicode_width
 0.0%   0.1%     280B __rustc
 0.0%   0.1%     252B syn
53.2% 100.0% 298.2KiB .text section size, the file size is 560.8KiB
```

### Packages reachable only through comfy-table: verctl

```text
## only reachable through comfy-table (darwin-arm64, normal+build)
ansi-str v0.9.0
ansitok v0.3.0
arrayvec v0.7.8
comfy-table v8.0.0
console v0.16.4
crossterm v0.29.0
minimal-lexical v0.2.1
nom v7.1.3
unicode-segmentation v1.13.3
vte v0.14.1
total: 315 ; without comfy-table: 305
```

### Packages reachable only through comfy-table: qctl

```text
## only reachable through comfy-table (darwin-arm64, normal+build)
ansi-str v0.9.0
ansitok v0.3.0
arrayvec v0.7.8
bitflags v2.13.1
comfy-table v8.0.0
console v0.16.4
crossterm v0.29.0
errno v0.3.14
minimal-lexical v0.2.1
nom v7.1.3
rustix v1.1.4
unicode-segmentation v1.13.3
vte v0.14.1
total: 167 ; without comfy-table: 154
```

### Packages reachable only through comfy-table: forkctl

```text
## only reachable through comfy-table (darwin-arm64, normal+build)
ansi-str v0.9.0
ansitok v0.3.0
arrayvec v0.7.8
comfy-table v8.0.0
console v0.16.4
crossterm v0.29.0
lock_api v0.4.14
minimal-lexical v0.2.1
nom v7.1.3
parking_lot v0.12.5
parking_lot_core v0.9.12
scopeguard v1.2.0
unicode-segmentation v1.13.3
vte v0.14.1
total: 119 ; without comfy-table: 105
```

### Packages reachable only through comfy-table: rawr

```text
## only reachable through comfy-table (darwin-arm64, normal+build)
ansi-str v0.9.0
ansitok v0.3.0
arrayvec v0.7.8
comfy-table v8.0.1
console v0.16.6
minimal-lexical v0.2.1
nom v7.1.3
vte v0.14.1
total: 94 ; without comfy-table: 86
```

## Scripts

Export `WORK` as the working directory, `TMP` as a scratch directory and `CARGO_HOME` and `BLOAT_BIN` as the cargo and cargo-bloat directories before running these. The scripts are zsh.

### measure.sh

```zsh
#!/bin/zsh
# usage: measure.sh <label> <srcdir> <targetdir> [cargo build args...]
# env: BIN (binary name, default toy), TREE_ARGS (cargo tree args, default the build args)
# appends one TSV row to $WORK/results.tsv:
# label file_bytes text_bytes lock_total lock_local built_total nm_total nm_rust
set -u
label=$1; src=$2; tgt=$3; shift 3
root=$WORK
export RUSTUP_TOOLCHAIN=1.99.1
export CARGO_TARGET_DIR=$tgt
bin=$tgt/release/${BIN:-toy}
cd $src
if ! cargo build --release "$@" > $root/logs/$label.log 2>&1; then
  printf '%s\tFAIL\n' $label >> $root/results.tsv
  exit 0
fi
file=$(stat -f %z $bin)
text=$(size -m $bin | awk '/Section __text:/ {print $3; exit}')
total=$(rg -c '^name = ' Cargo.lock)
local_pkgs=$(awk 'BEGIN{RS="\n\n"} /^\[\[package\]\]/ && !/source = / {n++} END{print n+0}' Cargo.lock)
built=$(cargo tree --prefix none -f '{p}' -e normal,build --target aarch64-apple-darwin ${=TREE_ARGS:-"$@"} 2>/dev/null | awk '{print $1, $2}' | sort -u | wc -l | tr -d ' ')
nmtotal=$(nm $bin 2>&1 | wc -l)
nmrust=$(nm -U $bin 2>&1 | rg -c '_ZN|__R' || true)
printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' $label $file $text $total ${local_pkgs:-0} $built $nmtotal ${nmrust:-0} >> $root/results.tsv
```

### runA.sh

```zsh
#!/bin/zsh
# usage: runA.sh [toy names...]   (default: every toy under toys/)
root=$WORK
names=($@)
(( $# )) || names=($(ls $root/toys))
printf '%s\n' $names | xargs -P 3 -I{} $root/measure.sh {} $root/toys/{} $root/target/{}
```

### runB.sh

```zsh
#!/bin/zsh
# Matrix B: four ctl-core tiers against main and against the prototype.
root=$WORK
$root/measure.sh B-main-cli $root/toy-ctl $root/target/B-main-cli --no-default-features -F cli
cp $root/toy-ctl/Cargo.lock $root/toy-ctl-proto/Cargo.lock
printf '%s\n' main-help main-view main-app proto-cli proto-help proto-view proto-app | xargs -P 3 -I{} zsh -c '
  v=${1%%-*}; t=${1##*-}
  src=$WORK/toy-ctl; [[ $v == proto ]] && src=$WORK/toy-ctl-proto
  $WORK/measure.sh B-$v-$t $src $WORK/target/B-$v-$t --no-default-features -F $t' _ {}
```

### runC.sh

```zsh
#!/bin/zsh
# Matrix C: each consumer's release binary with its own profile and with the common profile.
root=$WORK
printf '%s\n' verctl-own qctl-own forkctl-own rawr-own verctl-common qctl-common forkctl-common rawr-common | xargs -P 3 -I{} zsh -c '
  name=${1%%-*}; kind=${1##*-}
  root=$WORK
  export BIN=$name TREE_ARGS="-p $name"
  if [[ $kind == common ]]; then
    export CARGO_PROFILE_RELEASE_OPT_LEVEL=s CARGO_PROFILE_RELEASE_LTO=true CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 CARGO_PROFILE_RELEASE_PANIC=abort CARGO_PROFILE_RELEASE_STRIP=true
  fi
  $root/measure.sh C-$name-$kind $root/consumers/$name $root/target/C-$name-$kind --bin $name' _ {}
```

### prepD.sh

```zsh
#!/bin/zsh
# Copies each consumer into consumers-proto/ and patches ctl-core to the prototype.
# The prototype copy for a consumer pinned below 0.6.28 has only its version rewritten.
root=$WORK
rm -rf $root/consumers-proto; mkdir -p $root/consumers-proto
typeset -A pin=(verctl 0.6.23 qctl 0.6.24 forkctl 0.6.23 rawr 0.6.28)
for n in verctl qctl forkctl rawr; do
  v=$pin[$n]
  proto=$root/ctl-core-proto
  if [[ $v != 0.6.28 ]]; then
    proto=$root/ctl-core-proto-$v
    rm -rf $proto
    cp -R $root/ctl-core-proto $proto
    sd '^version = "0.6.28"' "version = \"$v\"" $proto/Cargo.toml
  fi
  rsync -a --exclude target --exclude .git $root/consumers/$n/ $root/consumers-proto/$n/
  printf '\n[patch.crates-io]\nctl-core = { path = "%s" }\n' $proto >> $root/consumers-proto/$n/Cargo.toml
  rg -n "patch.crates-io" -A1 $root/consumers-proto/$n/Cargo.toml
done
```

### runD.sh

```zsh
#!/bin/zsh
# Matrix C patched builds (own and common profile) and the clap-narrowed ctl-core variant of Matrix B.
root=$WORK
printf '%s\n' verctl-own qctl-own forkctl-own rawr-own verctl-common qctl-common forkctl-common rawr-common | xargs -P 3 -I{} zsh -c '
  name=${1%%-*}; kind=${1##*-}
  root=$WORK
  export BIN=$name TREE_ARGS="-p $name"
  if [[ $kind == common ]]; then
    export CARGO_PROFILE_RELEASE_OPT_LEVEL=s CARGO_PROFILE_RELEASE_LTO=true CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 CARGO_PROFILE_RELEASE_PANIC=abort CARGO_PROFILE_RELEASE_STRIP=true
  fi
  $root/measure.sh D-$name-$kind $root/consumers-proto/$name $root/target/D-$name-$kind --bin $name' _ {}
printf '%s\n' cli help view app | xargs -P 3 -I{} $root/measure.sh B-clapnarrow-{} $root/toy-ctl-clapnarrow $root/target/B-clapnarrow-{} --no-default-features -F {}
```

### analyze.sh

```zsh
#!/bin/zsh
# usage: analyze.sh <consumer>   -> logs/analysis-<consumer>.txt
# Lists the packages reachable only through comfy-table, using cargo tree --prune.
export RUSTUP_TOOLCHAIN=1.99.1
root=$WORK
name=$1
cd $root/consumers/$name
cargo=cargo
flags=(-p $name -e normal,build --target aarch64-apple-darwin --prefix none -f '{p}')
out=$root/logs/analysis-$name.txt
{
  echo "## $name: comfy-table direct dependencies (darwin-arm64)"
  $cargo tree -p $name -i comfy-table -e normal,build --target aarch64-apple-darwin --depth 1 2>&1 | head -5
  $cargo tree --offline $flags -p comfy-table --depth 1 2>&1 | head -20
  for p in crossterm ansi-str ansitok nom vte unicode-segmentation console unicode-width unicode-bidi unicode-general-category; do
    echo; echo "## cargo tree -i $p --target all"
    $cargo tree --offline -p $name -i $p -e normal,build --target all --depth 3 2>&1 | head -25
  done
  echo; echo "## only reachable through comfy-table (darwin-arm64, normal+build)"
  $cargo tree --offline $flags 2>/dev/null | awk '{print $1, $2}' | sort -u > /re-$name-all.txt
  $cargo tree --offline $flags --prune comfy-table 2>/dev/null | awk '{print $1, $2}' | sort -u > /re-$name-pruned.txt
  comm -23 /re-$name-all.txt /re-$name-pruned.txt
  echo "total: $(wc -l < /re-$name-all.txt | tr -d ' ') ; without comfy-table: $(wc -l < /re-$name-pruned.txt | tr -d ' ')"
} > $out 2>&1
```

### bloat.sh

```zsh
#!/bin/zsh
# usage: bloat.sh <label> <srcdir> [cargo args...]   -> logs/bloat-<label>.txt
set -u
root=$WORK
export PATH=$CARGO_HOME/bin:$BLOAT_BIN:$PATH RUSTUP_TOOLCHAIN=1.99.1
label=$1; src=$2; shift 2
cd $src
CARGO_TARGET_DIR=$root/target/bloat-$label cargo bloat --release --crates -n 20 "$@" > $root/logs/bloat-$label.txt 2>&1
```

### timeit.sh

```zsh
#!/bin/zsh
# Clean build wall times for Matrix B. Appends to $WORK/times.tsv:
# profile run variant tier real user sys load-before
root=$WORK
export RUSTUP_TOOLCHAIN=1.99.1
cargo=cargo
one() {  # profile run variant tier
  local profile=$1 run=$2 v=$3 t=$4 src tgt flag tf=$root/logs/time.$$
  src=$root/toy-ctl; [[ $v == proto ]] && src=$root/toy-ctl-proto
  tgt=$root/target/T-$profile-$v-$t
  rm -rf $tgt
  flag=(); [[ $profile == release ]] && flag=(--release)
  local load=$(uptime | sed 's/.*load averages: //')
  (cd $src; CARGO_TARGET_DIR=$tgt /usr/bin/time -p -o $tf $cargo build -q $flag --no-default-features -F $t >/dev/null 2>&1)
  printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' $profile $run $v $t $(awk '/^real/ {print $2}' $tf) $(awk '/^user/ {print $2}' $tf) $(awk '/^sys/ {print $2}' $tf) "$load" >> $root/times.tsv
  rm -rf $tgt $tf
}
for run in 1 2; do
  for t in cli help view app; do
    for v in main proto; do one release $run $v $t; done
  done
done
for run in 1 2; do
  for v in main proto; do one dev $run $v app; done
done
```

## Toy sources

Every toy has this `Cargo.toml`, with its own `[dependencies]` table. The `main.rs` of each follows.

```toml
[package]
name = "toy"
version = "0.0.0"
edition = "2024"
publish = false

[dependencies]


[profile.release]
opt-level = "s"
lto = true
codegen-units = 1
panic = "abort"
strip = true

[workspace]
```

### base

```toml
(none)
```

```rust
fn main() {
    println!("{:<10} {:>5}", "name", 42);
    println!("{} {}", "second", 3.5);
}
```

### base-plain

```toml
(none)
```

```rust
fn main() {
    println!("name {}", 42);
    println!("second");
}
```

### tab-comfy-a

```toml
comfy-table = "8.0.1"
```

```rust
use comfy_table::{ContentArrangement, Table};

fn main() {
    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic).set_width(40);
    table.set_header(["id", "name", "note"]);
    table.add_row(["1", "日本語", "ready to ship 🚀 today and then some more words"]);
    table.add_row(["2", "alpha", "\x1b[1;31mfailed\x1b[0m"]);
    table.add_row(["3", "gamma", "ok"]);
    println!("{table}");
}
```

### tab-comfy-b

```toml
comfy-table = { version = "8.0.1", default-features = false }
```

```rust
use comfy_table::{ContentArrangement, Table};

fn main() {
    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic).set_width(40);
    table.set_header(["id", "name", "note"]);
    table.add_row(["1", "日本語", "ready to ship 🚀 today and then some more words"]);
    table.add_row(["2", "alpha", "\x1b[1;31mfailed\x1b[0m"]);
    table.add_row(["3", "gamma", "ok"]);
    println!("{table}");
}
```

### tab-comfy-c

```toml
comfy-table = { version = "8.0.1", default-features = false, features = ["custom_styling", "tty"] }
```

```rust
use comfy_table::{ContentArrangement, Table};

fn main() {
    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic).set_width(40);
    table.set_header(["id", "name", "note"]);
    table.add_row(["1", "日本語", "ready to ship 🚀 today and then some more words"]);
    table.add_row(["2", "alpha", "\x1b[1;31mfailed\x1b[0m"]);
    table.add_row(["3", "gamma", "ok"]);
    println!("{table}");
}
```

### tab-tabled-a

```toml
tabled = "0.22.0"
```

```rust
use tabled::builder::Builder;
use tabled::settings::peaker::Priority;
use tabled::settings::{Style, Width};

fn main() {
    let mut builder = Builder::default();
    builder.push_record(["id", "name", "note"]);
    builder.push_record(["1", "日本語", "ready to ship 🚀 today and then some more words"]);
    builder.push_record(["2", "alpha", "\x1b[1;31mfailed\x1b[0m"]);
    builder.push_record(["3", "gamma", "ok"]);
    let mut table = builder.build();
    table.with(Style::modern()).with(Width::wrap(40).priority(Priority::max(true)));
    println!("{table}");
}
```

### tab-tabled-b

```toml
tabled = { version = "0.22.0", default-features = false, features = ["std"] }
```

```rust
use tabled::builder::Builder;
use tabled::settings::peaker::Priority;
use tabled::settings::{Style, Width};

fn main() {
    let mut builder = Builder::default();
    builder.push_record(["id", "name", "note"]);
    builder.push_record(["1", "日本語", "ready to ship 🚀 today and then some more words"]);
    builder.push_record(["2", "alpha", "\x1b[1;31mfailed\x1b[0m"]);
    builder.push_record(["3", "gamma", "ok"]);
    let mut table = builder.build();
    table.with(Style::modern()).with(Width::wrap(40).priority(Priority::max(true)));
    println!("{table}");
}
```

### tab-tabled-c

```toml
tabled = { version = "0.22.0", default-features = false, features = ["std", "ansi"] }
```

```rust
use tabled::builder::Builder;
use tabled::settings::peaker::Priority;
use tabled::settings::{Style, Width};

fn main() {
    let mut builder = Builder::default();
    builder.push_record(["id", "name", "note"]);
    builder.push_record(["1", "日本語", "ready to ship 🚀 today and then some more words"]);
    builder.push_record(["2", "alpha", "\x1b[1;31mfailed\x1b[0m"]);
    builder.push_record(["3", "gamma", "ok"]);
    let mut table = builder.build();
    table.with(Style::modern()).with(Width::wrap(40).priority(Priority::max(true)));
    println!("{table}");
}
```

### tab-ratatui

```toml
ratatui-core = { version = "0.1.2", features = ["std"] }
unicode-width = "0.2.3"
ratatui-widgets = { version = "0.3.2", default-features = false, features = ["std"] }
```

```rust
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::{Constraint, Rect};
use ratatui_core::style::{Style, Stylize};
use ratatui_core::text::{Line, Span, Text};
use ratatui_core::widgets::Widget;
use ratatui_widgets::table::{Cell, Row, Table};
use unicode_width::UnicodeWidthStr;

fn main() {
    // ratatui cells do not wrap, so the long cell arrives pre-wrapped.
    let wrapped = Text::from(vec![Line::from("ready to ship 🚀 today and"), Line::from("then some more words")]);
    let rows = vec![
        Row::new([Cell::from("1"), Cell::from("日本語"), Cell::from(wrapped)]).height(2),
        Row::new([
            Cell::from("2"),
            Cell::from("alpha"),
            Cell::from(Span::styled("failed", Style::new().bold().red())),
        ]),
        Row::new([Cell::from("3"), Cell::from("gamma"), Cell::from("ok")]),
    ];
    let table = Table::new(rows, [Constraint::Length(3), Constraint::Length(8), Constraint::Fill(1)])
        .header(Row::new(["id", "name", "note"]).bold());
    let area = Rect::new(0, 0, 40, 5);
    let mut buf = Buffer::empty(area);
    table.render(area, &mut buf);
    for y in 0..area.height {
        let mut line = String::new();
        let mut x = 0;
        while x < area.width {
            let symbol = buf[(x, y)].symbol();
            line.push_str(symbol);
            x += u16::try_from(symbol.width()).unwrap_or(1).max(1);
        }
        println!("{}", line.trim_end());
    }
}
```

### tab-clitable

```toml
cli-table = "0.5.0"
```

```rust
use cli_table::{Cell, Color, Style, Table, print_stdout};

fn main() -> std::io::Result<()> {
    // cli-table has no width limit, so nothing wraps.
    let table = vec![
        vec!["1".cell(), "日本語".cell(), "ready to ship 🚀 today and then some more words".cell()],
        vec!["2".cell(), "alpha".cell(), "failed".cell().bold(true).foreground_color(Some(Color::Red))],
        vec!["3".cell(), "gamma".cell(), "ok".cell()],
    ]
    .table()
    .title(vec!["id".cell().bold(true), "name".cell().bold(true), "note".cell().bold(true)]);
    print_stdout(table)
}
```

### tab-prettytable

```toml
prettytable-rs = "0.10.0"
```

```rust
use prettytable::{Cell, Row, Table, row};

fn main() {
    // prettytable has no width limit, so nothing wraps.
    let mut table = Table::new();
    table.set_titles(row!["id", "name", "note"]);
    table.add_row(row!["1", "日本語", "ready to ship 🚀 today and then some more words"]);
    table.add_row(Row::new(vec![Cell::new("2"), Cell::new("alpha"), Cell::new("failed").style_spec("bFr")]));
    table.add_row(row!["3", "gamma", "ok"]);
    table.printstd();
}
```

### sty-anstyle

```toml
anstyle = "1.0.14"
```

```rust
use anstyle::{Ansi256Color, AnsiColor, Color, RgbColor, Style};

fn main() {
    let bold_red = Style::new().bold().fg_color(Some(Color::Ansi(AnsiColor::Red)));
    let c256 = Style::new().fg_color(Some(Color::Ansi256(Ansi256Color(208))));
    let rgb = Style::new().fg_color(Some(Color::Rgb(RgbColor(10, 200, 30))));
    for (style, word) in [(bold_red, "alert"), (c256, "orange"), (rgb, "green")] {
        println!("{}{}{}", style.render(), word, style.render_reset());
    }
}
```

### sty-anstyle-anstream

```toml
anstyle = "1.0.14"
anstream = "1.0.0"
```

```rust
use anstream::AutoStream;
use anstyle::{Ansi256Color, AnsiColor, Color, RgbColor, Style};
use std::io::Write;

fn main() -> std::io::Result<()> {
    let mut out = AutoStream::auto(std::io::stdout());
    let bold_red = Style::new().bold().fg_color(Some(Color::Ansi(AnsiColor::Red)));
    let c256 = Style::new().fg_color(Some(Color::Ansi256(Ansi256Color(208))));
    let rgb = Style::new().fg_color(Some(Color::Rgb(RgbColor(10, 200, 30))));
    for (style, word) in [(bold_red, "alert"), (c256, "orange"), (rgb, "green")] {
        writeln!(out, "{}{}{}", style.render(), word, style.render_reset())?;
    }
    Ok(())
}
```

### sty-owo

```toml
owo-colors = "4.4.0"
```

```rust
use owo_colors::{OwoColorize, XtermColors};

fn main() {
    println!("{}", "alert".bold().red());
    println!("{}", "orange".color(XtermColors::from(208u8)));
    println!("{}", "green".truecolor(10, 200, 30));
}
```

### sty-yansi

```toml
yansi = "1.0.1"
```

```rust
use yansi::Paint;

fn main() {
    println!("{}", "alert".bold().red());
    println!("{}", "orange".fixed(208));
    println!("{}", "green".rgb(10, 200, 30));
}
```

### sty-nuansi

```toml
nu-ansi-term = "0.50.3"
```

```rust
use nu_ansi_term::{Color, Style};

fn main() {
    println!("{}", Style::new().bold().fg(Color::Red).paint("alert"));
    println!("{}", Color::Fixed(208).paint("orange"));
    println!("{}", Color::Rgb(10, 200, 30).paint("green"));
}
```

### sty-console

```toml
console = "0.16.6"
```

```rust
use console::Style;

fn main() {
    println!("{}", Style::new().bold().red().force_styling(true).apply_to("alert"));
    println!("{}", Style::new().color256(208).force_styling(true).apply_to("orange"));
    println!("{}", Style::new().true_color(10, 200, 30).force_styling(true).apply_to("green"));
}
```

### sty-termcolor

```toml
termcolor = "1.4.1"
```

```rust
use std::io::Write;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

fn main() -> std::io::Result<()> {
    let mut out = StandardStream::stdout(ColorChoice::Always);
    for (spec, word) in [
        (ColorSpec::new().set_bold(true).set_fg(Some(Color::Red)).clone(), "alert"),
        (ColorSpec::new().set_fg(Some(Color::Ansi256(208))).clone(), "orange"),
        (ColorSpec::new().set_fg(Some(Color::Rgb(10, 200, 30))).clone(), "green"),
    ] {
        out.set_color(&spec)?;
        write!(out, "{word}")?;
        out.reset()?;
        writeln!(out)?;
    }
    Ok(())
}
```

### sty-crossterm

```toml
crossterm = { version = "0.29.0", default-features = false }
```

```rust
use crossterm::style::{Color, Stylize};

fn main() {
    println!("{}", "alert".bold().red());
    println!("{}", "orange".with(Color::AnsiValue(208)));
    println!("{}", "green".with(Color::Rgb { r: 10, g: 200, b: 30 }));
}
```

### wrap-uwidth

```toml
unicode-width = "0.2.3"
```

```rust
use unicode_width::UnicodeWidthStr;

const MIXED: &str = "Hello 日本語 🚀 e\u{301} 👨‍👩‍👧";
const PARAGRAPH: &str = "The quick brown fox jumps over the lazy dog while 日本語 and 🚀 share one line of text that must wrap.";

fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut used = 0;
    for word in text.split_whitespace() {
        let w = word.width();
        if used > 0 && used + 1 + w > width {
            lines.push(std::mem::take(&mut current));
            used = 0;
        }
        if used > 0 {
            current.push(' ');
            used += 1;
        }
        current.push_str(word);
        used += w;
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

fn main() {
    println!("{}", MIXED.width());
    for line in wrap(PARAGRAPH, 40) {
        println!("{line}");
    }
}
```

### wrap-uwidth-seg

```toml
unicode-width = "0.2.3"
unicode-segmentation = "1.13.3"
```

```rust
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

const MIXED: &str = "Hello 日本語 🚀 e\u{301} 👨‍👩‍👧";
const PARAGRAPH: &str = "The quick brown fox jumps over the lazy dog while 日本語 and 🚀 share one line of text that must wrap.";

fn width(text: &str) -> usize {
    text.graphemes(true).map(|g| g.width()).sum()
}

fn wrap(text: &str, limit: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut used = 0;
    for word in text.split_whitespace() {
        for chunk in split_graphemes(word, limit) {
            let w = width(&chunk);
            if used > 0 && used + 1 + w > limit {
                lines.push(std::mem::take(&mut current));
                used = 0;
            }
            if used > 0 {
                current.push(' ');
                used += 1;
            }
            current.push_str(&chunk);
            used += w;
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

/// Breaks a word wider than the limit on grapheme boundaries.
fn split_graphemes(word: &str, limit: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut used = 0;
    for g in word.graphemes(true) {
        let w = g.width();
        if used + w > limit {
            chunks.push(std::mem::take(&mut current));
            used = 0;
        }
        current.push_str(g);
        used += w;
    }
    chunks.push(current);
    chunks
}

fn main() {
    println!("{}", width(MIXED));
    for line in wrap(PARAGRAPH, 40) {
        println!("{line}");
    }
}
```

### wrap-textwrap-default

```toml
textwrap = "0.16.4"
```

```rust
const MIXED: &str = "Hello 日本語 🚀 e\u{301} 👨‍👩‍👧";
const PARAGRAPH: &str = "The quick brown fox jumps over the lazy dog while 日本語 and 🚀 share one line of text that must wrap.";

fn main() {
    println!("{}", textwrap::core::display_width(MIXED));
    for line in textwrap::wrap(PARAGRAPH, 40) {
        println!("{line}");
    }
}
```

### wrap-textwrap-uw

```toml
textwrap = { version = "0.16.4", default-features = false, features = ["unicode-width"] }
```

```rust
const MIXED: &str = "Hello 日本語 🚀 e\u{301} 👨‍👩‍👧";
const PARAGRAPH: &str = "The quick brown fox jumps over the lazy dog while 日本語 and 🚀 share one line of text that must wrap.";

fn main() {
    println!("{}", textwrap::core::display_width(MIXED));
    for line in textwrap::wrap(PARAGRAPH, 40) {
        println!("{line}");
    }
}
```

### term-terminal-size

```toml
terminal_size = "0.4.4"
```

```rust
fn main() {
    match terminal_size::terminal_size() {
        Some((w, h)) => println!("{} x {}", w.0, h.0),
        None => println!("no terminal"),
    }
}
```

### term-rustix

```toml
rustix = { version = "1.1.6", default-features = false, features = ["termios", "stdio"] }
```

```rust
fn main() {
    match rustix::termios::tcgetwinsize(unsafe { rustix::stdio::stdout() }) {
        Ok(size) => println!("{} x {}", size.ws_col, size.ws_row),
        Err(_) => println!("no terminal"),
    }
}
```

### term-rustix-std

```toml
rustix = { version = "1.1.6", default-features = false, features = ["termios", "std"] }
```

```rust
fn main() {
    match rustix::termios::tcgetwinsize(std::io::stdout()) {
        Ok(size) => println!("{} x {}", size.ws_col, size.ws_row),
        Err(_) => println!("no terminal"),
    }
}
```

### term-crossterm

```toml
crossterm = { version = "0.29.0", default-features = false }
```

```rust
fn main() {
    match crossterm::terminal::size() {
        Ok((w, h)) => println!("{w} x {h}"),
        Err(_) => println!("no terminal"),
    }
}
```

### term-std

```toml
(none)
```

```rust
use std::io::IsTerminal;

fn main() {
    println!("{}", std::io::stdout().is_terminal());
}
```

### term-supports-color

```toml
supports-color = "3.0.2"
```

```rust
use supports_color::Stream;

fn main() {
    match supports_color::on(Stream::Stdout) {
        Some(level) => println!("{} {} {}", level.has_basic, level.has_256, level.has_16m),
        None => println!("no color"),
    }
}
```

### term-anstyle-query

```toml
anstyle-query = "1.1.5"
```

```rust
fn main() {
    println!(
        "{} {} {} {}",
        anstyle_query::term_supports_color(),
        anstyle_query::term_supports_ansi_color(),
        anstyle_query::truecolor(),
        anstyle_query::no_color()
    );
}
```

### prog-indicatif

```toml
indicatif = "0.18.6"
```

```rust
use indicatif::{ProgressBar, ProgressDrawTarget};

fn main() {
    let bar = ProgressBar::with_draw_target(Some(10), ProgressDrawTarget::stderr());
    for _ in 0..10 {
        bar.inc(1);
    }
    bar.finish();
    println!("done");
}
```

### diag-miette

```toml
miette = { version = "7.6.0", features = ["fancy"] }
```

```rust
use miette::{GraphicalReportHandler, LabeledSpan, MietteDiagnostic, Report};

fn main() {
    let diagnostic = MietteDiagnostic::new("unknown key")
        .with_code("config::unknown_key")
        .with_help("check the spelling")
        .with_label(LabeledSpan::at(11..16, "not a valid key"));
    let report = Report::new(diagnostic).with_source_code("version: 1\nrepos: 2\n");
    let mut out = String::new();
    GraphicalReportHandler::new().render_report(&mut out, report.as_ref()).unwrap();
    println!("{out}");
}
```

### rich-fast-rich

```toml
fast-rich = "0.4.0"
```

```rust
use fast_rich::prelude::*;

fn main() {
    let console = Console::new();
    console.print("[bold red]failed[/] and [italic]styled text[/]");
    let mut table = Table::new();
    table.add_column("id");
    table.add_column("name");
    table.add_column("note");
    table.add_row_strs(&["1", "日本語", "ready to ship 🚀 today and then some more words"]);
    table.add_row_strs(&["2", "alpha", "ok"]);
    table.add_row_strs(&["3", "gamma", "ok"]);
    console.print_renderable(&table);
}
```

### rich-rich_rust

```toml
rich_rust = "0.2.3"
```

```rust
use rich_rust::prelude::*;

fn main() {
    let console = Console::new();
    console.print("[bold red]failed[/] and [italic]styled text[/]");
    let mut table = Table::new()
        .with_column(Column::new("id"))
        .with_column(Column::new("name"))
        .with_column(Column::new("note"));
    table.add_row_cells(["1", "日本語", "ready to ship 🚀 today and then some more words"]);
    table.add_row_cells(["2", "alpha", "ok"]);
    table.add_row_cells(["3", "gamma", "ok"]);
    console.print_renderable(&table);
}
```

### rich-rs-rich-default

```toml
rs-rich = "0.0.15"
```

```rust
use rich::{Console, Table};

fn main() {
    let console = Console::builder().build();
    console.print_str("[bold red]failed[/] and [italic]styled text[/]");
    let mut table = Table::new();
    table.add_column("id");
    table.add_column("name");
    table.add_column("note");
    table.add_row(&["1", "日本語", "ready to ship 🚀 today and then some more words"]);
    table.add_row(&["2", "alpha", "ok"]);
    table.add_row(&["3", "gamma", "ok"]);
    console.print(&table);
}
```

### rich-rs-rich-minimal

```toml
rs-rich = { version = "0.0.15", default-features = false }
```

```rust
use rich::{Console, Table};

fn main() {
    let console = Console::builder().build();
    console.print_str("[bold red]failed[/] and [italic]styled text[/]");
    let mut table = Table::new();
    table.add_column("id");
    table.add_column("name");
    table.add_column("note");
    table.add_row(&["1", "日本語", "ready to ship 🚀 today and then some more words"]);
    table.add_row(&["2", "alpha", "ok"]);
    table.add_row(&["3", "gamma", "ok"]);
    console.print(&table);
}
```

### rich-richrs

```toml
richrs = "0.2.1"
```

```rust
use richrs::prelude::*;

fn main() -> Result<()> {
    let mut console = Console::new();
    console.print("[bold red]failed[/] and [italic]styled text[/]")?;
    let mut table = Table::new();
    table.add_column(Column::new("id"));
    table.add_column(Column::new("name"));
    table.add_column(Column::new("note"));
    table.add_row_cells(["1", "日本語", "ready to ship 🚀 today and then some more words"]);
    table.add_row_cells(["2", "alpha", "ok"]);
    table.add_row_cells(["3", "gamma", "ok"]);
    console.write_segments(&table.render(40))?;
    Ok(())
}
```

### rich-rich-rs

```toml
rich-rs = "1.3.0"
```

```rust
use rich_rs::{Console, Table, Text};

fn main() {
    let mut console = Console::new();
    let styled = Text::from_markup("[bold red]failed[/] and [italic]styled text[/]", false).unwrap();
    console.print(&styled, None, None, None, false, "\n").unwrap();
    let mut table = Table::new();
    table.add_column_str("id");
    table.add_column_str("name");
    table.add_column_str("note");
    table.add_row_strs(&["1", "日本語", "ready to ship 🚀 today and then some more words"]);
    table.add_row_strs(&["2", "alpha", "ok"]);
    table.add_row_strs(&["3", "gamma", "ok"]);
    console.print(&table, None, None, None, false, "\n").unwrap();
}
```

### rich-rusty-rich-default

```toml
rusty-rich = "0.4.3"
```

```rust
use rusty_rich::{Column, Console, Table};

fn main() {
    let mut console = Console::new();
    console.print_str("[bold red]failed[/] and [italic]styled text[/]");
    let mut table = Table::new();
    table.add_column(Column::new("id"));
    table.add_column(Column::new("name"));
    table.add_column(Column::new("note"));
    table.add_row_str(vec!["1".into(), "日本語".into(), "ready to ship 🚀 today and then some more words".into()]);
    table.add_row_str(vec!["2".into(), "alpha".into(), "ok".into()]);
    table.add_row_str(vec!["3".into(), "gamma".into(), "ok".into()]);
    console.println(&table);
}
```

### rich-rusty-rich-minimal

```toml
rusty-rich = { version = "0.4.3", default-features = false }
```

```rust
use rusty_rich::{Column, Console, Table};

fn main() {
    let mut console = Console::new();
    console.print_str("[bold red]failed[/] and [italic]styled text[/]");
    let mut table = Table::new();
    table.add_column(Column::new("id"));
    table.add_column(Column::new("name"));
    table.add_column(Column::new("note"));
    table.add_row_str(vec!["1".into(), "日本語".into(), "ready to ship 🚀 today and then some more words".into()]);
    table.add_row_str(vec!["2".into(), "alpha".into(), "ok".into()]);
    table.add_row_str(vec!["3".into(), "gamma".into(), "ok".into()]);
    console.println(&table);
}
```

### rich-richterm

```toml
richterm = "0.5.0"
```

```rust
use richterm::{print, text};

// richterm has styled text only; it has no table.
fn main() {
    print([
        text("failed", "fg:red eff:b"),
        text(" and ", ""),
        text("styled text", "eff:i"),
        text("\n", ""),
    ]);
}
```

### rich-wealthy

```toml
wealthy = "0.2.0"
```

```rust
use wealthy::cells::cell_len;

// wealthy ports only cell widths, colour triplets and spinner names; it has no table or console.
fn main() {
    println!("{}", cell_len("Hello 日本語 🚀"));
}
```

### toy-ctl (Matrix B)

```toml
[package]
name = "toy"
version = "0.0.0"
edition = "2024"
publish = false

[features]
cli = ["ctl-core/cli"]
help = ["cli", "ctl-core/help"]
view = ["cli", "ctl-core/view", "dep:serde"]
app = ["view", "ctl-core/app"]

[dependencies]
ctl-core = { path = "../ctl-core", default-features = false }
clap = { version = "4", default-features = false, features = ["std", "derive"] }
serde = { version = "1", features = ["derive"], optional = true }

[profile.release]
codegen-units = 1
lto = true
panic = "abort"
strip = true
opt-level = "s"
```

```rust
use ctl_core::prelude::*;

#[derive(Parser)]
#[command(version, about = "toy")]
struct Cli {
    #[command(flatten)]
    output: OutputArgs,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Show status.
    Status {
        /// Name to show.
        name: Option<String>,
    },
}

#[cfg(feature = "view")]
#[derive(serde::Serialize)]
struct Status {
    name: String,
    pending: usize,
}

#[cfg(feature = "view")]
impl Present for Status {
    fn present(&self) -> Document {
        Document::new()
            .fields(Fields::new().row("name", self.name.clone()).row("pending", self.pending.to_string()))
            .table(Table::new(["id", "title"]).row(["T-1", "first row"]).row(["T-2", "second row"]))
    }
}

#[cfg(feature = "app")]
fn main() -> ExitCode {
    App::<Cli>::new("toy").view(|cli| cli.output.view()).run(|cli| match cli.command {
        Command::Status { name } => Ok(Status { name: name.unwrap_or_default(), pending: 1 }),
    })
}

#[cfg(all(feature = "view", not(feature = "app")))]
fn main() -> ExitCode {
    let cli = Cli::parse();
    let view = cli.output.view();
    let Command::Status { name } = cli.command;
    view.show(&Status { name: name.unwrap_or_default(), pending: 1 }).unwrap_or(ExitCode::FAILURE)
}

#[cfg(all(feature = "help", not(feature = "view")))]
fn main() -> ExitCode {
    if ctl_core::help::try_emit::<Cli>().unwrap_or(false) {
        return ExitCode::SUCCESS;
    }
    let cli = Cli::parse();
    let Command::Status { name } = cli.command;
    println!("{} {:?}", name.unwrap_or_default(), cli.output.format);
    ExitCode::SUCCESS
}

#[cfg(all(feature = "cli", not(feature = "help"), not(feature = "view")))]
fn main() -> ExitCode {
    let cli = Cli::parse();
    let Command::Status { name } = cli.command;
    println!("{} {:?}", name.unwrap_or_default(), cli.output.format);
    ExitCode::SUCCESS
}
```

[main-sha]: https://github.com/victor-software-house/ctl-core/commit/7dc8060e2578af605f4ae858947693cb52477d6f
[verctl-sha]: https://github.com/victor-software-house/verctl/commit/41757015205f335135dfc84274b0330bcb9be4b0
[qctl-sha]: https://github.com/victor-software-house/qctl/commit/33a74b56af84f5ce237a8468fbe7a3b26f8081ec
[forkctl-sha]: https://github.com/victor-software-house/forkctl/commit/7b987070f9069842c2e9afdd17fca28353b81bc5
[rawr-sha]: https://github.com/victor-software-house/rawr/commit/40abc93a3ef4bb293540bfce6320a69d4ebbc5d9
