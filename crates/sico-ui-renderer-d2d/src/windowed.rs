//! The guest-facing windowed backend: the D2D tree + the OS shell, composed.
//!
//! [`WindowedUiBackend`] is the production seam the runner's `ui@1` binding
//! wraps: it owns the offscreen tree ([`D2dUiBackend`]) and a real OS window
//! ([`WindowShell`]) and answers the thirteen `ui@1` operations. It is the
//! slice that turns the STEP-0338 renderer core + the STEP-0577 window shell
//! into one `Send + Sync` object a runner can hold.
//!
//! Threading model: the shell owns the OS window and its message pump, and a
//! Win32 window is thread-affine — every message must be pumped on the thread
//! that created it. So the shell lives on a dedicated worker thread and this
//! object talks to it over a request/reply channel. The tree stays on the
//! caller thread (it is pure data behind a `Mutex`). Every call is a bounded
//! synchronous request; a lost shell thread is a typed [`UiBackendError`], never
//! a panic. This is what makes the struct `Send + Sync` **without** any
//! `unsafe impl`: `Sender`/`Receiver`/`JoinHandle` are all `Send + Sync`, and
//! the shell's `!Send` state never leaves its thread.
//!
//! Scope honesty (mirrors [`WindowShell`] and the `ui@1` contract):
//!
//! - authority is **not** part of this crate: the grant check and the
//!   default-deny gate stay in the runner (ADR-0018 §4);
//! - `poll_event` returns [`WindowEvent`] (client coordinates), **not** a
//!   `click(handle)`/`select`: the `ui@1` `click(ui-handle)` /
//!   `select(ui-selection)` variants **exist** (RFC-0052 §2), but point→handle
//!   hit-testing needs the renderer layout, which is the next slice, so this
//!   backend honestly delivers no node handle for a click;
//! - the guest-facing `ui-error` set has no render-failure discriminant, so a
//!   window-creation or present failure is reported here as
//!   [`UiBackendError::RenderFailed`] and the **runner** decides how to map it
//!   to the closed `ui-error` set (it collapses to `no-window`, documented in
//!   the runner binding);
//! - no M24 §3 gate is flipped by this module.

use std::sync::Mutex;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread::JoinHandle;

use crate::backend::D2dUiBackend;
use crate::error::UiBackendError;
use crate::events::WindowEvent;
use crate::frame::Frame;
use crate::shell::{MAX_TITLE_BYTES, WindowShell};
use crate::tree::UiTree;

/// The accepted client-size range in 96-DPI logical pixels. Mirrors the tree's
/// window-size budget so a size the tree would reject is refused before the OS
/// window is created (a typed refusal, never a half-created window).
pub const MAX_WINDOW_SIDE: u32 = 16_384;

/// One command the shell thread executes on its window.
enum ShellCommand {
    /// Creates the OS window with the requested client size.
    Create {
        title: String,
        width: u32,
        height: u32,
    },
    /// Presents one already-rendered frame (a pure byte copy, STEP-0577).
    Present(Frame),
    /// Pumps the message queue once and returns the next delivered event.
    Pump,
    /// Reads back the last presented frame from the DIB.
    ReadPresented,
    /// Reads the current client size from the OS.
    ClientSize,
    /// Posts a `WM_CLOSE`.
    Close,
    /// Destroys the window (idempotent).
    Destroy,
    /// Ends the loop; the shell thread returns and the window is destroyed.
    Shutdown,
}

/// One reply from the shell thread.
enum ShellReply {
    /// The shell finished its one-time setup.
    Ready(Result<(), UiBackendError>),
    /// The result of a `Create` command.
    Create(Result<(), UiBackendError>),
    /// The result of a `Present` command.
    Present(Result<(), UiBackendError>),
    /// The result of a `Pump` command.
    Pump(Option<WindowEvent>),
    /// The result of a `ReadPresented` command.
    ReadPresented(Result<Frame, UiBackendError>),
    /// The result of a `ClientSize` command.
    ClientSize(Result<(u32, u32), UiBackendError>),
    /// The result of a `Close` command.
    Close(Result<(), UiBackendError>),
    /// The result of a `Destroy` command.
    Destroy(Result<(), UiBackendError>),
}

