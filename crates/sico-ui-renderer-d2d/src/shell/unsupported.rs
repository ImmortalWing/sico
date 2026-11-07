//! The non-Windows window shell stub.
//!
//! The native window shell is a Windows implementation (ADR-0018 §2). On any
//! other platform every operation refuses with a typed error instead of
//! pretending to own a window.

use crate::error::UiBackendError;
use crate::events::WindowEvent;
use crate::frame::Frame;

/// The non-Windows shell: every method refuses.
pub(crate) struct NoShell;

impl NoShell {
    /// Refuses to create a shell.
    pub(crate) fn new() -> Result<Self, UiBackendError> {
        Err(UiBackendError::UnsupportedPlatform)
    }

    /// Refuses to create a window.
    pub(crate) fn create_window(
        &mut self,
        _title: &str,
        _width: u32,
        _height: u32,
    ) -> Result<(), UiBackendError> {
        Err(UiBackendError::UnsupportedPlatform)
    }

    /// Refuses to pump.
    pub(crate) fn pump_once(&mut self) -> Option<WindowEvent> {
        None
    }

    /// Refuses to drain.
    pub(crate) fn drain(&mut self, _max_events: usize) -> (Vec<WindowEvent>, bool) {
        (Vec::new(), false)
    }

    /// Refuses to present.
    pub(crate) fn present_frame(&mut self, _frame: &Frame) -> Result<(), UiBackendError> {
        Err(UiBackendError::UnsupportedPlatform)
    }

    /// Refuses to read back.
    pub(crate) fn read_presented_frame(&self) -> Result<Frame, UiBackendError> {
        Err(UiBackendError::UnsupportedPlatform)
    }

    /// Refuses to close.
    pub(crate) fn close(&self) -> Result<(), UiBackendError> {
        Err(UiBackendError::UnsupportedPlatform)
    }

    /// The loop never quits on a host with no window.
    pub(crate) fn should_quit(&self) -> bool {
        false
    }

    /// Refuses to report a client size.
    pub(crate) fn client_size(&self) -> Result<(u32, u32), UiBackendError> {
        Err(UiBackendError::UnsupportedPlatform)
    }

    /// Refuses to destroy.
    pub(crate) fn destroy(&mut self) -> Result<(), UiBackendError> {
        Err(UiBackendError::UnsupportedPlatform)
    }
}
