//! The renderer API.
//!
//! The surface mirrors the `ui@1` host-provider shape so the adapter can be
//! wired onto the runner seam without changing the guest contract: the same
//! thirteen operations, the same handle discipline, the same typed refusals.
//!
//! Authority is **not** part of this crate. The adapter renders what it is
//! given; the grant decision, the capability check and the audit trail stay
//! in the runner and the desktop host (ADR-0018 §4, RFC-0052 §3).

use std::sync::Mutex;

use crate::error::UiBackendError;
use crate::frame::Frame;
use crate::tree::{NodeKind, UiTree};
use crate::{render, tokens};

/// Maximum UTF-8 length of a window title.
pub const MAX_TITLE_BYTES: usize = 256;

/// The window the renderer draws into.
#[derive(Clone, Debug)]
struct WindowSpec {
    title: String,
    width: u32,
    height: u32,
}

#[derive(Debug, Default)]
struct State {
    tree: UiTree,
    window: Option<WindowSpec>,
}

/// The Fluent-subset Direct2D renderer.
///
/// The backend owns the UI tree and nothing else: no COM objects are stored,
/// so it is `Send + Sync` and every frame is rebuilt from scratch, which is
/// what keeps a frame a pure function of the tree (ADR-0018 §3).
#[derive(Debug, Default)]
pub struct D2dUiBackend {
    state: Mutex<State>,
}

impl D2dUiBackend {
    /// Creates an empty renderer.
    ///
    /// # Errors
    ///
    /// Returns [`UiBackendError::UnsupportedPlatform`] when the renderer is
    /// not available on this platform.
    pub fn new() -> Result<Self, UiBackendError> {
        if !cfg!(windows) {
            return Err(UiBackendError::UnsupportedPlatform);
        }
        Ok(Self::default())
    }

    /// Opens the window root.
    ///
    /// # Errors
    ///
    /// Returns [`UiBackendError::WrongParentKind`] when a root already
    /// exists and [`UiBackendError::InvalidArgument`] when the requested size
    /// is outside the accepted range.
    pub fn window(&self, title: &str, width: i64, height: i64) -> Result<u64, UiBackendError> {
        if title.len() > MAX_TITLE_BYTES {
            return Err(UiBackendError::Limit {
                which: "title-bytes",
            });
        }
        if !(1..=16_384).contains(&width) || !(1..=16_384).contains(&height) {
            return Err(UiBackendError::InvalidArgument {
                which: "window-size",
            });
        }
        let mut state = self.lock()?;
        if state.window.is_some() {
            return Err(UiBackendError::WrongParentKind);
        }
        let handle = state.tree.insert(NodeKind::Root, None)?;
        state.window = Some(WindowSpec {
            title: title.to_owned(),
            width: u32::try_from(width).map_err(|_| UiBackendError::InvalidArgument {
                which: "window-size",
            })?,
            height: u32::try_from(height).map_err(|_| UiBackendError::InvalidArgument {
                which: "window-size",
            })?,
        });
        Ok(handle)
    }

    /// Adds a vertical layout container.
    ///
    /// # Errors
    ///
    /// Returns the tree's typed refusals: unknown handle, wrong parent kind
    /// or a limit overrun.
    pub fn column(&self, parent: u64) -> Result<u64, UiBackendError> {
        self.lock()?.tree.insert(NodeKind::Column, Some(parent))
    }

    /// Adds a horizontal layout container.
    ///
    /// # Errors
    ///
    /// Returns the tree's typed refusals: unknown handle, wrong parent kind
    /// or a limit overrun.
    pub fn row(&self, parent: u64) -> Result<u64, UiBackendError> {
        self.lock()?.tree.insert(NodeKind::Row, Some(parent))
    }

    /// Adds a text run.
    ///
    /// # Errors
    ///
    /// Returns the tree's typed refusals: unknown handle, wrong parent kind
    /// or a limit overrun.
    pub fn text(&self, parent: u64, content: &str) -> Result<u64, UiBackendError> {
        let mut state = self.lock()?;
        let handle = state.tree.insert(NodeKind::Text, Some(parent))?;
        state.tree.set_text(handle, content)?;
        Ok(handle)
    }

    /// Adds a push button.
    ///
    /// # Errors
    ///
    /// Returns the tree's typed refusals: unknown handle, wrong parent kind
    /// or a limit overrun.
    pub fn button(&self, parent: u64, label: &str) -> Result<u64, UiBackendError> {
        let mut state = self.lock()?;
        let handle = state.tree.insert(NodeKind::Button, Some(parent))?;
        state.tree.set_text(handle, label)?;
        Ok(handle)
    }

