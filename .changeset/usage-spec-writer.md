---
ctl-core: patch
---

`--usage-spec` writes the Usage KDL with ctl-core's own writer, built from the clap `Command`.
The spec keeps usage-lib 7.3.0's bytes, except where they were not valid KDL v2 or read back as other text.
Each of those cases is now a fix:

- Help text or a default value that holds U+2028, U+2029, or U+0085 is written with `\u{…}` escapes.
  KDL v2 counts these characters as newlines, so mise could not read the spec, or read U+2028 back as a line feed.
- Bidi controls and the byte order mark in help text or a default value are written with `\u{…}` escapes.
  They are U+200E, U+200F, U+202A to U+202E, U+2066 to U+2069, and U+FEFF.
  So is ESC in a default value.
  KDL v2 forbids these characters literally.
- A default value that holds U+007F (DEL) is quoted and escaped.
  It was written bare, which KDL v2 forbids, and parsed only because the parsers miss that rule.
- Help text with a line of only whitespace is written as a quoted string.
  A raw multi-line string read that line back empty.
- A default value or possible value `-.5` or `+.5` is quoted.
  Written bare, it starts like a number, and KDL v2 refuses it.
