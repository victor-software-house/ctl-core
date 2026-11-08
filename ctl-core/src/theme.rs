//! The styles a [`Document`](crate::Document) is drawn in.
//!
//! A document says what each span means through its [`Role`]; the theme says
//! what that meaning looks like. The renderer applies the theme when a
//! document becomes widgets, so the layout crates never see a role, and the
//! writer decides afterwards whether to write the styles at all.

use ctl_text::Style;
use ctl_text::anstyle::{AnsiColor, Color};

use crate::document::{NoticeLevel, Role};

/// A bold span in one of the 16 named colours.
const fn bold(color: AnsiColor) -> Style {
    Style::new().bold().fg_color(Some(Color::Ansi(color)))
}

/// Maps each [`Role`], and the label of each [`NoticeLevel`], to an
/// [`anstyle::Style`](ctl_text::anstyle::Style).
///
/// [`Theme::default`] is ctl-core's palette. It uses only the 16 named
/// colours and bold, so it looks the same at every colour depth:
///
/// | Role | Style |
/// |:--|:--|
/// | [`Role::Plain`] | none |
/// | [`Role::Heading`] | bold cyan |
/// | [`Role::Success`] | bold green |
/// | [`Role::Warning`] | bold yellow |
/// | [`Role::Error`] | bold red |
/// | [`Role::Value`] | bold yellow |
/// | [`Role::Muted`] | bright black |
/// | [`Role::Token`] | bold green |
/// | [`Role::Id`] | bold, in the terminal's own foreground |
///
/// A notice label takes the style of the role of the same name until
/// [`Theme::with_notice_style`] replaces it.
///
/// Equality compares the choices a theme holds, including which notice labels
/// were given their own style, not the output they draw: a label set to its
/// role's style draws the same as an unset one, yet the two themes differ.
///
/// A theme replaces the default through [`RenderOptions::with_theme`],
/// [`View::with_theme`], and [`App::theme`]. It only says how a role looks:
/// whether colour is written is the colour mode's decision, so
/// [`ColorMode::Never`] writes none whatever the theme holds.
///
/// ```
/// use ctl_core::text::anstyle::{AnsiColor, Style};
/// use ctl_core::{Role, Theme};
///
/// let magenta = Style::new().fg_color(Some(AnsiColor::Magenta.into()));
/// let theme = Theme::default().with_style(Role::Id, magenta);
/// assert_eq!(theme.style(Role::Id), magenta);
/// assert_eq!(
///     theme.style(Role::Heading),
///     Theme::default().style(Role::Heading)
/// );
/// ```
///
/// [`RenderOptions::with_theme`]: crate::RenderOptions::with_theme
/// [`View::with_theme`]: crate::View::with_theme
/// [`App::theme`]: crate::App::theme
/// [`ColorMode::Never`]: crate::ColorMode::Never
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Theme {
    plain: Style,
    heading: Style,
    success: Style,
    warning: Style,
    error: Style,
    value: Style,
    muted: Style,
    token: Style,
    id: Style,
    /// A notice label's own style, or `None` to take its role's.
    success_notice: Option<Style>,
    warning_notice: Option<Style>,
    error_notice: Option<Style>,
}

