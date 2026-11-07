//! The typed refusal surface of the renderer adapter.
//!
//! The guest-visible subset mirrors the `ui@1` `ui-error` contract
//! (`unknown-handle`, `wrong-parent-kind`, `limit`, `no-window`); the
//! remaining variants are Host-internal failures of the renderer itself and
//! never reach a guest as a support claim.

/// A typed renderer failure.
///
/// The renderer never traps, panics or silently falls back: every failure
/// path is one of these variants, and `limit` is always reported before the
/// offending allocation happens.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UiBackendError {
    /// The renderer is not available on this platform (non-Windows host).
    UnsupportedPlatform,
    /// The handle does not refer to a live node.
    UnknownHandle,
    /// The node cannot be a child of the requested parent kind.
    WrongParentKind,
    /// A declared budget was exceeded (node count, tree depth, text size,
    /// option count, image size).
    Limit {
        /// Which budget was hit.
        which: &'static str,
    },
    /// No window root exists yet.
    NoWindow,
    /// The frame could not be rendered.
    RenderFailed {
        /// Host-internal detail; never guest text.
        detail: String,
    },
    /// An argument was outside the accepted range.
    InvalidArgument {
        /// Which argument was rejected.
        which: &'static str,
    },
}

impl std::fmt::Display for UiBackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedPlatform => f.write_str("ui-renderer: unsupported platform"),
            Self::UnknownHandle => f.write_str("ui-renderer: unknown handle"),
            Self::WrongParentKind => f.write_str("ui-renderer: wrong parent kind"),
            Self::Limit { which } => write!(f, "ui-renderer: limit exceeded ({which})"),
            Self::NoWindow => f.write_str("ui-renderer: no window"),
            Self::RenderFailed { detail } => write!(f, "ui-renderer: render failed ({detail})"),
            Self::InvalidArgument { which } => {
                write!(f, "ui-renderer: invalid argument ({which})")
            }
        }
    }
}

impl std::error::Error for UiBackendError {}
