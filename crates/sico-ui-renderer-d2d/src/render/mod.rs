//! The platform renderer.
//!
//! The renderer is split so that the unsafe Direct2D code lives in exactly
//! one module and the rest of the crate stays platform-neutral.

#[cfg(not(windows))]
mod unsupported;
#[cfg(windows)]
mod win;

use crate::error::UiBackendError;
use crate::frame::Frame;
use crate::tree::UiTree;

/// Renders one frame of the tree at the requested client size.
pub(crate) fn render_frame(
    tree: &UiTree,
    width: u32,
    height: u32,
) -> Result<Frame, UiBackendError> {
    #[cfg(windows)]
    {
        win::render_frame(tree, width, height)
    }
    #[cfg(not(windows))]
    {
        unsupported::render_frame(tree, width, height)
    }
}
