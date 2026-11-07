//! The non-Windows renderer stub.
//!
//! The native renderer is a Windows platform implementation (ADR-0018 §2).
//! On any other platform the renderer refuses with a typed error instead of
//! pretending to render.

use crate::error::UiBackendError;
use crate::frame::Frame;
use crate::tree::UiTree;

/// Refuses to render on a non-Windows host.
pub(crate) fn render_frame(
    _tree: &UiTree,
    _width: u32,
    _height: u32,
) -> Result<Frame, UiBackendError> {
    Err(UiBackendError::UnsupportedPlatform)
}
