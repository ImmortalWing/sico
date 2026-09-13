//! Windows adapter (ADR-0013 as amended by A1): Win32 enumeration, GDI
//! `PrintWindow` capture, `SendInput` commit. This module is the
//! architecture-sanctioned FFI home; the capability decisions stay in the
//! shared core ([`crate`]), which this adapter never bypasses — the real
//! loop composes core + adapter, so dry-run, tokens and audit apply to
//! every real action exactly as they do in the synthetic corpus.

use windows_sys::Win32::{
    Foundation::HWND,
    Graphics::Gdi::{
        BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateDIBSection, DIB_RGB_COLORS, GetDC, ReleaseDC,
    },
    Storage::Xps::{PW_CLIENTONLY, PrintWindow},
    UI::WindowsAndMessaging::{
        FindWindowW, GetClientRect, GetSystemMetrics, GetWindowRect, IsWindow,
        PW_RENDERFULLCONTENT, SetForegroundWindow,
    },
};

/// Raw fixture surface: the Win32 window handle (class-bound).
#[derive(Clone, Copy, Debug)]
pub struct FixtureWindow {
    pub hwnd: HWND,
}

/// Locates the fixture window by its registered class. The class name is
/// the binding: a user window with a matching title but another class is
/// invisible to this adapter by construction.
#[must_use]
pub fn find_fixture() -> Option<FixtureWindow> {
    let class: Vec<u16> = "SicoFixture\0".encode_utf16().collect();
    let hwnd = unsafe { FindWindowW(class.as_ptr(), std::ptr::null()) };
    (hwnd != 0).then_some(FixtureWindow { hwnd })
}

impl FixtureWindow {
    /// True while the underlying window still exists (identity validity).
    #[must_use]
    pub fn alive(&self) -> bool {
        unsafe { IsWindow(self.hwnd) != 0 }
    }

    /// Client-rect geometry (the capture and coordinate space).
    #[must_use]
    pub fn client_size(&self) -> Option<(u32, u32)> {
        let mut rect: windows_sys::Win32::Foundation::RECT = unsafe { std::mem::zeroed() };
        if unsafe { GetClientRect(self.hwnd, &raw mut rect) } != 0 {
            Some((
                u32::try_from((rect.right - rect.left).max(0)).unwrap_or(0),
                u32::try_from((rect.bottom - rect.top).max(0)).unwrap_or(0),
            ))
        } else {
            None
        }
    }

    /// Window rect (geometry drift detection for stale-revision checks).
    #[must_use]
    pub fn window_rect(&self) -> Option<(i32, i32, i32, i32)> {
        let mut rect: windows_sys::Win32::Foundation::RECT = unsafe { std::mem::zeroed() };
        if unsafe { GetWindowRect(self.hwnd, &raw mut rect) } != 0 {
            Some((rect.left, rect.top, rect.right, rect.bottom))
        } else {
            None
        }
    }

    /// One BGRA8 top-down frame of the client area via `PrintWindow`
    /// (`PW_RENDERFULLCONTENT`; ADR-0013 amendment A1's v0 capture path).
    /// Returns (width, height, stride, pixels).
    ///
    /// # Errors
    ///
    /// Typed `String` failures for DC setup, `PrintWindow` rejection, or
    /// an empty client area — never a partial frame.
    pub fn capture(&self) -> Result<(u32, u32, u32, Vec<u8>), String> {
        let (width, height) = self.client_size().ok_or("client rect failed")?;
        if width == 0 || height == 0 {
            return Err("empty client area".into());
        }
        let stride = width * 4;
        let mut bmi: BITMAPINFO = unsafe { std::mem::zeroed() };
        #[allow(clippy::cast_possible_truncation)] // fixed struct size < u32::MAX
        let bi_size = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biSize = bi_size;
        #[allow(clippy::cast_possible_wrap)] // BITMAPINFO biWidth is i32 by contract
        let bi_width = width as i32;
        bmi.bmiHeader.biWidth = bi_width;
        // Negative height = top-down rows (row 0 first), matching the
        // RFC-0041 orientation contract.
        bmi.bmiHeader.biHeight = -(height.cast_signed());
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB;

        unsafe {
            let screen_dc = GetDC(0);
            if screen_dc == 0 {
                return Err("GetDC failed".into());
            }
            // Both the DIB section and the compatible DC derive from the
            // screen DC; release the screen DC only after both exist.
            let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
            let section = CreateDIBSection(
                screen_dc,
                &raw const bmi,
                DIB_RGB_COLORS,
                &raw mut bits,
                0,
                0,
            );
            let memory_dc = CreateCompatibleDC(screen_dc);
            ReleaseDC(0, screen_dc);
            if section == 0 || bits.is_null() || memory_dc == 0 {
                if section != 0 {
                    DeleteObject(section);
                }
                if memory_dc != 0 {
                    DeleteDC(memory_dc);
                }
                return Err("capture DC setup failed".into());
            }
            let old = SelectObject(memory_dc, section);
            let painted = PrintWindow(self.hwnd, memory_dc, PW_RENDERFULLCONTENT | PW_CLIENTONLY);
            let outcome = if painted != 0 {
                let len = stride as usize * height as usize;
                let mut out = vec![0_u8; len];
                std::ptr::copy_nonoverlapping(bits as *const u8, out.as_mut_ptr(), len);
                Ok((width, height, stride, out))
            } else {
                Err("PrintWindow failed".into())
            };
            SelectObject(memory_dc, old);
            DeleteObject(section);
            DeleteDC(memory_dc);
            outcome
        }
    }

