//! Windows `IFileOpenDialog` / `IFileSaveDialog` backend (STEP-0627, RFC-0055).
//!
//! The COM file-dialog interfaces are STA-only and modal to an owner window;
//! mirroring the [`crate::windowed`] Direct2D thread model, a dedicated
//! thread initializes COM (`COINIT_APARTMENTTHREADED`) and services open/save
//! requests over a channel so the caller never blocks the runner and never
//! shares a COM pointer across apartments.
//!
//! `unsafe_code` is allowed by this crate's Cargo lints (ADR-0018 decision 1:
//! the sole home of platform `unsafe`).
//!
//! Ported from `codex/m24-vision-ui-v2` `crates/sico-ui-d2d/src/picker.rs`
//! (STEP-0557) and adapted to the `windows` **0.52** bindings this crate pins
//! (the codex adapter targeted 0.61.3). The only behavioural change is the
//! D4 fix below: the `PWSTR` returned by `IShellItem::GetDisplayName` is
//! allocated by `CoTaskMemAlloc` and the codex adapter leaked it; here it is
//! freed unconditionally on both the Ok and the Err path of the UTF-16 →
//! `String` conversion.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]

use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::sync::{Mutex, PoisonError};
use std::thread::JoinHandle;

#[cfg(windows)]
use windows::Win32::Foundation::ERROR_CANCELLED;
#[cfg(windows)]
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoCreateInstance,
    CoInitializeEx, CoTaskMemFree, CoUninitialize,
};
#[cfg(windows)]
use windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;
#[cfg(windows)]
use windows::Win32::UI::Shell::{
    FOS_FORCEFILESYSTEM, FOS_OVERWRITEPROMPT, FOS_PATHMUSTEXIST, FileOpenDialog, FileSaveDialog,
    IFileDialog, IFileOpenDialog, IFileSaveDialog, SIGDN_FILESYSPATH,
};
#[cfg(windows)]
use windows::core::{ComInterface, HRESULT, HSTRING, PCWSTR};

/// The runner-side error vocabulary. The runner maps this onto its own
/// `picker-error` enum, whose wire discriminants match this order exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerError {
    /// The user dismissed the dialog without selecting a path.
    Cancelled,
    /// The platform has no usable file-dialog stack (including on non-Windows
    /// hosts); nothing is faked in that case.
    NoWindow,
    /// The dialog could not be shown (a COM or Shell failure).
    Limit,
}

impl std::fmt::Display for PickerError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Self::Cancelled => "cancelled",
            Self::NoWindow => "no-window",
            Self::Limit => "limit",
        };
        formatter.write_str(text)
    }
}

/// One open/save filter: a display name plus a semicolon-separated pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickerFilter {
    /// The human-readable filter name shown in the dialog dropdown.
    pub name: String,
    /// The semicolon-separated extension pattern (for example `*.txt`).
    pub pattern: String,
}

/// The synchronous open/save surface the runner's [`crate::picker`]
/// `PathPicker` trait forwards to. Every call is dispatched onto the picker
/// thread and blocks until the user answers or the platform refuses.
pub trait PickerBackend: Send + Sync {
    /// Shows an open-file dialog. Returns the chosen path, or `Cancelled`.
    ///
    /// # Errors
    ///
    /// [`PickerError::Cancelled`] when the user dismisses the dialog;
    /// [`PickerError::NoWindow`] or [`PickerError::Limit`] when the platform
    /// cannot show it.
    fn open(&self, title: &str, filters: &[PickerFilter]) -> Result<String, PickerError>;

    /// Shows a save-file dialog with a suggested file name.
    ///
    /// # Errors
    ///
    /// As [`PickerBackend::open`].
    fn save(
        &self,
        title: &str,
        suggested_name: &str,
        filters: &[PickerFilter],
    ) -> Result<String, PickerError>;
}

