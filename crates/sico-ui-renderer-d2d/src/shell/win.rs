//! The Win32 window shell and event pump.
//!
//! This module is the **second** unsafe module in the crate (the renderer is
//! the first; ADR-0018 §1 keeps platform-specific unsafe code inside the
//! smallest isolated adapter and the shared core unsafe-free). It owns:
//!
//! - the window class registration (one class per process, via a `OnceLock`);
//! - the `HWND` lifecycle: `CreateWindowExW` / `DestroyWindow`, with the
//!   client area sized to the requested 96-DPI logical dimensions;
//! - the message pump: `PeekMessageW` + `TranslateMessage` +
//!   `DispatchMessageW`, pump-once and drain, mapping `WM_*` messages through
//!   [`translate`](crate::events::translate);
//! - the present path: a rendered frame is copied **verbatim** into a GDI DIB
//!   (`BITMAPV5HEADER` + `BI_BITFIELDS` + explicit BGRA masks — the exact DIB
//!   form the Direct2D renderer requires, measured in STEP-0338) and blitted
//!   into the window DC with `BitBlt`. The frame bytes are never recomputed.
//!
//! Safety model: the window state lives in a boxed [`ShellState`] whose
//! pointer is handed to the static `WndProc` through `GWLP_USERDATA`. The
//! `Box` is owned by [`WinShell`] and outlives the `HWND`; the `WndProc` only
//! runs on the creating thread while the `Box` is alive, and `WM_NCDESTROY`
//! nulls the pointer before the `Box` is freed. The queue is pre-allocated to
//! [`MAX_QUEUE_EVENTS`] and a push is rejected (and reported) before any new
//! allocation.

#![allow(clippy::cast_sign_loss)]

use std::collections::VecDeque;
use std::ffi::c_void;
use std::marker::PhantomData;
use std::sync::OnceLock;