/// The shell-thread body: it owns the [`WindowShell`] for its whole lifetime.
///
/// The channels are borrowed: the thread closure that owns them moves them into
/// the spawned thread and this body borrows them for its lifetime.
fn shell_thread(cmd_rx: &Receiver<ShellCommand>, reply_tx: &Sender<ShellReply>) {
    // The window class registration and the DPI-awareness request are process
    // wide and must happen on the thread that owns the window.
    let mut shell = match WindowShell::new() {
        Ok(shell) => shell,
        Err(error) => {
            let _ = reply_tx.send(ShellReply::Ready(Err(error)));
            return;
        }
    };
    if reply_tx.send(ShellReply::Ready(Ok(()))).is_err() {
        return;
    }
    while let Ok(command) = cmd_rx.recv() {
        let reply = match command {
            ShellCommand::Create {
                title,
                width,
                height,
            } => ShellReply::Create(shell.create_window(&title, width, height)),
            ShellCommand::Present(frame) => ShellReply::Present(shell.present_frame(&frame)),
            ShellCommand::Pump => ShellReply::Pump(shell.pump_once()),
            ShellCommand::ReadPresented => ShellReply::ReadPresented(shell.read_presented_frame()),
            ShellCommand::ClientSize => ShellReply::ClientSize(shell.client_size()),
            ShellCommand::Close => ShellReply::Close(shell.close()),
            ShellCommand::Destroy => ShellReply::Destroy(shell.destroy()),
            ShellCommand::Shutdown => return,
        };
        if reply_tx.send(reply).is_err() {
            return;
        }
    }
}

/// A `Send + Sync` handle to the dedicated shell thread.
///
/// `Receiver` is `!Sync`, so the reply side is behind a `Mutex`. There is
/// exactly one consumer (the guest's host-call thread) and commands are
/// serialized, so the lock is never contended; the mutex exists only to make
/// the handle `Sync` without an `unsafe impl`.
struct ShellClient {
    cmd_tx: Sender<ShellCommand>,
    reply_rx: Mutex<Receiver<ShellReply>>,
    join: Option<JoinHandle<()>>,
}

impl ShellClient {
    /// Spawns the shell thread and waits for it to become ready.
    ///
    /// # Errors
    ///
    /// Returns [`UiBackendError::UnsupportedPlatform`] on a non-Windows host,
    /// [`UiBackendError::RenderFailed`] when the thread cannot start or the
    /// shell refuses to initialize.
    fn start() -> Result<Self, UiBackendError> {
        if !cfg!(windows) {
            return Err(UiBackendError::UnsupportedPlatform);
        }
        let (cmd_tx, cmd_rx) = channel();
        let (reply_tx, reply_rx) = channel();
        let join = std::thread::Builder::new()
            .name("sico-ui-shell".to_owned())
            .spawn(move || shell_thread(&cmd_rx, &reply_tx))
            .map_err(|error| UiBackendError::RenderFailed {
                detail: format!("shell thread spawn: {error}"),
            })?;
        let mut client = Self {
            cmd_tx,
            reply_rx: Mutex::new(reply_rx),
            join: Some(join),
        };
        match client.recv()? {
            ShellReply::Ready(Ok(())) => Ok(client),
            ShellReply::Ready(Err(error)) => {
                client.shutdown();
                Err(error)
            }
            _ => {
                client.shutdown();
                Err(protocol_error("shell ready"))
            }
        }
    }

    /// Blocks for the next shell reply.
    fn recv(&self) -> Result<ShellReply, UiBackendError> {
        let receiver = self
            .reply_rx
            .lock()
            .map_err(|_| UiBackendError::RenderFailed {
                detail: "shell reply mutex poisoned".to_owned(),
            })?;
        receiver.recv().map_err(|_| UiBackendError::RenderFailed {
            detail: "shell thread lost".to_owned(),
        })
    }

    /// Sends one command and waits for its reply.
    fn request(&self, command: ShellCommand) -> Result<ShellReply, UiBackendError> {
        self.cmd_tx
            .send(command)
            .map_err(|_| UiBackendError::RenderFailed {
                detail: "shell thread lost".to_owned(),
            })?;
        self.recv()
    }

    fn create_window(&self, title: &str, width: u32, height: u32) -> Result<(), UiBackendError> {
        match self.request(ShellCommand::Create {
            title: title.to_owned(),
            width,
            height,
        })? {
            ShellReply::Create(result) => result,
            _ => Err(protocol_error("shell create")),
        }
    }

