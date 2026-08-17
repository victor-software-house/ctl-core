# Mark

Two discs, lapped: the shared core carried by its consumer, descending left to
right like the rest of the family. `ctl-core` is the one mark here allowed to be
round — `verctl` is circles, `forkctl` and `qctl` are rectangles, and the crate
they all compile against is the only place both geometries meet. Same palette
and 32-unit grid as the three CLIs.

| File | Use |
|:--|:--|
| [`mark.svg`](mark.svg) | Square mark, cream field. Avatar, favicon, docs. |
| [`mark-dark.svg`](mark-dark.svg) | Same geometry, ink field. |
| [`banner.svg`](banner.svg) | README header, 1200×240. |
| [`banner-dark.svg`](banner-dark.svg) | Same, ink field. |

Pair the two fields with `<picture>` and `prefers-color-scheme`, as the README
header does. Never recolour a single file at the call site.

## Palette

| | Hex | Role |
|:--|:--|:--|
| Cream | `#f3efe6` | Field, or figure on ink |
| Ink | `#161616` | Figure, or field |
| Rust | `#c45c2a` | The consumer that carries the core. One accent, never two |

Banner-only tints: `#6f675c` (muted on cream), `#8d857a` (muted on ink),
`#ddd6c8` / `#2f2f2f` (hairline).

## Construction

A 32-unit square, corner radius 6. Two discs of r 8, centred `12.5,12.5` and
`19.5,19.5`. The centres sit on `y = x`, 7 units apart on each axis, so the
figure is symmetric about the square's diagonal and clears the field by 4.5
units on every side. Rust is drawn second and therefore in front; the overlap is
the whole subject, so do not reduce it to a tangent.

Same discipline as the other three marks: parts of one colour belong in one
`<path>` or one shape. Abutting `<rect>`s each antialias against the field, so a
shared edge composites to roughly 75% coverage and shows as a grey hairline —
measured on the `forkctl` mark, `srgb(77,76,74)` where it should be
`srgb(22,22,22)`.

## Banner text

Set in [Geist Mono](https://github.com/vercel/geist-font) (OFL) and converted
to outlines, so nothing depends on a font at render time: wordmark Black 60px
with −3 tracking, tagline Medium 17px, chip Regular 16px. To change the
wording, reshape with `fonttools` + `uharfbuzz` at those sizes rather than
adding a `<text>` element.