use windows::Win32::Foundation::{HANDLE, HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BI_BITFIELDS, BITMAPINFO, BITMAPV5HEADER, BeginPaint, BitBlt, CreateCompatibleDC,
    CreateDIBSection, DIB_USAGE, DeleteDC, DeleteObject, EndPaint, GetDC, HBITMAP, HBRUSH, HDC,
    HGDIOBJ, PAINTSTRUCT, ReleaseDC, SRCCOPY, SelectObject,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::{DPI_AWARENESS_CONTEXT_UNAWARE, SetProcessDpiAwarenessContext};
use windows::Win32::UI::WindowsAndMessaging::{
    AdjustWindowRectEx, CREATESTRUCTW, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, CreateWindowExW,
    DefWindowProcW, DestroyWindow, DispatchMessageW, GWLP_USERDATA, GetClientRect,
    GetWindowLongPtrW, HICON, HMENU, IDC_ARROW, IsWindow, LoadCursorW, MSG, PM_REMOVE,
    PeekMessageW, PostMessageW, RegisterClassW, SetWindowLongPtrW, TranslateMessage,
    WINDOW_EX_STYLE, WM_CLOSE, WM_LBUTTONDOWN, WM_NCCREATE, WM_NCDESTROY, WM_PAINT, WM_QUIT,
    WM_SIZE, WNDCLASSW, WS_OVERLAPPEDWINDOW,
};
use windows::core::PCWSTR;

use crate::error::UiBackendError;
use crate::events::{MAX_QUEUE_EVENTS, RawMessage, WindowEvent, translate};
use crate::frame::Frame;

/// The registered window class name.
const CLASS_NAME: &str = "sico-ui-renderer-d2d-v0";

/// `LCS_sRGB` — the logical colour space of the DIB (same as the renderer).
const LCS_SRGB: u32 = 0x7352_4742;

/// The boxed per-window state the `WndProc` mutates through `GWLP_USERDATA`.
struct ShellState {
    /// The bounded, pre-allocated delivered-event queue.
    queue: VecDeque<WindowEvent>,
    /// True when the queue cap was hit since the last drain.
    overflow: bool,
    /// The last known client size in 96-DPI logical pixels.
    client: (u32, u32),
    /// True once `WM_NCDESTROY` arrived (the window is gone).
    destroyed: bool,
}

impl ShellState {
    fn new() -> Self {
        Self {
            queue: VecDeque::with_capacity(MAX_QUEUE_EVENTS),
            overflow: false,
            client: (0, 0),
            destroyed: false,
        }
    }

    /// Queues one event, coalescing by kind and reporting an overflow instead
    /// of dropping an event silently (RFC-0052 §3: coalesce by kind, never
    /// drop silently into wrong delivery).
    fn push(&mut self, event: WindowEvent) {
        // `Closed` is terminal: once queued, later events carry no meaning
        // and are ignored, and a duplicate `Closed` coalesces to one.
        if matches!(event, WindowEvent::Closed) {
            if !self.queue.iter().any(|e| matches!(e, WindowEvent::Closed)) {
                if self.queue.len() >= MAX_QUEUE_EVENTS {
                    self.queue.pop_front();
                    self.overflow = true;
                }
                self.queue.push_back(event);
            }
            return;
        }
        if self.queue.iter().any(|e| matches!(e, WindowEvent::Closed)) {
            return;
        }
        if self.queue.len() < MAX_QUEUE_EVENTS {
            self.queue.push_back(event);
            return;
        }
        match event {
            // A repaint request coalesces losslessly onto a pending one.
            WindowEvent::Paint => {
                if !self.queue.iter().any(|e| matches!(e, WindowEvent::Paint)) {
                    self.overflow = true;
                }
            }
            // A resize coalesces onto a pending one (latest size wins).
            WindowEvent::Resized { .. } => {
                if let Some(slot) = self
                    .queue
                    .iter_mut()
                    .find(|e| matches!(e, WindowEvent::Resized { .. }))
                {
                    *slot = event;
                } else {
                    self.overflow = true;
                }
            }
            // A click is a point event; it cannot coalesce, so a full queue
            // is reported (never silent).
            WindowEvent::Clicked { .. } => self.overflow = true,
            WindowEvent::Closed => {}
        }
    }

    fn take_overflow(&mut self) -> bool {
        std::mem::take(&mut self.overflow)
    }
}

/// The held GDI DIB used to present frames.
struct Dib {
    dc: HDC,
    bitmap: HBITMAP,
    bits: *mut c_void,
    width: u32,
    height: u32,
}

impl Drop for Dib {
    fn drop(&mut self) {
        // SAFETY: the DC and the bitmap are owned here; the DC is deleted
        // first so the bitmap is no longer selected when it is deleted.
        unsafe {
            DeleteDC(self.dc);
            DeleteObject(HGDIOBJ(self.bitmap.0));
        }
    }
}

/// The BGRA top-down DIB description (`BITMAPV5HEADER` + `BI_BITFIELDS`).
#[repr(C)]
struct BitmapInfoV5 {
    header: BITMAPV5HEADER,
    masks: [u32; 3],
}

impl BitmapInfoV5 {
    fn new(width: i32, height: i32) -> Self {
        let mut info = Self {
            header: BITMAPV5HEADER::default(),
            masks: [0x00FF_0000, 0x0000_FF00, 0x0000_00FF],
        };
        info.header.bV5Size =
            u32::try_from(std::mem::size_of::<BITMAPV5HEADER>()).expect("BITMAPV5HEADER fits u32");
        info.header.bV5Width = width;
        info.header.bV5Height = -height;
        info.header.bV5Planes = 1;
        info.header.bV5BitCount = 32;
        info.header.bV5Compression = BI_BITFIELDS;
        info.header.bV5RedMask = 0x00FF_0000;
        info.header.bV5GreenMask = 0x0000_FF00;
        info.header.bV5BlueMask = 0x0000_00FF;
        info.header.bV5AlphaMask = 0xFF00_0000;
        info.header.bV5CSType = LCS_SRGB;
        info
    }
}

fn render_error(stage: &str, error: &windows::core::Error) -> UiBackendError {
    UiBackendError::RenderFailed {
        detail: format!("{stage}: {}", error.code().0),
    }
}

/// Runs `body` against the live per-window state, if the window still has one.
fn with_state(hwnd: HWND, body: impl FnOnce(&mut ShellState)) {
    let state_ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut ShellState };
    if !state_ptr.is_null() {
        // SAFETY: the pointer was stored on `WM_NCCREATE` and points into the
        // `Box` owned by the `WinShell`; the `Box` is only freed after the
        // `HWND` is destroyed (the pointer is nulled on `WM_NCDESTROY` first),
        // so it is alive for the whole of this call on the creating thread.
        let state = unsafe { &mut *state_ptr };
        body(state);
    }
}