    fn present(&self, frame: Frame) -> Result<(), UiBackendError> {
        match self.request(ShellCommand::Present(frame))? {
            ShellReply::Present(result) => result,
            _ => Err(protocol_error("shell present")),
        }
    }

    fn pump(&self) -> Result<Option<WindowEvent>, UiBackendError> {
        match self.request(ShellCommand::Pump)? {
            ShellReply::Pump(event) => Ok(event),
            _ => Err(protocol_error("shell pump")),
        }
    }

    fn read_presented_frame(&self) -> Result<Frame, UiBackendError> {
        match self.request(ShellCommand::ReadPresented)? {
            ShellReply::ReadPresented(result) => result,
            _ => Err(protocol_error("shell read-presented")),
        }
    }

    fn client_size(&self) -> Result<(u32, u32), UiBackendError> {
        match self.request(ShellCommand::ClientSize)? {
            ShellReply::ClientSize(result) => result,
            _ => Err(protocol_error("shell client-size")),
        }
    }

    fn close(&self) -> Result<(), UiBackendError> {
        match self.request(ShellCommand::Close)? {
            ShellReply::Close(result) => result,
            _ => Err(protocol_error("shell close")),
        }
    }

    fn destroy(&self) -> Result<(), UiBackendError> {
        match self.request(ShellCommand::Destroy)? {
            ShellReply::Destroy(result) => result,
            _ => Err(protocol_error("shell destroy")),
        }
    }