    /// Commits one pointer click at client coordinates: the surface is
    /// promoted to the foreground, the client point converts to screen
    /// space at commit time, and one atomic `SendInput` sequence (absolute
    /// move + press + release) delivers the action (ADR-0013 §3: the
    /// committed action is bounded and never split).
    ///
    /// # Errors
    ///
    /// Typed `String` failures for coordinate conversion, foreground
    /// promotion, cursor positioning, or a short `SendInput` delivery.
    pub fn commit_click(&self, x: u32, y: u32) -> Result<(), String> {
        use windows_sys::Win32::{
            Foundation::POINT,
            Graphics::Gdi::ClientToScreen,
            UI::Input::KeyboardAndMouse::{
                INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_LEFTDOWN,
                MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MOVE, MOUSEINPUT, SendInput, keybd_event,
            },
            UI::WindowsAndMessaging::SetCursorPos,
        };
        const VK_MENU: i32 = 0x12;
        const KEYEVENTF_KEYUP: u32 = 0x0002;
        #[allow(clippy::cast_possible_wrap)] // client coords are inside i32 view bounds
        let mut point = POINT {
            x: x as i32,
            y: y as i32,
        };
        if unsafe { ClientToScreen(self.hwnd, &raw mut point) } == 0 {
            return Err("ClientToScreen failed".into());
        }
        unsafe {
            // Windows foreground lock: a background process may not call
            // SetForegroundWindow unless it "received the last input
            // event". A synthesized ALT tap (the canonical unlock) grants
            // exactly that; it is delivered to the current foreground
            // window as a no-op modifier press and released immediately.
            #[allow(clippy::cast_possible_truncation)] // VK_MENU = 0x12 fits u8
            let vk_menu = VK_MENU as u8;
            keybd_event(vk_menu, 0, 0, 0);
            keybd_event(vk_menu, 0, KEYEVENTF_KEYUP, 0);
            if SetForegroundWindow(self.hwnd) == 0 {
                return Err("SetForegroundWindow failed (foreground lock)".into());
            }
        }
        let (screen_w, screen_h) = unsafe { (GetSystemMetrics(0), GetSystemMetrics(1)) };
        if screen_w <= 1 || screen_h <= 1 {
            return Err("screen metrics failed".into());
        }
        #[allow(clippy::cast_possible_truncation)] // absolute coords fit i32 (0..=65535)
        let to_absolute =
            |value: i32, span: i32| ((i64::from(value) * 65_535) / i64::from(span - 1)) as i32;
        let (ax, ay) = (
            to_absolute(point.x, screen_w),
            to_absolute(point.y, screen_h),
        );
        let make = |dx: i32, dy: i32, dw_flags: u32| INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx,
                    dy,
                    mouseData: 0,
                    dwFlags: dw_flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
        // INPUT size < i32::MAX
        let input_stride = std::mem::size_of::<INPUT>() as i32;
        let mut inputs = [
            make(ax, ay, MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE),
            make(0, 0, MOUSEEVENTF_LEFTDOWN),
            make(0, 0, MOUSEEVENTF_LEFTUP),
        ];
        unsafe {
            if SetCursorPos(point.x, point.y) == 0 {
                return Err("SetCursorPos failed".into());
            }
            let sent = SendInput(3, inputs.as_mut_ptr(), input_stride);
            if sent != 3 {
                return Err("SendInput did not deliver the full action".into());
            }
        }
        Ok(())
    }
}

use windows_sys::Win32::Graphics::Gdi::{CreateCompatibleDC, DeleteDC, DeleteObject, SelectObject};

/// (handle count, working-set bytes) of the current process — budget
/// evidence for the real-loop driver (M16 gate 5).
///
/// # Panics
///
/// Panics if the process-budget queries fail, which would mean the
/// evidence numbers are untrustworthy anyway.
#[must_use]
pub fn process_budget() -> (u32, usize) {
    use windows_sys::Win32::{
        Foundation::HANDLE,
        System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS},
        System::Threading::{GetCurrentProcess, GetProcessHandleCount},
    };
    unsafe {
        let process: HANDLE = GetCurrentProcess();
        let mut handles: u32 = 0;
        assert!(GetProcessHandleCount(process, &raw mut handles) != 0);
        #[allow(clippy::cast_possible_truncation)] // fixed struct size < u32::MAX
        let counters_size = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
        let mut counters: PROCESS_MEMORY_COUNTERS = std::mem::zeroed();
        counters.cb = counters_size;
        assert!(GetProcessMemoryInfo(process, &raw mut counters, counters.cb) != 0);
        (handles, counters.WorkingSetSize)
    }
}