/// The static window procedure.
///
/// Every message is handled on the creating thread while the boxed
/// [`ShellState`] is alive; the pointer is nulled on `WM_NCDESTROY` before the
/// `Box` is freed.
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
#[allow(clippy::too_many_lines)]
unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // SAFETY: this is the window procedure registered with the class. The
    // `HWND`, `WPARAM` and `LPARAM` come from the OS; the boxed state pointer
    // in `GWLP_USERDATA` (if any) is owned by the `WinShell` for the window's
    // whole lifetime and is nulled on `WM_NCDESTROY` before the `Box` is
    // freed, so every dereference below is valid on the creating thread.
    unsafe {
        match msg {
            WM_NCCREATE => {
                // `lParam` points to the `CREATESTRUCTW` whose `lpCreateParams`
                // is the boxed state pointer we passed to `CreateWindowExW`.
                let create = &*(lparam.0 as *const CREATESTRUCTW);
                let state_ptr = create.lpCreateParams.cast::<ShellState>();
                let _ = SetWindowLongPtrW(hwnd, GWLP_USERDATA, state_ptr as isize);
                LRESULT(1)
            }
            WM_CLOSE => {
                with_state(hwnd, |state| state.push(WindowEvent::Closed));
                // `DefWindowProcW` turns `WM_CLOSE` into `DestroyWindow`, which
                // sends `WM_DESTROY` (→ `PostQuitMessage`) and `WM_NCDESTROY`.
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
            WM_SIZE => {
                let width = low_word(lparam.0);
                let height = high_word(lparam.0);
                with_state(hwnd, |state| {
                    state.client = (width, height);
                    if let Some(event) = translate(RawMessage::Resize { width, height }) {
                        state.push(event);
                    }
                });
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
            WM_PAINT => {
                // Validate the damaged region so `WM_PAINT` does not re-fire.
                // `BeginPaint` returns the paint DC directly in the `windows`
                // 0.52 binding; a null DC means the begin failed.
                let mut ps = PAINTSTRUCT::default();
                let paint_dc = BeginPaint(hwnd, &raw mut ps);
                if paint_dc.0 != 0 {
                    // SAFETY: `EndPaint` pairs with a successful `BeginPaint`
                    // on the same window and `PAINTSTRUCT`; the returned paint
                    // DC is released by `EndPaint` (never released again here).
                    //
                    // The `EndPaint` result is intentionally ignored: it is
                    // the last step of the paint tear-down, there is no
                    // recovery action and no dereference follows, and the OS
                    // reclaims the paint state when the window is destroyed —
                    // an ignored result here cannot make a later use unsafe
                    // (STEP-0585 low (b), AGENTS §7).
                    let _ = EndPaint(hwnd, &raw const ps);
                } else {
                    // STEP-0585 low (b): a failed `BeginPaint` leaves no valid
                    // paint state, so `EndPaint` must NOT be called (an
                    // invalid `PAINTSTRUCT` is Win32 API misuse). The damaged
                    // region is not validated, so `WM_PAINT` may re-fire — the
                    // honest Win32 behaviour. The repaint need is still
                    // reported as a `Paint` event below.
                }
                with_state(hwnd, |state| state.push(WindowEvent::Paint));
                LRESULT(0)
            }
            WM_LBUTTONDOWN => {
                let x = i32::from(low_word(lparam.0) as u16 as i16);
                let y = i32::from(high_word(lparam.0) as u16 as i16);
                with_state(hwnd, |state| {
                    if let Some(event) = translate(RawMessage::LeftButtonDown { x, y }) {
                        state.push(event);
                    }
                });
                LRESULT(0)
            }
            WM_NCDESTROY => {
                let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut ShellState;
                if !state_ptr.is_null() {
                    // SAFETY: as above; the pointer is still valid here.
                    (*state_ptr).destroyed = true;
                }
                let _ = SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

/// The low 16 bits of a packed message `LPARAM`, as an unsigned width.
fn low_word(value: isize) -> u32 {
    (value as u64 & 0xFFFF) as u32
}

/// The high 16 bits of a packed message `LPARAM`, as an unsigned height.
fn high_word(value: isize) -> u32 {
    ((value as u64 >> 16) & 0xFFFF) as u32
}

/// Registers the window class once per process.
fn register_class() -> Result<u16, UiBackendError> {
    static ATOM: OnceLock<Result<u16, ()>> = OnceLock::new();
    let atom = ATOM.get_or_init(register_class_inner);
    atom.as_ref()
        .map_err(|()| UiBackendError::RenderFailed {
            detail: "window-class-registration".to_owned(),
        })
        .copied()
}

fn register_class_inner() -> Result<u16, ()> {
    // SAFETY: every call below is a documented Win32 entry point.
    unsafe {
        let instance = GetModuleHandleW(None).map_err(|_| ())?;
        let cursor = LoadCursorW(None, IDC_ARROW).map_err(|_| ())?;
        let class_name: Vec<u16> = CLASS_NAME
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: HINSTANCE(instance.0),
            hIcon: HICON(0),
            hCursor: cursor,
            hbrBackground: HBRUSH(0),
            lpszMenuName: PCWSTR::null(),
            lpszClassName: PCWSTR::from_raw(class_name.as_ptr()),
        };
        let atom = RegisterClassW(&raw const class);
        if atom == 0 {
            return Err(());
        }
        Ok(atom)
    }
}

/// The native shell on Windows.
///
/// Thread affinity is a **deliberate** invariant (STEP-0585, from the
/// STEP-0577 independent safety review): a Win32 window's message pump must
/// run on the thread that created the window, so the shell must never cross a
/// thread. The `windows` 0.52 handle types (`HWND`, `HDC`, `HBITMAP`, ...)
/// are `pub struct(pub isize)` and are themselves `Send + Sync`, so the
/// affinity must **not** rest on an accidental field (the DIB's raw `bits`
/// pointer once carried it; a refactor that swaps the DIB to owned storage
/// would silently make the shell `Send + Sync`). [`PhantomData<*mut ()>`] is
/// `!Send + !Sync` and pins the invariant, and the test module asserts both
/// the pin and that a marker-free mirror would be `Send + Sync`.
pub(crate) struct WinShell {
    hwnd: Option<HWND>,
    state: Option<Box<ShellState>>,
    dib: Option<Dib>,
    quit: bool,
    /// Deliberate `!Send + !Sync` pin; never read, never leaves the thread.
    _not_send_sync: PhantomData<*mut ()>,
}

impl WinShell {
    pub(crate) fn new() -> Result<Self, UiBackendError> {
        let _ = register_class()?;
        // Best-effort DPI-unawareness so the client area stays in the 96-DPI
        // logical pixels the frozen token table uses. If the process already
        // pinned a different awareness this fails and that awareness wins; no
        // coordinate-determinism claim is made in that case.
        unsafe {
            let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_UNAWARE);
        }
        Ok(Self {
            hwnd: None,
            state: None,
            dib: None,
            quit: false,
            _not_send_sync: PhantomData,
        })
    }

    pub(crate) fn create_window(
        &mut self,
        title: &str,
        width: u32,
        height: u32,
    ) -> Result<(), UiBackendError> {
        if self.hwnd.is_some() {
            return Err(UiBackendError::WrongParentKind);
        }
        let atom = register_class()?;
        let instance = unsafe { GetModuleHandleW(None) }
            .map_err(|error| render_error("GetModuleHandleW", &error))?;
        let instance = HINSTANCE(instance.0);
        let class_name: Vec<u16> = CLASS_NAME
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let title_utf16: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
        // Size the outer window so the client area is exactly width×height in
        // 96-DPI logical pixels.
        let mut rect = RECT {
            left: 0,
            top: 0,
            right: i32::try_from(width).map_err(|_| UiBackendError::InvalidArgument {
                which: "window-size",
            })?,
            bottom: i32::try_from(height).map_err(|_| UiBackendError::InvalidArgument {
                which: "window-size",
            })?,
        };
        unsafe {
            AdjustWindowRectEx(
                &raw mut rect,
                WS_OVERLAPPEDWINDOW,
                false,
                WINDOW_EX_STYLE(0),
            )
        }
        .map_err(|error| render_error("AdjustWindowRectEx", &error))?;

        let mut state = Box::new(ShellState::new());
        let state_ptr: *mut ShellState = &raw mut *state;
        let hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                PCWSTR::from_raw(class_name.as_ptr()),
                PCWSTR::from_raw(title_utf16.as_ptr()),
                WS_OVERLAPPEDWINDOW,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                rect.right - rect.left,
                rect.bottom - rect.top,
                HWND::default(),
                HMENU::default(),
                instance,
                Some(state_ptr.cast::<c_void>()),
            )
        };
        if hwnd.0 == 0 {
            // A partially created window is destroyed by the OS (and its
            // `WM_NCDESTROY` nulls the user-data pointer), so the boxed state
            // is safe to drop here.
            return Err(UiBackendError::RenderFailed {
                detail: "CreateWindowExW returned a null handle".to_owned(),
            });
        }
        let _ = atom;
        self.state = Some(state);
        self.hwnd = Some(hwnd);
        Ok(())
    }

    pub(crate) fn pump_once(&mut self) -> Option<WindowEvent> {
        // Deliver a queued event first; only touch the OS queue when empty.
        if let Some(event) = self.state.as_mut().and_then(|s| s.queue.pop_front()) {
            return Some(event);
        }
        loop {
            let destroyed = self.state.as_ref().is_some_and(|s| s.destroyed);
            if destroyed {
                // The window is gone (`WM_CLOSE` → `DestroyWindow`). The
                // default `WM_DESTROY` handling posts `WM_QUIT`, which is
                // queued by now; peek it once (non-blocking) so the loop
                // terminates deterministically instead of spinning on the
                // destroyed flag and never observing `WM_QUIT`.
                let mut quit_msg = MSG::default();
                // SAFETY: `MSG` is a plain value; `PeekMessageW` fills it in.
                if unsafe { PeekMessageW(&raw mut quit_msg, None, 0, 0, PM_REMOVE) }.as_bool()
                    && quit_msg.message == WM_QUIT
                {
                    self.quit = true;
                }
                return None;
            }
            let mut msg = MSG::default();
            // SAFETY: `MSG` is a plain value; `PeekMessageW` fills it in.
            if !unsafe { PeekMessageW(&raw mut msg, None, 0, 0, PM_REMOVE) }.as_bool() {
                return None;
            }
            if msg.message == WM_QUIT {
                self.quit = true;
                return None;
            }
            // SAFETY: the message is live and the dispatch delivers it to the
            // `WndProc`, which mutates the state through `GWLP_USERDATA`.
            unsafe {
                TranslateMessage(&raw const msg);
                DispatchMessageW(&raw const msg);
            }
            if let Some(event) = self.state.as_mut().and_then(|s| s.queue.pop_front()) {
                return Some(event);
            }
            if self
                .hwnd
                .is_some_and(|hwnd| unsafe { !IsWindow(hwnd).as_bool() })
            {
                self.hwnd = None;
            }
        }
    }

    pub(crate) fn drain(&mut self, max_events: usize) -> (Vec<WindowEvent>, bool) {
        let mut events = Vec::new();
        for _ in 0..max_events {
            match self.pump_once() {
                Some(event) => events.push(event),
                None => break,
            }
        }
        let overflow = self.state.as_mut().is_some_and(|s| s.take_overflow());
        (events, overflow)
    }

    pub(crate) fn present_frame(&mut self, frame: &Frame) -> Result<(), UiBackendError> {
        let hwnd = self.hwnd.ok_or(UiBackendError::NoWindow)?;
        let window_dc = unsafe { GetDC(hwnd) };
        if window_dc.0 == 0 {
            return Err(UiBackendError::RenderFailed {
                detail: "GetDC returned a null DC".to_owned(),
            });
        }
        let result = self.present_frame_into(window_dc, frame);
        unsafe {
            ReleaseDC(hwnd, window_dc);
        }
        result
    }

    fn present_frame_into(&mut self, window_dc: HDC, frame: &Frame) -> Result<(), UiBackendError> {
        self.ensure_dib(frame.width(), frame.height(), window_dc)?;
        let dib = self.dib.as_ref().ok_or(UiBackendError::RenderFailed {
            detail: "present DIB missing".to_owned(),
        })?;
        let dib_dc = dib.dc;
        let bits = dib.bits;
        // SAFETY: the DIB is alive and its `bits` buffer is `width*height*4`
        // bytes, which matches the validated frame payload length exactly.
        unsafe {
            std::ptr::copy_nonoverlapping(
                frame.pixels().as_ptr(),
                bits.cast::<u8>(),
                frame.pixels().len(),
            );
            BitBlt(
                window_dc,
                0,
                0,
                i32::try_from(frame.width()).map_err(|_| UiBackendError::InvalidArgument {
                    which: "frame-size",
                })?,
                i32::try_from(frame.height()).map_err(|_| UiBackendError::InvalidArgument {
                    which: "frame-size",
                })?,
                dib_dc,
                0,
                0,
                SRCCOPY,
            )
            .map_err(|error| render_error("BitBlt", &error))
        }
    }

    fn ensure_dib(
        &mut self,
        width: u32,
        height: u32,
        window_dc: HDC,
    ) -> Result<(), UiBackendError> {
        if let Some(dib) = &self.dib
            && dib.width == width
            && dib.height == height
        {
            return Ok(());
        }
        self.recreate_dib(width, height, window_dc)
    }

    fn recreate_dib(
        &mut self,
        width: u32,
        height: u32,
        window_dc: HDC,
    ) -> Result<(), UiBackendError> {
        self.dib = None;
        let mem_dc = unsafe { CreateCompatibleDC(window_dc) };
        if mem_dc.0 == 0 {
            return Err(UiBackendError::RenderFailed {
                detail: "CreateCompatibleDC returned a null DC".to_owned(),
            });
        }
        let info = BitmapInfoV5::new(
            i32::try_from(width).map_err(|_| UiBackendError::InvalidArgument {
                which: "frame-size",
            })?,
            i32::try_from(height).map_err(|_| UiBackendError::InvalidArgument {
                which: "frame-size",
            })?,
        );
        let bmi: *const BITMAPINFO = (&raw const info.header).cast::<BITMAPINFO>();
        let mut bits: *mut c_void = std::ptr::null_mut();
        let bitmap =
            unsafe { CreateDIBSection(mem_dc, bmi, DIB_USAGE(0), &raw mut bits, HANDLE(0), 0) };
        let bitmap = match bitmap {
            Ok(bitmap) => bitmap,
            Err(error) => {
                unsafe {
                    DeleteDC(mem_dc);
                }
                return Err(render_error("CreateDIBSection", &error));
            }
        };
        unsafe {
            SelectObject(mem_dc, HGDIOBJ(bitmap.0));
        }
        self.dib = Some(Dib {
            dc: mem_dc,
            bitmap,
            bits,
            width,
            height,
        });
        Ok(())
    }

    pub(crate) fn read_presented_frame(&self) -> Result<Frame, UiBackendError> {
        let dib = self.dib.as_ref().ok_or(UiBackendError::NoWindow)?;
        let byte_len = (dib.width as usize)
            .checked_mul(dib.height as usize)
            .and_then(|area| area.checked_mul(4))
            .ok_or(UiBackendError::RenderFailed {
                detail: "present frame size overflow".to_owned(),
            })?;
        // SAFETY: the DIB is alive and its `bits` buffer is `byte_len` bytes
        // (the exact payload a `Frame` of this size holds).
        let pixels =
            unsafe { std::slice::from_raw_parts(dib.bits.cast::<u8>(), byte_len).to_vec() };
        Frame::new(dib.width, dib.height, pixels)
            .map_err(|detail| UiBackendError::RenderFailed { detail })
    }

    pub(crate) fn close(&self) -> Result<(), UiBackendError> {
        let hwnd = self.hwnd.ok_or(UiBackendError::NoWindow)?;
        unsafe { PostMessageW(hwnd, WM_CLOSE, WPARAM(0), LPARAM(0)) }
            .map_err(|error| render_error("PostMessageW", &error))
    }

    pub(crate) fn should_quit(&self) -> bool {
        self.quit
    }

    pub(crate) fn client_size(&self) -> Result<(u32, u32), UiBackendError> {
        let hwnd = self.hwnd.ok_or(UiBackendError::NoWindow)?;
        let mut rect = RECT::default();
        unsafe { GetClientRect(hwnd, &raw mut rect) }
            .map_err(|error| render_error("GetClientRect", &error))?;
        Ok((
            u32::try_from(rect.right - rect.left).map_err(|_| UiBackendError::InvalidArgument {
                which: "window-size",
            })?,
            u32::try_from(rect.bottom - rect.top).map_err(|_| UiBackendError::InvalidArgument {
                which: "window-size",
            })?,
        ))
    }

    pub(crate) fn destroy(&mut self) -> Result<(), UiBackendError> {
        let hwnd = self.hwnd.ok_or(UiBackendError::NoWindow)?;
        // SAFETY: `IsWindow` is a documented Win32 entry point.
        if unsafe { !IsWindow(hwnd).as_bool() } {
            // The window is already gone (the `WM_CLOSE` → `DestroyWindow`
            // path destroyed it and `WM_NCDESTROY` orphaned the boxed state);
            // destroy is idempotent, so clear the shell and report success
            // instead of double-destroying the handle.
            self.hwnd = None;
            self.state = None;
            self.dib = None;
            return Ok(());
        }
        // On success, `DestroyWindow` synchronously sends `WM_DESTROY` and
        // `WM_NCDESTROY`, which nulls the `GWLP_USERDATA` pointer before the
        // boxed state below is freed. On failure the `?` returns early with
        // `self.hwnd` and `self.state` intact, so the boxed state stays alive
        // and still reachable through `GWLP_USERDATA` — a failed destroy
        // never frees memory a live window references (STEP-0585 low (a)).
        unsafe { DestroyWindow(hwnd) }.map_err(|error| render_error("DestroyWindow", &error))?;
        self.hwnd = None;
        // The `WM_NCDESTROY` has nulled the user-data pointer; the boxed state
        // is no longer reachable and can be freed here.
        self.state = None;
        self.dib = None;
        Ok(())
    }
}

impl Drop for WinShell {
    fn drop(&mut self) {
        if let Some(hwnd) = self.hwnd {
            // SAFETY: guard against a handle the OS already destroyed; the
            // boxed state is still alive for a live window.
            let alive = unsafe { IsWindow(hwnd).as_bool() };
            if alive {
                // SAFETY: the handle is alive, so `DestroyWindow` is a
                // documented Win32 entry point on a live window.
                let destroyed = unsafe { DestroyWindow(hwnd) }.is_ok();
                if !destroyed {
                    // STEP-0585 low (a): the window survived the destroy
                    // request, but its `GWLP_USERDATA` still points into the
                    // boxed `ShellState`. `Drop` cannot return a typed error,
                    // so the boxed state is **deliberately leaked** (never
                    // freed) rather than released while a live window still
                    // references it — freeing it would leave `WndProc` with a
                    // dangling `GWLP_USERDATA` dereference (UB). A later
                    // `DestroyWindow` on the still-live window is the only
                    // correct teardown; leaking the small boxed state is the
                    // memory-safe choice. The DIB and the handle are plain
                    // owned values and drop normally.
                    std::mem::forget(self.state.take());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clicked(index: usize) -> WindowEvent {
        WindowEvent::Clicked {
            x: i32::try_from(index).unwrap_or(0),
            y: i32::try_from(index).unwrap_or(0),
        }
    }

    #[test]
    fn a_full_queue_reports_an_overflow_instead_of_silently_dropping() {
        let mut state = ShellState::new();
        for index in 0..MAX_QUEUE_EVENTS {
            state.push(clicked(index));
            assert!(!state.overflow, "no overflow below the cap");
        }
        assert_eq!(state.queue.len(), MAX_QUEUE_EVENTS);
        // A point event cannot coalesce; the cap overrun is reported, never
        // dropped silently into a wrong delivery.
        state.push(clicked(MAX_QUEUE_EVENTS));
        assert!(state.overflow, "the overrun must be reported");
        assert_eq!(state.queue.len(), MAX_QUEUE_EVENTS);
        assert!(state.take_overflow());
        assert!(!state.overflow, "the overflow flag is one-shot");
    }

    #[test]
    fn a_paint_with_no_pending_paint_is_reported_not_silently_lost() {
        let mut state = ShellState::new();
        for index in 0..MAX_QUEUE_EVENTS {
            state.push(clicked(index));
        }
        state.push(WindowEvent::Paint);
        assert!(
            state.overflow,
            "a repaint with no pending paint is reported, not silently lost"
        );
        assert!(
            !state
                .queue
                .iter()
                .any(|event| matches!(event, WindowEvent::Paint)),
            "the dropped repaint is never silently added"
        );
    }

    #[test]
    fn a_paint_coalesces_losslessly_onto_a_pending_one() {
        let mut state = ShellState::new();
        state.push(WindowEvent::Paint);
        for index in 0..(MAX_QUEUE_EVENTS - 1) {
            state.push(clicked(index));
        }
        assert_eq!(state.queue.len(), MAX_QUEUE_EVENTS);
        // A second repaint with a pending Paint is coalesced losslessly.
        state.push(WindowEvent::Paint);
        assert!(!state.overflow, "a coalesced repaint is not an overflow");
        assert_eq!(
            state
                .queue
                .iter()
                .filter(|event| matches!(event, WindowEvent::Paint))
                .count(),
            1,
            "the queue holds exactly one Paint"
        );
    }

    #[test]
    fn a_resize_coalesces_onto_a_pending_one_latest_wins() {
        let mut state = ShellState::new();
        state.push(WindowEvent::Resized {
            width: 100,
            height: 50,
        });
        for index in 0..(MAX_QUEUE_EVENTS - 1) {
            state.push(clicked(index));
        }
        assert_eq!(state.queue.len(), MAX_QUEUE_EVENTS);
        state.push(WindowEvent::Resized {
            width: 200,
            height: 100,
        });
        assert!(!state.overflow, "a coalesced resize is not an overflow");
        assert!(
            state.queue.iter().any(|event| *event
                == WindowEvent::Resized {
                    width: 200,
                    height: 100
                }),
            "the latest size wins"
        );
    }

    #[test]
    fn closed_is_terminal_and_idempotent() {
        let mut state = ShellState::new();
        state.push(WindowEvent::Closed);
        state.push(WindowEvent::Closed);
        assert_eq!(
            state
                .queue
                .iter()
                .filter(|event| **event == WindowEvent::Closed)
                .count(),
            1,
            "a duplicate Closed coalesces to one"
        );
        // After Closed, later events carry no meaning and are ignored.
        state.push(WindowEvent::Paint);
        state.push(clicked(1));
        assert!(
            state
                .queue
                .iter()
                .all(|event| *event == WindowEvent::Closed),
            "no event after Closed is delivered"
        );
        assert!(!state.overflow);
    }

    /// Compile-time negative trait assertion: proves `$x` implements **neither**
    /// `Send` nor `Sync`.
    ///
    /// Hand-rolled `static_assertions`-style `assert_not_impl_any!` (no new
    /// dependency, STEP-0585): [`AmbiguousIfImpl`] has a blanket impl for every
    /// type at `A = ()` and a second impl for any type implementing the
    /// asserted trait at `A = Invalid`. If `$x` implements the asserted trait,
    /// `<T as AmbiguousIfImpl<_>>::some_item` has two candidate impls and the
    /// inference of `_` is ambiguous — a compile error. If it does **not**, only
    /// the blanket applies and the invocation compiles.
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

    // Compile-time proof that `WinShell` is `!Send + !Sync`. If a refactor ever
    // makes it `Send` or `Sync`, this fails the build (STEP-0585).
    assert_not_impl_any!(WinShell: Send, Sync);

    /// The field set `WinShell` would have if a refactor swapped the DIB's raw
    /// `bits` pointer for owned storage (`Vec<u8>`/`usize`) **and** the
    /// deliberate `!Send + !Sync` marker were absent. Every field is `Send +
    /// Sync` (the `windows` 0.52 handles are `pub struct(pub isize)` wrappers
    /// that are themselves `Send + Sync`), so this mirror is `Send + Sync` —
    /// exactly the silent soundness regression the STEP-0577 review flagged. It
    /// is the compile-time proof that the deliberate marker, not an accidental
    /// pointer field, carries the thread-affinity invariant. The fields are
    /// never read (the mirror is used only as a `Send + Sync` witness), so
    /// `dead_code` is allowed deliberately.
    #[allow(dead_code)]
    struct WinShellMarkerFreeMirror {
        hwnd: Option<HWND>,
        state: Option<Box<ShellState>>,
        pixels: Option<Vec<u8>>,
        quit: bool,
    }

    /// Compile-time proof that `T` is `Send + Sync` (used only on the mirror).
    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn winshell_is_deliberately_not_send_sync() {
        // The real evidence is the `assert_not_impl_any!(WinShell: Send, Sync)`
        // invocation at module scope: a compile-time check that fails the build
        // if `WinShell` is `Send` or `Sync`. This test re-confirms the
        // deliberate marker is zero-sized (it must not cost a field).
        assert_eq!(std::mem::size_of::<PhantomData<*mut ()>>(), 0);
    }

    #[test]
    fn marker_free_refactor_mirror_would_be_send_sync() {
        // The mirror (owned DIB storage, no deliberate marker) is `Send + Sync`;
        // the real `WinShell` is `!Send + !Sync` (asserted above at compile
        // time). Together these prove the marker is the only thing keeping the
        // shell thread-affine, so a refactor cannot silently turn it `Send +
        // Sync` (STEP-0585).
        assert_send_sync::<WinShellMarkerFreeMirror>();
    }
}
