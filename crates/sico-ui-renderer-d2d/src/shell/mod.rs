//! The OS window shell and event pump (STEP-0577).
//!
//! This module owns the native window: it registers a window class, creates
//! and destroys an `HWND`, pumps the thread's message queue and presents
//! rendered frames into the client area. It is the second half of the M24
//! §8.6.1 renderer after STEP-0338: the renderer core stays offscreen and
//! headless, and this shell is the only thing that touches the OS window.
//!
//! The split mirrors the `crate::render` module: the Win32 FFI lives in exactly
//! one module (`win`) and every other module in the crate stays unsafe-free.
//! The public [`WindowShell`] is platform-dispatched — Windows gives a real
//! shell, every other host gives the typed
//! [`UiBackendError::UnsupportedPlatform`] refusal.
//!
//! Determinism note: the shell renders nothing itself. Presenting a frame is
//! a pure byte copy of an existing offscreen frame (STEP-0338) into a GDI DIB
//! followed by a `BitBlt`; the frame bytes are never recomputed, so the
//! frozen frame corpus is untouched. The shell requests DPI-unawareness so
//! the client area stays in the 96-DPI logical pixels the token table uses;
//! if the process already pinned a different awareness, that wins and no
//! coordinate determinism is claimed.
//!
//! What this slice does **not** do (and therefore does not claim):
//!
//! - no guest-visible UI capability: the runner-side `ui@1` wiring is the
//!   next slice, so nothing here reaches a guest;
//! - no `click(handle)` / `select` delivery: the `ui@1` `click(ui-handle)` /
//!   `select(ui-selection)` variants **exist** (RFC-0052 §2), but a click is
//!   captured as client coordinates and point→handle hit-testing needs the
//!   renderer layout (next slice), so no handle is fabricated;
//! - no M24 §3 gate is flipped by this slice.

#[cfg(not(windows))]
mod unsupported;
#[cfg(windows)]
mod win;

use crate::error::UiBackendError;
use crate::events::WindowEvent;
use crate::frame::Frame;

use std::marker::PhantomData;

/// Maximum UTF-8 length of a window title.
///
/// The limit is checked before the title is encoded to UTF-16 (before any
/// window-text allocation), so an over-long title is a typed refusal and
/// never a half-created window.
pub const MAX_TITLE_BYTES: usize = 256;

/// The native OS window shell with a message pump.
///
/// The shell is **not** `Send`/`Sync` by a **deliberate** marker (STEP-0585,
/// from the STEP-0577 independent safety review): it owns an OS window handle
/// and the message pump must run on the thread that created the window. The
/// `windows` 0.52 handle types are `pub struct(pub isize)` and themselves
/// `Send + Sync`, so the affinity must not rest on an accidental field (the
/// DIB's raw `bits` pointer once carried it); the
/// [`PhantomData<*mut ()>`](std::marker::PhantomData) field pins `!Send +
/// !Sync`, and the test module asserts the pin. It is platform-dispatched — on
/// a non-Windows host every method refuses with
/// [`UiBackendError::UnsupportedPlatform`].
pub struct WindowShell {
    #[cfg(windows)]
    inner: win::WinShell,
    #[cfg(not(windows))]
    inner: unsupported::NoShell,
    /// Deliberate `!Send + !Sync` pin; never read, never leaves its thread.
    _not_send_sync: PhantomData<*mut ()>,
}

impl WindowShell {
    /// Registers the window class and requests DPI-unawareness.
    ///
    /// # Errors
    ///
    /// Returns [`UiBackendError::UnsupportedPlatform`] on a non-Windows host
    /// and [`UiBackendError::RenderFailed`] when the window class cannot be
    /// registered.
    pub fn new() -> Result<Self, UiBackendError> {
        #[cfg(windows)]
        {
            Ok(Self {
                inner: win::WinShell::new()?,
                _not_send_sync: PhantomData,
            })
        }
        #[cfg(not(windows))]
        {
            let _ = unsupported::NoShell;
            Err(UiBackendError::UnsupportedPlatform)
        }
    }

    /// Creates a top-level window with the requested client size.
    ///
    /// The window is created **hidden**: showing it is the caller's choice
    /// (the message pump and the present path work on a hidden window too).
    ///
    /// # Errors
    ///
    /// Returns [`UiBackendError::Limit`] when the title exceeds
    /// [`MAX_TITLE_BYTES`], [`UiBackendError::WrongParentKind`] when a window
    /// already exists and [`UiBackendError::RenderFailed`] when the OS
    /// refuses the window.
    pub fn create_window(
        &mut self,
        title: &str,
        width: u32,
        height: u32,
    ) -> Result<(), UiBackendError> {
        if title.len() > MAX_TITLE_BYTES {
            return Err(UiBackendError::Limit {
                which: "title-bytes",
            });
        }
        self.inner.create_window(title, width, height)
    }

    /// Pumps the message queue once and returns one delivered event.
    ///
    /// When no event is pending the method returns `None`; it never blocks.
    /// Events already queued from a previous dispatch are delivered before
    /// the OS queue is touched again.
    pub fn pump_once(&mut self) -> Option<WindowEvent> {
        self.inner.pump_once()
    }