    fn shutdown(&mut self) {
        // The shell may already be gone (a lost thread); the join is best-effort
        // and the window is destroyed by the shell's `Drop` on the thread.
        let _ = self.cmd_tx.send(ShellCommand::Shutdown);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

impl Drop for ShellClient {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn protocol_error(stage: &str) -> UiBackendError {
    UiBackendError::RenderFailed {
        detail: format!("shell {stage} protocol mismatch"),
    }
}

/// The composed production backend: offscreen tree + OS window shell.
///
/// This is `Send + Sync` by construction (no `unsafe impl`): the shell state
/// never leaves its thread, and every operation crosses the channel. It is the
/// object the runner wraps to serve the granted `ui@1` surface.
pub struct WindowedUiBackend {
    tree: D2dUiBackend,
    shell: ShellClient,
}

impl WindowedUiBackend {
    /// Creates the backend and its dedicated shell thread.
    ///
    /// # Errors
    ///
    /// Returns [`UiBackendError::UnsupportedPlatform`] on a non-Windows host
    /// and [`UiBackendError::RenderFailed`] when the shell thread cannot start
    /// or the window class cannot be registered.
    pub fn new() -> Result<Self, UiBackendError> {
        let tree = D2dUiBackend::new()?;
        let shell = ShellClient::start()?;
        Ok(Self { tree, shell })
    }

    /// Opens the window root (the OS window and the tree root, together).
    ///
    /// # Errors
    ///
    /// Returns [`UiBackendError::Limit`] for an over-long title,
    /// [`UiBackendError::InvalidArgument`] for an out-of-range client size,
    /// [`UiBackendError::WrongParentKind`] when a window already exists and
    /// [`UiBackendError::RenderFailed`] when the OS refuses the window.
    pub fn window(&self, title: &str, width: i64, height: i64) -> Result<u64, UiBackendError> {
        if title.len() > MAX_TITLE_BYTES {
            return Err(UiBackendError::Limit {
                which: "title-bytes",
            });
        }
        let width = u32::try_from(width).map_err(|_| UiBackendError::InvalidArgument {
            which: "window-size",
        })?;
        let height = u32::try_from(height).map_err(|_| UiBackendError::InvalidArgument {
            which: "window-size",
        })?;
        if !(1..=MAX_WINDOW_SIDE).contains(&width) || !(1..=MAX_WINDOW_SIDE).contains(&height) {
            return Err(UiBackendError::InvalidArgument {
                which: "window-size",
            });
        }
        // The OS window is created first; the tree root is inserted second, so
        // a shell failure never leaves a half-built tree behind. If the tree
        // step somehow refuses (a root already exists — unreachable because the
        // shell would have refused first), the fresh OS window is torn down.
        self.shell.create_window(title, width, height)?;
        match self.tree.window(title, i64::from(width), i64::from(height)) {
            Ok(handle) => Ok(handle),
            Err(error) => {
                let _ = self.shell.destroy();
                Err(error)
            }
        }
    }

    /// Adds a vertical layout container.
    ///
    /// # Errors
    ///
    /// Returns the tree's typed refusals: unknown handle, wrong parent kind
    /// or a limit overrun.
    pub fn column(&self, parent: u64) -> Result<u64, UiBackendError> {
        self.tree.column(parent)
    }

    /// Adds a horizontal layout container.
    ///
    /// # Errors
    ///
    /// Returns the tree's typed refusals: unknown handle, wrong parent kind
    /// or a limit overrun.
    pub fn row(&self, parent: u64) -> Result<u64, UiBackendError> {
        self.tree.row(parent)
    }

    /// Adds a text run.
    ///
    /// # Errors
    ///
    /// Returns the tree's typed refusals: unknown handle, wrong parent kind
    /// or a limit overrun.
    pub fn text(&self, parent: u64, content: &str) -> Result<u64, UiBackendError> {
        self.tree.text(parent, content)
    }

    /// Adds a push button.
    ///
    /// # Errors
    ///
    /// Returns the tree's typed refusals: unknown handle, wrong parent kind
    /// or a limit overrun.
    pub fn button(&self, parent: u64, label: &str) -> Result<u64, UiBackendError> {
        self.tree.button(parent, label)
    }

    /// Adds a dropdown with a fixed option list.
    ///
    /// # Errors
    ///
    /// Returns the tree's typed refusals: unknown handle, wrong parent kind
    /// or a limit overrun.
    pub fn dropdown(&self, parent: u64, options: &[String]) -> Result<u64, UiBackendError> {
        self.tree.dropdown(parent, options)
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
        self.tree.image_preview(parent, width, height)
    }

    /// Adds a progress bar.
    ///
    /// # Errors
    ///
    /// Returns the tree's typed refusals: unknown handle, wrong parent kind
    /// or a limit overrun.
    pub fn progress(&self, parent: u64) -> Result<u64, UiBackendError> {
        self.tree.progress(parent)
    }

    /// Replaces the text content of a node.
    ///
    /// # Errors
    ///
    /// Returns the tree's typed refusals: unknown handle or a limit overrun.
    pub fn set_text(&self, handle: u64, content: &str) -> Result<bool, UiBackendError> {
        self.tree.set_text(handle, content)
    }

    /// Sets the `(done, of)` position of a progress bar.
    ///
    /// # Errors
    ///
    /// Returns the tree's typed refusals: unknown handle or a wrong parent
    /// kind.
    pub fn set_progress(&self, handle: u64, done: u64, of: u64) -> Result<bool, UiBackendError> {
        self.tree.set_progress(handle, done, of)
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
        self.tree.draw_image(handle, width, height, pixels)
    }

    /// Pumps the OS event queue once and returns the next delivered event.
    ///
    /// The event is a [`WindowEvent`] in client coordinates. A click carries no
    /// node handle (the `ui@1` `click(ui-handle)` variant exists, but
    /// point→handle hit-testing needs the renderer layout — the next slice),
    /// a resize/paint is a window-lifecycle notification with no `ui@1`
    /// variant — the **runner** decides which of these cross the guest
    /// boundary.
    ///
    /// # Errors
    ///
    /// Returns [`UiBackendError::RenderFailed`] when the shell thread was lost.
    pub fn poll_event(&self) -> Result<Option<WindowEvent>, UiBackendError> {
        self.shell.pump()
    }

    /// Renders the current tree offscreen and presents it into the window.
    ///
    /// The frame is a pure function of the tree (ADR-0018 §3): it is computed
    /// on the caller thread and only the bytes cross to the shell thread.
    ///
    /// # Errors
    ///
    /// Returns [`UiBackendError::NoWindow`] when no window root exists and
    /// [`UiBackendError::RenderFailed`] when the renderer or the present blit
    /// fails.
    pub fn run(&self) -> Result<bool, UiBackendError> {
        let frame = self.tree.render_frame()?;
        self.shell.present(frame)?;
        Ok(true)
    }

    /// Reads back the last presented frame from the shell's DIB.
    ///
    /// This is the audit affordance that makes the composed present path
    /// verifiable end-to-end: the returned frame is byte-identical to the frame
    /// that was blitted into the client area (the same copy the STEP-0577
    /// `WindowShell::read_presented_frame` returns).
    ///
    /// # Errors
    ///
    /// Returns [`UiBackendError::NoWindow`] when nothing has been presented.
    pub fn read_presented_frame(&self) -> Result<Frame, UiBackendError> {
        self.shell.read_presented_frame()
    }

    /// The current client size + a tree snapshot, for hit-testing (STEP-0597).
    ///
    /// This is a **read-only audit affordance** (never touches the render path
    /// or the OS event queue): it gives the runner the inputs [`crate::layout::compute`]
    /// needs to turn a [`crate::events::WindowEvent::Clicked`] coordinate into a
    /// `click(ui-handle)` event. The client size is the shell's **live** client
    /// rect ([`crate::shell::WindowShell::client_size`] — the OS reports the
    /// current client area, which is the coordinate space the click arrives
    /// in); the tree is a cloned snapshot of the current UI state.
    ///
    /// The tuple is `(width, height, tree)` in 96-DPI logical pixels.
    ///
    /// # Errors
    ///
    /// Returns [`UiBackendError::NoWindow`] when no window exists and
    /// [`UiBackendError::RenderFailed`] when the shell thread was lost or the
    /// tree state lock is poisoned.
    pub fn layout_input(&self) -> Result<(u32, u32, UiTree), UiBackendError> {
        let (width, height) = self.shell.client_size()?;
        let tree = self.tree.tree_snapshot()?;
        Ok((width, height, tree))
    }

    /// Posts a `WM_CLOSE` so the window closes deterministically.
    ///
    /// # Errors
    ///
    /// Returns [`UiBackendError::NoWindow`] when no window exists.
    pub fn close(&self) -> Result<(), UiBackendError> {
        self.shell.close()
    }

    /// Destroys the window; idempotent (a window the OS already destroyed is
    /// cleared without a second `DestroyWindow`).
    ///
    /// # Errors
    ///
    /// Returns [`UiBackendError::NoWindow`] when no window exists.
    pub fn destroy(&self) -> Result<(), UiBackendError> {
        self.shell.destroy()
    }

    /// The title of the open window, if one exists.
    #[must_use]
    pub fn window_title(&self) -> Option<String> {
        self.tree.window_title()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drains pending events (a hidden window may queue an initial resize or
    /// paint) and reports whether a `Closed` was seen within `rounds` pumps.
    #[cfg(windows)]
    fn pump_until_closed(backend: &WindowedUiBackend, rounds: usize) -> bool {
        for _ in 0..rounds {
            if let Some(WindowEvent::Closed) = backend.poll_event().expect("pump") {
                return true;
            }
        }
        false
    }

    #[test]
    #[cfg(windows)]
    #[ignore = "creates a real OS window; run with --ignored on an interactive Windows session"]
    fn real_window_lifecycle_via_windowed_backend() {
        let backend = WindowedUiBackend::new().expect("backend");
        let root = backend.window("Lifecycle", 320, 200).expect("window");
        assert_eq!(root, 1, "the first node is the window root handle");
        assert_eq!(backend.window_title().as_deref(), Some("Lifecycle"));

        // Render + present one frame through the composed path.
        let column = backend.column(root).expect("column");
        backend.text(column, "Sico").expect("text");
        backend.run().expect("present");
        // The read-back is the DIB the blit wrote: the present path is a pure
        // byte copy (STEP-0577), so the composed backend's run() leaves an
        // exact 320x200 frame in the shell.
        let read_back = backend.read_presented_frame().expect("read back");
        assert_eq!((read_back.width(), read_back.height()), (320, 200));

        // No `Closed` before the close.
        assert!(
            !pump_until_closed(&backend, 8),
            "no Closed event before the close is posted"
        );

        // A close posts WM_CLOSE; pumping delivers Closed deterministically.
        backend.close().expect("close");
        assert!(
            pump_until_closed(&backend, 64),
            "WM_CLOSE must deliver a Closed event"
        );

        // Destroy is idempotent for the close-then-destroy path: the OS already
        // destroyed the window on close, so the first destroy clears it (Ok);
        // a second destroy with no window is the typed NoWindow refusal.
        backend.destroy().expect("destroy clears the closed window");
        assert_eq!(
            backend.destroy(),
            Err(UiBackendError::NoWindow),
            "a second destroy with no window is a typed refusal"
        );
    }
}
