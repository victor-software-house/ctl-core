//! A handler error that names its exit code and its kind.

use std::error::Error;
use std::fmt;
use std::num::NonZeroU8;

/// A failure a handler returns when the failure has its own exit code, or a
/// kind a script can match on, such as `not_found`.
///
/// Return it through `anyhow`, as the error itself or as context on its
/// cause. [`App`](super::App) looks for it in the error's chain, at any
/// depth. It exits with the code, and a JSON envelope carries the kind in
/// [`ErrorBody::kind`](crate::model::ErrorBody::kind). The message is the
/// whole chain, as for any other error. A handler error that carries none
/// exits 1 with no kind.
///
/// ```
/// use std::num::NonZeroU8;
///
/// use anyhow::Context as _;
/// use ctl_core::app::CommandError;
///
/// const NO_ROOM: NonZeroU8 = NonZeroU8::new(75).unwrap();
///
/// let refused = CommandError::new("the budget has no room")
///     .kind("no_room")
///     .exit_code(NO_ROOM);
/// assert_eq!(refused.to_string(), "the budget has no room");
///
/// let read: anyhow::Result<()> = Err(anyhow::anyhow!("connection refused"));
/// let error = read
///     .context(CommandError::new("cannot reach the daemon").kind("unreachable"))
///     .unwrap_err();
/// assert_eq!(
///     format!("{error:#}"),
///     "cannot reach the daemon: connection refused"
/// );
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandError {
    message: String,
    kind: Option<String>,
    exit_code: NonZeroU8,
}

impl CommandError {
    /// A failure with `message`, exit code 1, and no kind.
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind: None,
            exit_code: NonZeroU8::MIN,
        }
    }

    /// Set the kind a JSON envelope carries, such as `not_found`.
    #[must_use]
    pub fn kind(mut self, kind: impl Into<String>) -> Self {
        self.kind = Some(kind.into());
        self
    }

    /// Set the exit code. It cannot be 0, so a failure never exits as a
    /// success.
    #[must_use]
    pub const fn exit_code(mut self, code: NonZeroU8) -> Self {
        self.exit_code = code;
        self
    }

    /// The first `CommandError` in `error`'s chain, as the error itself,
    /// as context at any depth, or as the source of another error.
    pub(crate) fn find(error: &anyhow::Error) -> Option<&Self> {
        error
            .downcast_ref::<Self>()
            .or_else(|| error.chain().find_map(|cause| cause.downcast_ref::<Self>()))
    }

    /// The kind, as [`Self::kind`] set it.
    pub(crate) fn kind_name(&self) -> Option<&str> {
        self.kind.as_deref()
    }

    /// The exit code, as [`Self::exit_code`] set it, or 1.
    pub(crate) const fn code(&self) -> NonZeroU8 {
        self.exit_code
    }
}

/// The message alone; `{error:#}` on the `anyhow` error adds its cause.
impl fmt::Display for CommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for CommandError {}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU8;

    use anyhow::{Context as _, anyhow};

    use super::CommandError;

    #[test]
    fn find_reads_the_error_its_context_and_a_deeper_context() {
        let code = NonZeroU8::new(75).unwrap();
        let refused = CommandError::new("no room").kind("no_room").exit_code(code);
        let direct = anyhow::Error::new(refused.clone());
        assert_eq!(CommandError::find(&direct), Some(&refused));
        let wrapped = Err::<(), _>(anyhow!("refused"))
            .context(refused.clone())
            .context("while making a box")
            .unwrap_err();
        assert_eq!(CommandError::find(&wrapped), Some(&refused));
        assert_eq!(CommandError::find(&anyhow!("plain")), None);
        assert_eq!(refused.kind_name(), Some("no_room"));
        assert_eq!(refused.code(), code);
        assert_eq!(CommandError::new("x").code(), NonZeroU8::MIN);
    }
}