/// A native file-dialog backend served by a dedicated STA thread.
///
/// Holds only the command channel and the worker thread handle — no COM
/// pointer crosses a thread, so it is legitimately `Send + Sync`.
pub struct NativePicker {
    commands: SyncSender<Command>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

enum Command {
    Open {
        title: String,
        filters: Vec<PickerFilter>,
        reply: SyncSender<Result<String, PickerError>>,
    },
    Save {
        title: String,
        suggested_name: String,
        filters: Vec<PickerFilter>,
        reply: SyncSender<Result<String, PickerError>>,
    },
    /// Tells the picker thread to exit; sent from [`Drop`] before joining so
    /// the blocking `recv` wakes up and the thread can be reaped.
    Close,
}

impl Drop for NativePicker {
    fn drop(&mut self) {
        // Send Close before joining so the thread's blocking `recv` wakes up
        // and exits; the sender is still held here, so dropping it alone
        // would not wake the thread and `join` would hang forever.
        let _ = self.commands.send(Command::Close);
        if let Some(thread) = self
            .thread
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
        {
            let _ = thread.join();
        }
    }
}

impl NativePicker {
    /// Starts the picker thread and initializes its STA.
    ///
    /// # Errors
    ///
    /// [`PickerError::NoWindow`] when the thread cannot be spawned (the same
    /// honest answer the Direct2D backend gives).
    #[cfg(windows)]
    pub fn start() -> Result<Self, PickerError> {
        let (commands, command_rx) = sync_channel(8);
        let (ready_tx, ready_rx) = sync_channel::<Result<(), PickerError>>(1);
        let thread = std::thread::Builder::new()
            .name("sico-ui-picker".to_owned())
            .spawn(move || picker_thread(&command_rx, &ready_tx))
            .map_err(|_| PickerError::NoWindow)?;
        // Wait for COM initialization to finish so the first call does not
        // race against it.
        ready_rx.recv().map_err(|_| PickerError::NoWindow)??;
        Ok(Self {
            commands,
            thread: Mutex::new(Some(thread)),
        })
    }

    /// The non-Windows answer: the platform has no usable file-dialog stack,
    /// so the picker thread is never started (ADR-0018 §Consequences).
    #[cfg(not(windows))]
    pub fn start() -> Result<Self, PickerError> {
        Err(PickerError::NoWindow)
    }
}

impl PickerBackend for NativePicker {
    fn open(&self, title: &str, filters: &[PickerFilter]) -> Result<String, PickerError> {
        let (reply_tx, reply_rx) = sync_channel(1);
        self.commands
            .send(Command::Open {
                title: title.to_owned(),
                filters: filters.to_vec(),
                reply: reply_tx,
            })
            .map_err(|_| PickerError::NoWindow)?;
        reply_rx.recv().map_err(|_| PickerError::NoWindow)?
    }

    fn save(
        &self,
        title: &str,
        suggested_name: &str,
        filters: &[PickerFilter],
    ) -> Result<String, PickerError> {
        let (reply_tx, reply_rx) = sync_channel(1);
        self.commands
            .send(Command::Save {
                title: title.to_owned(),
                suggested_name: suggested_name.to_owned(),
                filters: filters.to_vec(),
                reply: reply_tx,
            })
            .map_err(|_| PickerError::NoWindow)?;
        reply_rx.recv().map_err(|_| PickerError::NoWindow)?
    }
}

#[cfg(windows)]
fn picker_thread(command_rx: &Receiver<Command>, ready_tx: &SyncSender<Result<(), PickerError>>) {
    let initialized =
        unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) };
    let ready = if initialized.is_ok() {
        Ok(())
    } else {
        Err(PickerError::NoWindow)
    };
    let _ = ready_tx.send(ready);
    if ready.is_err() {
        return;
    }
    while let Ok(command) = command_rx.recv() {
        match command {
            Command::Open {
                title,
                filters,
                reply,
            } => {
                let _ = reply.send(run_dialog(false, &title, "", &filters));
            }
            Command::Save {
                title,
                suggested_name,
                filters,
                reply,
            } => {
                let _ = reply.send(run_dialog(true, &title, &suggested_name, &filters));
            }
            Command::Close => break,
        }
    }
    unsafe { CoUninitialize() };
}

