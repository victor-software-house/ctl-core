//! Combining two styles.

use anstyle::Style;

/// `base` with `top` painted over it.
///
/// A colour `top` sets (foreground, background, or underline) replaces the one
/// in `base`; a colour it leaves unset keeps `base`'s. Effects add up, because
/// [`anstyle::Effects`] has no way to say "not bold": `top` can switch an
/// effect on but not off.
#[must_use]
pub fn overlay(base: Style, top: Style) -> Style {
    Style::new()
        .fg_color(top.get_fg_color().or_else(|| base.get_fg_color()))
        .bg_color(top.get_bg_color().or_else(|| base.get_bg_color()))
        .underline_color(
            top.get_underline_color()
                .or_else(|| base.get_underline_color()),
        )
        .effects(base.get_effects() | top.get_effects())
}
