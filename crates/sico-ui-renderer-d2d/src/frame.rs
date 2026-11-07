//! A rendered frame.

use std::fmt::Write as _;

use sha2::{Digest, Sha256};

/// One rendered frame as BGRA8 pixels.
///
/// The frame is the renderer's observable output. Its SHA-256 digest is the
/// frame-corpus identity: because the renderer is deterministic (a fixed
/// token table, a fixed client size, no timers and no environment-derived
/// values), the same tree always produces the same digest on the evidence
/// host.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Frame {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl Frame {
    /// Builds a frame from raw BGRA8 pixels.
    ///
    /// # Errors
    ///
    /// Returns `Err` when the payload length does not match
    /// `width * height * 4`.
    pub fn new(width: u32, height: u32, pixels: Vec<u8>) -> Result<Self, String> {
        let expected = (width as usize)
            .checked_mul(height as usize)
            .and_then(|area| area.checked_mul(4))
            .ok_or_else(|| "frame dimensions overflow".to_owned())?;
        if expected != pixels.len() {
            return Err(format!(
                "frame payload length {} does not match {}x{} BGRA8",
                pixels.len(),
                width,
                height
            ));
        }
        Ok(Self {
            width,
            height,
            pixels,
        })
    }

    /// The frame width in pixels.
    #[must_use]
    pub fn width(&self) -> u32 {
        self.width
    }

    /// The frame height in pixels.
    #[must_use]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// The raw BGRA8 pixels, row-major, no padding.
    #[must_use]
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// The SHA-256 digest of the pixel payload, lowercase hex.
    #[must_use]
    pub fn sha256(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(&self.pixels);
        let digest = hasher.finalize();
        let mut out = String::with_capacity(digest.len() * 2);
        for byte in digest {
            let _ = write!(out, "{byte:02x}");
        }
        out
    }
}
