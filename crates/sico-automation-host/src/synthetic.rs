//! The synthetic surface backend (M16 plan §5.4): every RFC-level
//! guarantee testable without OS access. Evidence class is
//! `contract-verified` and is never advertised as runtime support.
//!
//! The simulated surface owns a deterministic pixel buffer and scripted
//! behaviors: geometry drift, dialog injection, content repaints. The
//! Host core never reads it directly — the test driver plays both sides,
//! exactly like the Windows adapter will.

/// One simulated surface's observable behavior.
#[derive(Clone, Debug)]
pub struct SimulatedSurface {
    pub title: String,
    pub width: u32,
    pub height: u32,
    /// Content marker: bumps on every scripted repaint so drift is
    /// observable through capture comparisons.
    pub content_marker: u64,
    /// Dialogs injected after N committed actions (scripted behavior).
    pub dialog_after_actions: Option<u64>,
    pub dialog_visible: bool,
}

impl SimulatedSurface {
    #[must_use]
    pub fn new(title: &str, width: u32, height: u32) -> Self {
        Self {
            title: title.to_owned(),
            width,
            height,
            content_marker: 1,
            dialog_after_actions: None,
            dialog_visible: false,
        }
    }

    /// Scripted: a dialog pops after N committed actions.
    #[must_use]
    pub fn with_dialog_after(mut self, actions: u64) -> Self {
        self.dialog_after_actions = Some(actions);
        self
    }

    /// Scripted repaint: the driver bumps the Host revision alongside.
    pub fn repaint(&mut self) {
        self.content_marker = self.content_marker.wrapping_add(1);
    }

    /// Deterministic bgra8 pixel buffer for the current state: a fixed
    /// pattern derived from the marker, so frame equality in the corpus
    /// is byte-exact and drift is detectable.
    #[must_use]
    pub fn frame(&self) -> Vec<u8> {
        let stride = self.width * 4;
        let mut pixels = vec![0_u8; stride as usize * self.height as usize];
        let marker = self.content_marker.to_le_bytes();
        for (index, byte) in pixels.iter_mut().enumerate() {
            let mix = marker[(index / 64) % 4];
            // Truncation to u8 is the deterministic pattern, intended.
            #[allow(clippy::cast_possible_truncation)]
            let mixed = (index % 256) as u8;
            *byte = mixed.wrapping_mul(31).wrapping_add(mix);
            *byte = byte.wrapping_mul(31).wrapping_add(mix);
        }
        if self.dialog_visible {
            // A dialog occupies the top band deterministically.
            for pixel in pixels.iter_mut().take(stride as usize * 32) {
                *pixel = 0xFF;
            }
        }
        pixels
    }
}