    /// Drains up to `max_events` delivered events.
    ///
    /// Returns the events in delivery order and whether the queue cap was hit
    /// during the drain (the [`WindowEvent`] cap coalesces by kind and
    /// reports an overflow instead of dropping an event silently).
    pub fn drain(&mut self, max_events: usize) -> (Vec<WindowEvent>, bool) {
        self.inner.drain(max_events)
    }

    /// Presents one frame into the window client area.
    ///
    /// The frame bytes are copied verbatim into a GDI DIB and blitted; the
    /// frame is never recomputed here, so the frozen frame corpus is
    /// unaffected. `frame` must have been rendered for the current client
    /// size.
    ///
    /// # Errors
    ///
    /// Returns [`UiBackendError::NoWindow`] when no window exists and
    /// [`UiBackendError::RenderFailed`] when the blit fails.
    pub fn present_frame(&mut self, frame: &Frame) -> Result<(), UiBackendError> {
        self.inner.present_frame(frame)
    }

    /// Reads back the last presented frame from the DIB.
    ///
    /// This is the audit affordance that makes the present path verifiable:
    /// the returned frame is byte-identical to the frame that was blitted
    /// into the client area.
    ///
    /// # Errors
    ///
    /// Returns [`UiBackendError::NoWindow`] when nothing has been presented
    /// yet.
    pub fn read_presented_frame(&self) -> Result<Frame, UiBackendError> {
        self.inner.read_presented_frame()
    }

    /// Posts a `WM_CLOSE` so the window closes deterministically.
    ///
    /// # Errors
    ///
    /// Returns [`UiBackendError::NoWindow`] when no window exists.
    pub fn close(&self) -> Result<(), UiBackendError> {
        self.inner.close()
    }

    /// Whether `WM_QUIT` was seen (the message loop should end).
    #[must_use]
    pub fn should_quit(&self) -> bool {
        self.inner.should_quit()
    }

    /// The current client size in 96-DPI logical pixels.
    ///
    /// # Errors
    ///
    /// Returns [`UiBackendError::NoWindow`] when no window exists.
    pub fn client_size(&self) -> Result<(u32, u32), UiBackendError> {
        self.inner.client_size()
    }

    /// Destroys the window and releases the shell.
    ///
    /// Destroy is idempotent: a window the OS already destroyed (the
    /// `WM_CLOSE` → `DestroyWindow` path) is cleared without error, so a
    /// close-then-destroy sequence is safe.
    ///
    /// # Errors
    ///
    /// Returns [`UiBackendError::NoWindow`] when no window exists.
    pub fn destroy(&mut self) -> Result<(), UiBackendError> {
        self.inner.destroy()
    }
}

#[cfg(windows)]
impl std::fmt::Debug for WindowShell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WindowShell").finish_non_exhaustive()
    }
}

#[cfg(not(windows))]
impl std::fmt::Debug for WindowShell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("WindowShell(unsupported)")
    }
}

// The public shell type is `!Send + !Sync` on every host. On Windows this is
// also implied by the wrapped `win::WinShell`, but the marker on `WindowShell`
// makes the invariant hold uniformly (including the non-Windows `NoShell`
// branch) and pins it against a refactor.
#[cfg(test)]
mod ffi_invariants {
    use super::WindowShell;

    /// Compile-time proof that [`WindowShell`] is `!Send + !Sync`.
    ///
    /// Hand-rolled `static_assertions`-style `assert_not_impl_any!` (no new
    /// dependency, STEP-0585): [`AmbiguousIfImpl`] has a blanket impl for every
    /// type at `A = ()` and a second impl for any type implementing the
    /// asserted trait at `A = Invalid`. If `$x` implements the asserted trait,
    /// `<T as AmbiguousIfImpl<_>>::some_item` has two candidate impls and the
    /// inference of `_` is ambiguous — a compile error. If it does **not**,
    /// only the blanket applies and the invocation compiles. If a refactor
    /// ever makes the shell `Send` or `Sync`, this fails the build.
    macro_rules! assert_not_impl_any {
        ($x:ty: $($t:path),+ $(,)?) => {
            const _: fn() = || {
                trait AmbiguousIfImpl<A> {
                    fn some_item() {}
                }
                impl<T: ?Sized> AmbiguousIfImpl<()> for T {}
                $({
                    struct Invalid;
                    impl<T: ?Sized + $t> AmbiguousIfImpl<Invalid> for T {}
                })+
                let _ = <$x as AmbiguousIfImpl<_>>::some_item;
            };
        };
    }

    assert_not_impl_any!(WindowShell: Send, Sync);

    #[test]
    fn windowshell_is_deliberately_not_send_sync() {
        // The real evidence is the `assert_not_impl_any!` invocation above: a
        // compile-time check that fails the build if `WindowShell` is `Send`
        // or `Sync`. This test re-confirms the deliberate marker is zero-sized
        // (it must not cost a field).
        assert_eq!(std::mem::size_of::<std::marker::PhantomData<*mut ()>>(), 0);
    }
}