impl Theme {
    /// ctl-core's palette, the table in the [type docs](Self).
    ///
    /// The same as [`Theme::default`], usable in a `const`.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            plain: Style::new(),
            heading: bold(AnsiColor::Cyan),
            success: bold(AnsiColor::Green),
            warning: bold(AnsiColor::Yellow),
            error: bold(AnsiColor::Red),
            value: bold(AnsiColor::Yellow),
            muted: Style::new().fg_color(Some(Color::Ansi(AnsiColor::BrightBlack))),
            token: bold(AnsiColor::Green),
            id: Style::new().bold(),
            success_notice: None,
            warning_notice: None,
            error_notice: None,
        }
    }

    /// The style `role` is drawn in.
    #[must_use]
    pub const fn style(self, role: Role) -> Style {
        match role {
            Role::Plain => self.plain,
            Role::Heading => self.heading,
            Role::Success => self.success,
            Role::Warning => self.warning,
            Role::Error => self.error,
            Role::Value => self.value,
            Role::Muted => self.muted,
            Role::Token => self.token,
            Role::Id => self.id,
        }
    }

    /// This theme with `role` drawn in `style`.
    #[must_use]
    pub const fn with_style(mut self, role: Role, style: Style) -> Self {
        match role {
            Role::Plain => self.plain = style,
            Role::Heading => self.heading = style,
            Role::Success => self.success = style,
            Role::Warning => self.warning = style,
            Role::Error => self.error = style,
            Role::Value => self.value = style,
            Role::Muted => self.muted = style,
            Role::Token => self.token = style,
            Role::Id => self.id = style,
        }
        self
    }

    /// The style the label of a `level` notice (`success`, `warning`, or
    /// `error`) is drawn in: the style [`Theme::with_notice_style`] set, or
    /// else the style of the role of the same name.
    #[must_use]
    pub const fn notice_style(self, level: NoticeLevel) -> Style {
        let (own, role) = match level {
            NoticeLevel::Success => (self.success_notice, self.success),
            NoticeLevel::Warning => (self.warning_notice, self.warning),
            NoticeLevel::Error => (self.error_notice, self.error),
        };
        match own {
            Some(style) => style,
            None => role,
        }
    }

    /// This theme with the label of a `level` notice drawn in `style`.
    #[must_use]
    pub const fn with_notice_style(mut self, level: NoticeLevel, style: Style) -> Self {
        match level {
            NoticeLevel::Success => self.success_notice = Some(style),
            NoticeLevel::Warning => self.warning_notice = Some(style),
            NoticeLevel::Error => self.error_notice = Some(style),
        }
        self
    }
}

impl Default for Theme {
    /// ctl-core's palette; see [`Theme::new`].
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use ctl_text::Style;
    use ctl_text::anstyle::{AnsiColor, Color, Effects};

    use super::Theme;
    use crate::document::{NoticeLevel, Role};

    #[test]
    fn the_default_palette_is_bold_named_colours() {
        let theme = Theme::default();
        let cyan = Style::new()
            .bold()
            .fg_color(Some(Color::Ansi(AnsiColor::Cyan)));
        assert_eq!(theme.style(Role::Heading), cyan);
        assert_eq!(theme.style(Role::Plain), Style::new());
        assert_eq!(theme.style(Role::Id), Style::new().bold());
        assert_eq!(
            theme.notice_style(NoticeLevel::Error),
            theme.style(Role::Error)
        );
    }

    #[test]
    fn replacing_one_role_keeps_the_others() {
        let underline = Style::new().effects(Effects::UNDERLINE);
        let theme = Theme::new().with_style(Role::Muted, underline);
        assert_eq!(theme.style(Role::Muted), underline);
        assert_eq!(theme.style(Role::Value), Theme::new().style(Role::Value));
        assert_eq!(
            theme.notice_style(NoticeLevel::Warning),
            Theme::new().notice_style(NoticeLevel::Warning)
        );
    }

    #[test]
    fn a_notice_label_has_its_own_style() {
        let plain = Theme::new().with_notice_style(NoticeLevel::Success, Style::new());
        assert_eq!(plain.notice_style(NoticeLevel::Success), Style::new());
        assert_eq!(
            plain.style(Role::Success),
            Theme::new().style(Role::Success)
        );
    }

    #[test]
    fn a_notice_label_follows_its_role_until_given_its_own_style() {
        let magenta = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Magenta)));
        let follows = Theme::new().with_style(Role::Error, magenta);
        assert_eq!(follows.notice_style(NoticeLevel::Error), magenta);

        let underline = Style::new().effects(Effects::UNDERLINE);
        let own = Theme::new()
            .with_notice_style(NoticeLevel::Error, underline)
            .with_style(Role::Error, magenta);
        assert_eq!(own.notice_style(NoticeLevel::Error), underline);
        assert_eq!(own.style(Role::Error), magenta);
    }
}