    /// Adds a dropdown with a fixed option list.
    ///
    /// # Errors
    ///
    /// Returns the tree's typed refusals: unknown handle, wrong parent kind
    /// or a limit overrun.
    pub fn dropdown(&self, parent: u64, options: &[String]) -> Result<u64, UiBackendError> {
        let mut state = self.lock()?;
        let handle = state.tree.insert(NodeKind::Dropdown, Some(parent))?;
        state.tree.set_options(handle, options)?;
        Ok(handle)
    }

    /// Adds a fixed-size image preview.
    ///
    /// # Errors
    ///
    /// Returns the tree's typed refusals: unknown handle, wrong parent kind
    /// or a limit overrun.
    pub fn image_preview(
        &self,
        parent: u64,
        width: u64,
        height: u64,
    ) -> Result<u64, UiBackendError> {
        let mut state = self.lock()?;
        let handle = state.tree.insert(NodeKind::ImagePreview, Some(parent))?;
        state.tree.set_image(handle, width, height, Vec::new())?;
        Ok(handle)
    }

    /// Adds a progress bar.
    ///
    /// # Errors
    ///
    /// Returns the tree's typed refusals: unknown handle, wrong parent kind
    /// or a limit overrun.
    pub fn progress(&self, parent: u64) -> Result<u64, UiBackendError> {
        let mut state = self.lock()?;
        let handle = state.tree.insert(NodeKind::Progress, Some(parent))?;
        state.tree.set_progress(handle, 0, 1)?;
        Ok(handle)
    }

    /// Replaces the text content of a node.
    ///
    /// # Errors
    ///
    /// Returns the tree's typed refusals: unknown handle or a limit overrun.
    pub fn set_text(&self, handle: u64, content: &str) -> Result<bool, UiBackendError> {
        self.lock()?.tree.set_text(handle, content)?;
        Ok(true)
    }

    /// Sets the `(done, of)` position of a progress bar.
    ///
    /// # Errors
    ///
    /// Returns the tree's typed refusals: unknown handle or a wrong parent
    /// kind.
    pub fn set_progress(&self, handle: u64, done: u64, of: u64) -> Result<bool, UiBackendError> {
        self.lock()?.tree.set_progress(handle, done, of)?;
        Ok(true)
    }

    /// Blits a BGRA8 buffer into an image preview.
    ///
    /// # Errors
    ///
    /// Returns the tree's typed refusals: unknown handle, a wrong parent kind
    /// or a limit overrun.
    pub fn draw_image(
        &self,
        handle: u64,
        width: u64,
        height: u64,
        pixels: &[u8],
    ) -> Result<bool, UiBackendError> {
        self.lock()?
            .tree
            .set_image(handle, width, height, pixels.to_vec())?;
        Ok(true)
    }

    /// Renders the current tree into one deterministic frame.
    ///
    /// # Errors
    ///
    /// Returns [`UiBackendError::NoWindow`] when no window root exists and
    /// [`UiBackendError::RenderFailed`] when the platform renderer refuses
    /// the frame.
    pub fn render_frame(&self) -> Result<Frame, UiBackendError> {
        let (tree, window) = {
            let state = self.lock()?;
            let window = state.window.clone().ok_or(UiBackendError::NoWindow)?;
            (state.tree.clone(), window)
        };
        render::render_frame(&tree, window.width, window.height)
    }

    /// The frozen token-set identity this renderer draws with.
    #[must_use]
    pub fn token_set_version(&self) -> &'static str {
        tokens::TOKEN_SET_VERSION
    }

    /// The title of the open window, if one exists.
    #[must_use]
    pub fn window_title(&self) -> Option<String> {
        self.state
            .lock()
            .ok()
            .and_then(|state| state.window.as_ref().map(|window| window.title.clone()))
    }

    /// Clones the current UI tree for hit-testing (STEP-0597).
    ///
    /// The tree is pure data; this read-only snapshot lets the runner run
    /// [`crate::layout::compute`] + [`crate::layout::Layout::hit`] for a
    /// coordinate click without touching the render path or the OS window. The
    /// client size is **not** part of this snapshot — the live client rect is a
    /// shell concern ([`crate::windowed::WindowedUiBackend::layout_input`]).
    ///
    /// # Errors
    ///
    /// Returns [`UiBackendError::RenderFailed`] when the state lock is
    /// poisoned.
    pub fn tree_snapshot(&self) -> Result<UiTree, UiBackendError> {
        Ok(self.lock()?.tree.clone())
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, State>, UiBackendError> {
        self.state.lock().map_err(|_| UiBackendError::RenderFailed {
            detail: "state poisoned".to_owned(),
        })
    }
}