/// Shows one dialog and returns its chosen path. The dialog is modal to no
/// owner window (`None`), so the runner can show it from any thread while a
/// UI window is also open.
#[cfg(windows)]
fn run_dialog(
    save: bool,
    title: &str,
    suggested_name: &str,
    filters: &[PickerFilter],
) -> Result<String, PickerError> {
    unsafe {
        // windows 0.52: there is no `From<IFileSaveDialog/IFileOpenDialog> for
        // IFileDialog`, so the concrete dialog is upcast to its base interface
        // with `ComInterface::cast` (the codex adapter relied on `Into`, which
        // the 0.61.3 bindings provided).
        let dialog: IFileDialog = if save {
            CoCreateInstance::<_, IFileSaveDialog>(&FileSaveDialog, None, CLSCTX_INPROC_SERVER)
                .map_err(|_| PickerError::Limit)?
                .cast::<IFileDialog>()
                .map_err(|_| PickerError::Limit)?
        } else {
            CoCreateInstance::<_, IFileOpenDialog>(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)
                .map_err(|_| PickerError::Limit)?
                .cast::<IFileDialog>()
                .map_err(|_| PickerError::Limit)?
        };
        dialog
            .SetTitle(&HSTRING::from(title))
            .map_err(|_| PickerError::Limit)?;
        if save && !suggested_name.is_empty() {
            dialog
                .SetFileName(&HSTRING::from(suggested_name))
                .map_err(|_| PickerError::Limit)?;
        }
        dialog
            .SetOptions(
                dialog.GetOptions().map_err(|_| PickerError::Limit)?
                    | FOS_FORCEFILESYSTEM
                    | if save {
                        FOS_OVERWRITEPROMPT
                    } else {
                        FOS_PATHMUSTEXIST
                    },
            )
            .map_err(|_| PickerError::Limit)?;
        if !filters.is_empty() {
            // Keep the HSTRINGs alive for as long as the COMDLG_FILTERSPEC
            // pointer slices reference them (the dialog copies them during
            // SetFileTypes).
            let hstrings: Vec<(HSTRING, HSTRING)> = filters
                .iter()
                .map(|filter| (HSTRING::from(&filter.name), HSTRING::from(&filter.pattern)))
                .collect();
            // windows 0.52: `HSTRING::as_ptr()` returns `*const u16`, so the
            // filter spec fields are rebuilt as `PCWSTR` (the codex adapter
            // targeted 0.61.3 where `as_ptr()` returned `PCWSTR` directly).
            let specs: Vec<COMDLG_FILTERSPEC> = hstrings
                .iter()
                .map(|(name, pattern)| COMDLG_FILTERSPEC {
                    pszName: PCWSTR(name.as_ptr()),
                    pszSpec: PCWSTR(pattern.as_ptr()),
                })
                .collect();
            dialog
                .SetFileTypes(&specs)
                .map_err(|_| PickerError::Limit)?;
        }
        if let Err(error) = dialog.Show(None) {
            // Only `HRESULT_FROM_WIN32(ERROR_CANCELLED)` means the user
            // dismissed the dialog; any other failure is a genuine platform
            // error and must not be reported as a cancellation.
            if error.code() == HRESULT::from_win32(ERROR_CANCELLED.0) {
                return Err(PickerError::Cancelled);
            }
            return Err(PickerError::Limit);
        }
        let shell_item = dialog.GetResult().map_err(|_| PickerError::Limit)?;
        let display = shell_item
            .GetDisplayName(SIGDN_FILESYSPATH)
            .map_err(|_| PickerError::Limit)?;
        // `PWSTR`'s `to_string` decodes the null-terminated UTF-16 buffer. A
        // filesystem path cannot contain a lone surrogate, so this conversion
        // is lossless in practice; an undecodable buffer surfaces as a
        // `Limit` rather than a fake path.
        let path = display.to_string();
        // D4 fix (STEP-0627): `GetDisplayName(SIGDN_FILESYSPATH)` allocates
        // the returned `PWSTR` with `CoTaskMemAlloc`. The codex adapter leaked
        // that buffer on every dialog; here it is freed **unconditionally**,
        // on both the Ok and the Err path of the `to_string` conversion,
        // before the result is returned. `display.0` is the `*mut u16` the COM
        // API allocated; `CoTaskMemFree` takes it as `Option<*const c_void>`.
        // (This call is already inside the enclosing `unsafe` block.)
        CoTaskMemFree(Some(display.0.cast::<std::ffi::c_void>().cast_const()));
        path.map_err(|_| PickerError::Limit)
    }
}

/// The non-Windows answer: the platform has no usable file-dialog stack.
///
/// ADR-0018 §Consequences: v0 is Windows-only; elsewhere the honest answer is
/// that no native backend exists, never a faked path.
#[cfg(not(windows))]
pub fn native_picker() -> Result<NativePicker, PickerError> {
    Err(PickerError::NoWindow)
}

/// Windows entry: starts the native STA picker thread.
///
/// # Errors
///
/// [`PickerError::NoWindow`] when the thread cannot be spawned or the COM
/// initialization fails.
#[cfg(windows)]
pub fn native_picker() -> Result<NativePicker, PickerError> {
    NativePicker::start()
}
