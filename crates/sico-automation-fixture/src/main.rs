//! M16 fixture app (M16 plan §5.4): a deterministic Windows window with
//! scripted state — two buttons (increment/reset) and the counter rendered
//! as `count=<n>` in the client area. No anti-automation hardening; the
//! Sico automation adapter (and nothing else) drives it for the real-path
//! evidence.
//!
//! Determinism contract: fixed 480x320 window, fixed client rects for the
//! two buttons, pure white background, black text, no timers, no
//! environment-dependent drawing. Every frame is byte-identical for the
//! same counter value.
//!
//! Usage: `sico-automation-fixture [TITLE]`; exits 0 on `WM_CLOSE`.

// Platform FFI lives here by architecture rule (ADR-0013 §5: the smallest
// isolated crate holds the unsafe); the workspace-wide forbid relaxes for
// this crate alone.
#![allow(unsafe_code)]

use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    Graphics::Gdi::{
        BeginPaint, CreateSolidBrush, DeleteObject, EndPaint, FillRect, InvalidateRect,
        PAINTSTRUCT, SetBkMode, SetTextColor, TRANSPARENT, TextOutW,
    },
    System::LibraryLoader::GetModuleHandleW,
    UI::WindowsAndMessaging::{
        CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, CreateWindowExW, DefWindowProcW, DispatchMessageW,
        GetClientRect, GetMessageW, GetWindowLongPtrW, HMENU, MSG, PostQuitMessage, RegisterClassW,
        SW_SHOW, SetWindowLongPtrW, ShowWindow, TranslateMessage, WM_COMMAND, WM_DESTROY, WM_PAINT,
        WNDCLASSW, WS_CHILD, WS_OVERLAPPEDWINDOW, WS_VISIBLE,
    },
};

/// Wide (UTF-16, NUL-terminated) from a byte string literal.
fn wide(bytes: &[u8]) -> Vec<u16> {
    let mut out = Vec::with_capacity(bytes.len() + 1);
    let mut i = 0;
    while i < bytes.len() {
        out.push(u16::from(bytes[i]));
        i += 1;
    }
    out.push(0);
    out
}

fn wide_from(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

const CLASS_NAME: &[u8] = b"SicoFixture";
const ID_INCREMENT: isize = 1001;
const ID_RESET: isize = 1002;
const GWLP_USERDATA: i32 = -21;

struct State {
    counter: std::cell::Cell<i64>,
}

/// Reads the shared state pointer out of the window's user slot.
unsafe fn state_of(hwnd: HWND) -> Option<&'static State> {
    // Edition-2024 `unsafe_op_in_unsafe_fn`: the unsafe fn body still needs
    // an explicit unsafe block for the FFI read.
    unsafe {
        let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const State;
        ptr.as_ref()
    }
}

fn refresh(hwnd: HWND) {
    // InvalidateRect alone queues a full erase+repaint; the message loop
    // dispatches WM_PAINT deterministically afterwards.
    unsafe {
        InvalidateRect(hwnd, std::ptr::null(), 1);
    }
}

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        match msg {
            WM_COMMAND => {
                #[allow(clippy::cast_possible_wrap)] // WPARAM: child ids are 1001..=1002
                let id = wparam as isize;
                if let Some(state) = state_of(hwnd) {
                    match id {
                        ID_INCREMENT => {
                            state.counter.set(state.counter.get() + 1);
                            refresh(hwnd);
                            0
                        }
                        ID_RESET => {
                            state.counter.set(0);
                            refresh(hwnd);
                            0
                        }
                        _ => 1,
                    }
                } else {
                    1
                }
            }
            WM_PAINT => {
                let mut paint: PAINTSTRUCT = std::mem::zeroed();
                let hdc = BeginPaint(hwnd, &raw mut paint);
                let put_text = |y: i32, text: &str| {
                    let wide = wide_from(text);
                    let len = i32::try_from(wide.len()).unwrap_or(1);
                    TextOutW(hdc, 20, y, wide.as_ptr(), len - 1);
                };
                if let Some(state) = state_of(hwnd) {
                    let mut rect = std::mem::zeroed();
                    GetClientRect(hwnd, &raw mut rect);
                    let background = CreateSolidBrush(0x00FF_FFFF);
                    FillRect(hdc, &raw const rect, background);
                    DeleteObject(background);
                    SetBkMode(hdc, i32::from(u16::try_from(TRANSPARENT).unwrap_or(0)));
                    SetTextColor(hdc, 0x0000_0000);
                    put_text(20, &format!("count={}", state.counter.get()));
                    put_text(52, "buttons: increment | reset");
                }
                EndPaint(hwnd, &raw const paint);
                0
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

fn main() {
    let title = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "SicoFixture".to_owned());
    unsafe {
        let instance = GetModuleHandleW(std::ptr::null());
        let class_name = wide(CLASS_NAME);
        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            hInstance: instance,
            lpszClassName: class_name.as_ptr(),
            ..std::mem::zeroed()
        };
        RegisterClassW(&raw const wc);

        let state: &'static State = Box::leak(Box::new(State {
            counter: std::cell::Cell::new(0),
        }));

        let title_w = wide_from(&title);
        // windows-sys 0.52 handles are `isize`; zero is the null handle.
        let hwnd: HWND = CreateWindowExW(
            0,
            class_name.as_ptr(),
            title_w.as_ptr(),
            WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            480,
            320,
            0,
            0,
            instance,
            std::ptr::null(),
        );
        assert!(hwnd != 0, "CreateWindowExW failed");
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, std::ptr::from_ref(state) as isize);

        let button_class = wide(b"BUTTON");
        let inc_label = wide(b"increment");
        let reset_label = wide(b"reset");
        // Child ids ride the HMENU parameter for WM_COMMAND routing.
        let inc = CreateWindowExW(
            0,
            button_class.as_ptr(),
            inc_label.as_ptr(),
            WS_CHILD | WS_VISIBLE,
            20,
            96,
            140,
            40,
            hwnd,
            ID_INCREMENT as HMENU,
            instance,
            std::ptr::null(),
        );
        let reset = CreateWindowExW(
            0,
            button_class.as_ptr(),
            reset_label.as_ptr(),
            WS_CHILD | WS_VISIBLE,
            20,
            156,
            140,
            40,
            hwnd,
            ID_RESET as HMENU,
            instance,
            std::ptr::null(),
        );
        assert!(inc != 0 && reset != 0, "button creation failed");

        ShowWindow(hwnd, SW_SHOW);

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&raw mut msg, 0, 0, 0) > 0 {
            TranslateMessage(&raw const msg);
            DispatchMessageW(&raw const msg);
        }
    }
}
